use oxc_css_parser::{Span, ast::Declaration};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, decl_value_span, ident_name, is_standard_syntax_value,
        tokenize_value,
    },
};

#[derive(Debug, Default)]
pub struct DeclarationPropertyMaxValues {
    entries: Vec<(OptionMatcher, u64)>,
}

impl DeclarationPropertyMaxValues {
    pub fn from_config(primary: &serde_json::Value) -> Option<Self> {
        let object = primary.as_object()?;
        let mut entries = Vec::with_capacity(object.len());
        for (key, value) in object {
            if let Some(max) = value.as_u64() {
                entries.push((OptionMatcher::parse(key), max));
            }
        }
        Some(Self { entries })
    }
}

pub fn expected_message(property: &str, max: u64) -> String {
    format!("Too many values for property \"{property}\", maximum {max}")
}

impl Rule for DeclarationPropertyMaxValues {
    fn name(&self) -> &'static str {
        "declaration-property-max-values"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        if !is_standard_syntax_value(ctx.slice(value_span)) {
            return;
        }

        let mut max = None;
        for (matcher, value) in &self.entries {
            if matcher.is_match(prop) {
                max = Some(*value);
                break;
            }
        }
        let Some(max) = max else {
            return;
        };

        if count_value_nodes(ctx.source, value_span) as u64 <= max {
            return;
        }

        let end = if ctx.source.as_bytes().get(decl.span.end as usize) == Some(&b';') {
            decl.span.end + 1
        } else {
            decl.span.end
        };
        ctx.report(
            self.name(),
            Span {
                start: decl.span.start,
                end,
            },
            expected_message(prop, max),
        );
    }
}

fn count_value_nodes(source: &str, span: Span) -> usize {
    let tokens = tokenize_value(source, span);
    let mut depth = 0usize;
    let mut count = 0usize;
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].kind {
            ValueTokenKind::Function => {
                if depth == 0 {
                    count += 1;
                }
                depth += 1;
            }
            ValueTokenKind::CloseParen => {
                depth = depth.saturating_sub(1);
            }
            ValueTokenKind::Comma | ValueTokenKind::Slash => {}
            _ => {
                if depth == 0 {
                    count += 1;
                    if tokens[i].raw == "#" {
                        while i + 1 < tokens.len()
                            && tokens[i + 1].span.start == tokens[i].span.end
                            && !matches!(
                                tokens[i + 1].kind,
                                ValueTokenKind::Comma
                                    | ValueTokenKind::Slash
                                    | ValueTokenKind::Function
                                    | ValueTokenKind::CloseParen
                            )
                        {
                            i += 1;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    fn expected(property: &str, max: u64) -> String {
        format!("Too many values for property \"{property}\", maximum {max}")
    }

    #[test]
    fn default_config() {
        let pass = [
            "a { margin: 0; }",
            "a { margin: 1px; }",
            "a { margin: var(--foo); }",
            "a { margin: 1px /* 3px */; }",
            "a { margin-inline: 1px; }",
            "a { margin: ; }",
            "a { border: 1px; }",
            "a { border: 1px solid; }",
            "a { -webkit-transform: scale(1); }",
            "a { transition: margin-right 2s ease-in-out; }",
            "a { margin: $variable 1em; }",
        ];
        let fail = [
            reject(
                "a { margin: 1px 2px; }",
                None,
                expected("margin", 1),
                (1, 5, 1, 21),
            ),
            reject(
                "a { margin-inline: 1px 2px; }",
                None,
                expected("margin-inline", 1),
                (1, 5, 1, 28),
            ),
            reject(
                "a { margin: var(--foo) var(--bar); }",
                None,
                expected("margin", 1),
                (1, 5, 1, 35),
            ),
            reject(
                "a { margin: 1px 2px 3px 4px; }",
                None,
                expected("margin", 1),
                (1, 5, 1, 29),
            ),
            reject(
                "a { margin: 0 0 0 0; }",
                None,
                expected("margin", 1),
                (1, 5, 1, 21),
            ),
            reject(
                "a { border: 1px solid blue; }",
                None,
                expected("border", 2),
                (1, 5, 1, 28),
            ),
            reject(
                "a { -webkit-transform: scale(1) rotate(30deg); }",
                None,
                expected("-webkit-transform", 1),
                (1, 5, 1, 47),
            ),
        ];

        Tester::new(
            "declaration-property-max-values",
            json!([{
                "border": 2,
                "/^margin/": 1,
                "/(-webkit-)?transform$/": 1,
            }]),
            pass,
            fail,
        )
        .test();
    }
}
