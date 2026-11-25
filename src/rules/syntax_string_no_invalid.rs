use crate::{
    context::LintContext,
    rules::Rule,
    utils::{ValueTokenKind, decl_value_span, ident_name, tokenize_value},
};
use oxc_css_parser::{
    Allocator, ParserBuilder, Span, Syntax,
    ast::{AtRule, SimpleBlock, Statement, Stylesheet},
};

const TYPES: &[&str] = &[
    "angle",
    "color",
    "custom-ident",
    "image",
    "integer",
    "length",
    "length-percentage",
    "number",
    "percentage",
    "resolution",
    "string",
    "time",
    "url",
    "transform-function",
    "transform-list",
];

fn valid_syntax(value: &str) -> bool {
    let value = value.trim();
    if value == "*" || value.contains('\\') {
        return true;
    }
    if value.is_empty() || value.contains("||") {
        return false;
    }
    value.split('|').all(valid_component)
}

fn valid_component(component: &str) -> bool {
    let component = component.trim();
    if component.is_empty() {
        return false;
    }

    let (name, multiplier) = if let Some(start) = component.strip_prefix('<') {
        let Some(end) = start.find('>') else {
            return false;
        };
        let name = &start[..end];
        let suffix = start[end + 1..].trim();
        let multiplier = if suffix.is_empty() {
            None
        } else if suffix == "#" || suffix == "+" {
            Some(suffix)
        } else {
            return false;
        };
        (name, multiplier)
    } else {
        (component, None)
    };

    if name.is_empty()
        || name
            .chars()
            .any(|ch| matches!(ch, '<' | '>' | '#' | '+' | '*' | '\'' | '"'))
    {
        return false;
    }
    if component.starts_with('<') {
        if !TYPES.iter().any(|ty| ty.eq_ignore_ascii_case(name)) {
            return false;
        }
        multiplier.is_some()
            || component.ends_with('>')
            || component.ends_with("#")
            || component.ends_with('+')
    } else {
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        (first.is_ascii_alphabetic() || matches!(first, '-' | '_') || !first.is_ascii())
            && chars
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') || !ch.is_ascii())
    }
}

crate::define_stx_lint! {
    /// Reports invalid syntax strings in `@property` registrations.
    ///
    /// Set to `true`. Only checks quoted `syntax` descriptor values, not whether the
    /// registration is otherwise complete. Accepts `*`, known type names and literal
    /// identifiers, with `|` alternatives and supported `+` and `#` multipliers.
    /// Does not fix violations.
    ///
    /// Example: `@property --foo { syntax: "<length> | <foo>" }` is reported for `<foo>`.
    "syntax-string-no-invalid", correctness;
    #[derive(Debug, Default)]
    pub struct SyntaxStringNoInvalid;
}

impl SyntaxStringNoInvalid {
    fn check_block(&self, block: &SimpleBlock<'_>, offset: u32, ctx: &mut LintContext<'_>) {
        let source = &ctx.source[offset as usize..];
        for st in &block.statements {
            let Statement::Declaration(decl) = st else {
                continue;
            };
            let Some(prop) = ident_name(&decl.name) else {
                continue;
            };
            if !prop.eq_ignore_ascii_case("syntax") {
                continue;
            }
            let Some(vspan) = decl_value_span(source, decl) else {
                continue;
            };
            for token in tokenize_value(source, vspan) {
                if token.kind != ValueTokenKind::String {
                    continue;
                }
                let raw = token.raw;
                if raw.len() < 2 {
                    continue;
                }
                let inner = &raw[1..raw.len() - 1];
                if valid_syntax(inner) {
                    continue;
                }
                ctx.report(
                    self.name(),
                    Span {
                        start: offset + token.span.start,
                        end: offset + token.span.end,
                    },
                    format!("Invalid syntax string \"{inner}\""),
                );
            }
        }
    }
}

