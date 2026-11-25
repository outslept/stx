use oxc_css_parser::ast::Declaration;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, ValueTokenKind, decl_value_span, ident_name, tokenize_value},
};

#[derive(Debug, Default)]
pub struct DeclarationPropertyUnitDisallowedList {
    entries: Vec<(OptionMatcher, Vec<String>)>,
}

impl DeclarationPropertyUnitDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Option<Self> {
        let object = primary.as_object()?;
        let mut entries = Vec::with_capacity(object.len());
        for (key, value) in object {
            let mut units = Vec::new();
            match value {
                serde_json::Value::String(s) => units.push(s.to_ascii_lowercase()),
                serde_json::Value::Array(items) => {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            units.push(s.to_ascii_lowercase());
                        }
                    }
                }
                _ => {}
            }
            entries.push((OptionMatcher::parse(key), units));
        }
        Some(Self { entries })
    }
}

pub fn rejected_message(property: &str, unit: &str) -> String {
    format!("Disallowed unit \"{unit}\" for property \"{property}\"")
}

impl Rule for DeclarationPropertyUnitDisallowedList {
    fn name(&self) -> &'static str {
        "declaration-property-unit-disallowed-list"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };

        let mut disallowed = None;
        for (matcher, units) in &self.entries {
            if matcher.is_match(prop) {
                disallowed = Some(units);
                break;
            }
        }
        let Some(disallowed) = disallowed else {
            return;
        };

        let tokens = tokenize_value(ctx.source, value_span);
        let mut depth = 0usize;
        let mut skip_from: Option<usize> = None;

        for token in &tokens {
            match token.kind {
                ValueTokenKind::Function => {
                    depth += 1;
                    if skip_from.is_none() {
                        let name = token.raw.strip_suffix('(').unwrap_or(token.raw);
                        if name.eq_ignore_ascii_case("url") {
                            skip_from = Some(depth);
                        }
                    }
                }
                ValueTokenKind::CloseParen => {
                    depth = depth.saturating_sub(1);
                    if skip_from.is_some_and(|from| depth < from) {
                        skip_from = None;
                    }
                }
                _ if skip_from.is_some() => {}
                ValueTokenKind::Dimension { num_len } => {
                    let unit = &token.raw[num_len..];
                    if disallowed.iter().any(|u| u.eq_ignore_ascii_case(unit)) {
                        ctx.report(self.name(), token.span, rejected_message(prop, unit));
                    }
                }
                ValueTokenKind::Percentage if disallowed.iter().any(|u| u == "%") => {
                    ctx.report(self.name(), token.span, rejected_message(prop, "%"));
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "a { color: pink; }",
            "a { top: 0; }",
            "a { color: #000; }",
            "a { margin: 0 0 0 0 }",
            "a { margin: 0 10px 5rem 2in; }",
            "a { margin: 0 10pX 5rem 2in; }",
            "a { margin: 0 10PX 5rem 2in; }",
            "a { background-position: top right, 1em 5vh; }",
            "a { margin: calc(30vh - 10vh); }",
            "a { animation: animation-name 300ms ease; }",
            "a { -webkit-animation: animation-name 300ms ease; }",
            "a { animation-duration: 3s; }",
            "a { -webkit-animation-duration: 3s; }",
            "a { font-size: /* 100px */ 1.2rem; }",
            "a::before { font-size: \"10px\"}",
            "a { font-size: $fs10px; }",
            "a { font-size: --some-fs-10px; }",
        ];

        let fail = vec![
            reject(
                "a { font-size: 12px; }",
                None,
                rejected_message("font-size", "px"),
                (1, 16, 1, 20),
            ),
            reject(
                "a { font-size: 12pX; }",
                None,
                rejected_message("font-size", "pX"),
                (1, 16, 1, 20),
            ),
            reject(
                "a { font-size: 12PX; }",
                None,
                rejected_message("font-size", "PX"),
                (1, 16, 1, 20),
            ),
            reject(
                "a { margin: 10px 0 5em; }",
                None,
                rejected_message("margin", "em"),
                (1, 20, 1, 23),
            ),
            reject(
                "a { background-position: 0 10%; }",
                None,
                rejected_message("background-position", "%"),
                (1, 28, 1, 31),
            ),
            reject(
                "a { background-position: top right, 0 10%; }",
                None,
                rejected_message("background-position", "%"),
                (1, 39, 1, 42),
            ),
            reject(
                "a { margin: calc(10vh - 10em); }",
                None,
                rejected_message("margin", "em"),
                (1, 25, 1, 29),
            ),
            reject(
                "a { animation: foo 3s; }",
                None,
                rejected_message("animation", "s"),
                (1, 20, 1, 22),
            ),
            reject(
                "a { -webkit-animation: foo 3s; }",
                None,
                rejected_message("-webkit-animation", "s"),
                (1, 28, 1, 30),
            ),
        ];

        Tester::new(
            "declaration-property-unit-disallowed-list",
            json!([{
                "font-size": ["px", "em"],
                "margin": "em",
                "background-position": ["%"],
                "/^(-webkit-)?animation$/": ["s"],
            }]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn animation_units() {
        let pass = vec![
            "a { animation: animation-name 300ms ease; }",
            "a { -webkit-animation: animation-name 300ms ease; }",
            "a { animation-duration: 300ms; }",
            "a { -webkit-animation-duration: 300ms; }",
        ];

        let fail = vec![
            reject_message(
                "a { animation: animation-name 3s ease; }",
                None,
                rejected_message("animation", "s"),
            ),
            reject_message(
                "a { -webkit-animation: animation-name 3s ease; }",
                None,
                rejected_message("-webkit-animation", "s"),
            ),
            reject_message(
                "a { animation-duration: 3s; }",
                None,
                rejected_message("animation-duration", "s"),
            ),
            reject_message(
                "a { -webkit-animation-duration: 3s; }",
                None,
                rejected_message("-webkit-animation-duration", "s"),
            ),
        ];

        Tester::new(
            "declaration-property-unit-disallowed-list",
            json!([{
                "/^(-webkit-)?animation/": ["s"],
            }]),
            &pass,
            &fail,
        )
        .test();
    }
}
