use oxc_css_parser::ast::Declaration;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, decl_value_span, ident_name},
};

crate::define_stx_lint! {
    /// Disallows specified declaration values for configured properties.
    ///
    /// The primary option maps property names or `/regex/` patterns to a
    /// value matcher or an array of matchers. Use literal strings or
    /// `/regex/` patterns. A literal matches the whole value, not individual
    /// tokens. The rule reports a value that matches any applicable entry.
    ///
    /// With `{"color": "red"}`:
    ///
    /// ```css
    /// a { color: pink; } /* passes */
    /// a { color: red; } /* reported */
    /// ```
    "declaration-property-value-disallowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct DeclarationPropertyValueDisallowedList {
        entries: Vec<(OptionMatcher, Vec<OptionMatcher>)>,
    }
}

impl DeclarationPropertyValueDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Option<Self> {
        let object = primary.as_object()?;
        let mut entries = Vec::with_capacity(object.len());
        for (key, value) in object {
            let mut disallowed = Vec::new();
            match value {
                serde_json::Value::String(s) => disallowed.push(OptionMatcher::parse(s)),
                serde_json::Value::Array(items) => {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            disallowed.push(OptionMatcher::parse(s));
                        }
                    }
                }
                _ => {}
            }
            entries.push((OptionMatcher::parse(key), disallowed));
        }
        Some(Self { entries })
    }
}

pub fn rejected_message(property: &str, value: &str) -> String {
    format!("Disallowed value \"{value}\" for property \"{property}\"")
}

impl Rule for DeclarationPropertyValueDisallowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let value = ctx.slice(value_span);

        let mut matched = false;
        let mut disallowed = false;
        for (key, values) in &self.entries {
            if key.is_match(prop) {
                matched = true;
                if values.iter().any(|matcher| matcher.is_match(value)) {
                    disallowed = true;
                    break;
                }
            }
        }
        if !matched || !disallowed {
            return;
        }

        ctx.report(self.name(), value_span, rejected_message(prop, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_message, reject_messages};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "a { color: pink; }",
            "a { color: lightgreen; }",
            "a { text-transform: lowercase; }",
            "a { transform: matrix(1.0, 2.0, 3.0, 4.0, 5.0, 6.0) translate(12px, 50%); }",
            "a { -webkit-transform: matrix(1.0, 2.0, 3.0, 4.0, 5.0, 6.0) translate(12px, 50%); }",
            "a { color: /* red */ pink; }",
            "a::before { color: \"red\"}",
            "a { color: $red; }",
            "a { color: --some-red; }",
            "a { color: darkseagreen }",
        ];

        let fail = vec![
            reject(
                "a { color: red; }",
                None,
                rejected_message("color", "red"),
                (1, 12, 1, 15),
            ),
            reject(
                "a { color: green }",
                None,
                rejected_message("color", "green"),
                (1, 12, 1, 17),
            ),
            reject(
                "a { text-transform: uppercase; }",
                None,
                rejected_message("text-transform", "uppercase"),
                (1, 21, 1, 30),
            ),
            reject(
                "a { transform: scale3d(1, 2, 3) }",
                None,
                rejected_message("transform", "scale3d(1, 2, 3)"),
                (1, 16, 1, 32),
            ),
            reject(
                "a { -webkit-transform: scale3d(1, 2, 3) }",
                None,
                rejected_message("-webkit-transform", "scale3d(1, 2, 3)"),
                (1, 24, 1, 40),
            ),
            reject(
                "a { color: seagreen }",
                None,
                rejected_message("color", "seagreen"),
                (1, 12, 1, 20),
            ),
        ];

        Tester::new(
            "declaration-property-value-disallowed-list",
            json!([{
                "text-transform": "uppercase",
                "/^(-webkit-)?transform$/": ["/scale3d/", "/rotate3d/", "/translate3d/"],
                "color": ["red", "green", "blue", "/^sea/"],
            }]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn animation_units() {
        let pass = vec![
            "a { animation: foo 1s linear; }",
            "a { -webkit-animation: foo 1s linear; }",
            "a { animation-timing-function: linear; }",
            "a { -webkit-animation-timing-function: linear; }",
        ];

        let fail = vec![
            reject_message(
                "a { animation: foo 1s ease-in-out; }",
                None,
                rejected_message("animation", "foo 1s ease-in-out"),
            ),
            reject_message(
                "a { -webkit-animation: foo 1s ease-in-out; }",
                None,
                rejected_message("-webkit-animation", "foo 1s ease-in-out"),
            ),
            reject_message(
                "a { animation-timing-function: ease-in-out; }",
                None,
                rejected_message("animation-timing-function", "ease-in-out"),
            ),
            reject_message(
                "a { -webkit-animation-timing-function: ease-in-out; }",
                None,
                rejected_message("-webkit-animation-timing-function", "ease-in-out"),
            ),
        ];

        Tester::new(
            "declaration-property-value-disallowed-list",
            json!([{
                "/^(-webkit-)?animation/": ["/ease/"],
            }]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn irrelevant_css() {
        Tester::new(
            "declaration-property-value-disallowed-list",
            json!({"position": ["fixed"]}),
            ["a { font-size: 1em; }"],
            [],
        )
        .test();
    }

    #[test]
    fn wildcard_property() {
        let fail = vec![reject_messages(
            "a { color: rebeccapurple; background-color: none; }",
            None,
            vec![
                rejected_message("color", "rebeccapurple"),
                rejected_message("background-color", "none"),
            ],
        )];

        Tester::new(
            "declaration-property-value-disallowed-list",
            json!({"/.*/": ["rebeccapurple"], "background-color": ["none"]}),
            [],
            &fail,
        )
        .test();
    }
}