impl Rule for SyntaxStringNoInvalid {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if at_rule.name.name.eq_ignore_ascii_case("property")
            && let Some(block) = &at_rule.block
        {
            self.check_block(block, 0, ctx);
        }
    }

    fn check_stylesheet<'a>(&self, ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let source = ctx.source;
        if !ast.statements.is_empty() || ctx.syntax != Syntax::Css || !source.contains('@') {
            return;
        }
        let tokens = tokenize_value(
            source,
            Span {
                start: 0,
                end: source.len() as u32,
            },
        );
        let mut i = 0;
        while let Some(pair) = tokens.get(i..i + 2) {
            if pair[0].raw.eq_ignore_ascii_case("@property")
                && pair[1].raw == "{"
                && (i == 0 || matches!(tokens[i - 1].raw, "{" | "}" | ";"))
            {
                let start = pair[1].span.start;
                let allocator = Allocator::default();
                let mut parser = ParserBuilder::new(&allocator, &source[start as usize..]).build();
                let Ok(block) = parser.parse::<SimpleBlock>() else {
                    break;
                };
                self.check_block(&block, start, ctx);
                i += tokens[i..].partition_point(|token| token.span.start < start + block.span.end);
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        lint_once,
        rules::build_rules,
        tester::{Tester, reject_message},
    };
    use serde_json::json;

    #[test]
    fn checks_the_block_when_the_property_name_is_missing() {
        let config =
            Config::from_json_str(r#"{"rules":{"syntax-string-no-invalid":true}}"#).unwrap();
        let code = "/* @property { syntax: '<foo>'; } */\na { content: '@property { syntax: x }'; }\n@PROPERTY /* name missing */ { syntax: '<length> | <foo>'; }\n@property { syntax: '<color>'; }";
        let pass = lint_once(code, Syntax::Css, &build_rules(&config), false);
        assert!(pass.parse_error.is_some());
        let [warning] = pass.warnings.as_slice() else {
            panic!("Expected one warning, got {:?}", pass.warnings);
        };
        assert_eq!(
            warning.text,
            "Invalid syntax string \"<length> | <foo>\" (syntax-string-no-invalid)"
        );
        assert_eq!(
            (
                warning.line,
                warning.column,
                warning.end_line,
                warning.end_column
            ),
            (3, 40, 3, 58)
        );
    }

    #[test]
    fn default_config() {
        let pass = vec![
            "@property --foo {}",
            "@foo { syntax: \"<bar>\"; }",
            "@property --foo { syntax: \"<color>\"; }",
            "@property --foo { syntax: \"<length> | <percentage>\"; }",
            "@property --foo { syntax: \"<color>+\"; }",
            "@property --foo { syntax: \"<length>#\"; }",
            "@property --foo { syntax: \"small | medium | large\"; }",
            "@property --foo { syntax: \"<length> | auto\"; }",
            "@property --foo { syntax: \"<transform-list>\"; }",
            "@property --foo { syntax: \"*\"; }",
        ];
        let fail = vec![
            reject_message(
                "@property --foo { syntax: \"<alpha-value>\" }",
                None,
                "Invalid syntax string \"<alpha-value>\"",
            ),
            reject_message(
                "@property --foo { syntax: \"<foo>\" }",
                None,
                "Invalid syntax string \"<foo>\"",
            ),
            reject_message(
                "@property --foo { syntax: \"<length> | <foo>\" }",
                None,
                "Invalid syntax string \"<length> | <foo>\"",
            ),
            reject_message(
                "@property --foo { syntax: \"<length>*\" }",
                None,
                "Invalid syntax string \"<length>*\"",
            ),
            reject_message(
                "@property --foo { syntax: \"<length> || <color>\" }",
                None,
                "Invalid syntax string \"<length> || <color>\"",
            ),
            reject_message(
                "@property --foo { syntax: \"small |\" }",
                None,
                "Invalid syntax string \"small |\"",
            ),
        ];
        Tester::new("syntax-string-no-invalid", json!(true), &pass, &fail).test();
    }
}
