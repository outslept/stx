use crate::{
    context::LintContext,
    rules::Rule,
    utils::{ValueTokenKind, decl_value_span, ident_name, tokenize_value},
};
use oxc_css_parser::ast::{AtRule, Statement};

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

#[derive(Debug, Default)]
pub struct SyntaxStringNoInvalid;

impl Rule for SyntaxStringNoInvalid {
    fn name(&self) -> &'static str {
        "syntax-string-no-invalid"
    }
    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("property") {
            return;
        }
        let Some(block) = &at_rule.block else {
            return;
        };
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
            let Some(vspan) = decl_value_span(ctx.source, decl) else {
                continue;
            };
            for token in tokenize_value(ctx.source, vspan) {
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
                    token.span,
                    format!("Invalid syntax string \"{inner}\""),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject_message};
    use serde_json::json;
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
