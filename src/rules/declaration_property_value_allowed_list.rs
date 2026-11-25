use oxc_css_parser::ast::Declaration;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, decl_value_span, ident_name},
};

crate::define_stx_lint! {
    /// Restricts declaration values to an allow list for each configured property.
    ///
    /// The primary option maps property names or `/regex/` patterns to a
    /// value matcher or an array of matchers. Use literal strings or
    /// `/regex/` patterns. A literal matches the whole value, not individual
    /// tokens. A match under any applicable entry is enough; properties
    /// with no matching entry are unrestricted.
    ///
    /// With `{"color": "green"}`:
    ///
    /// ```css
    /// a { color: green; } /* passes */
    /// a { color: pink; } /* reported */
    /// ```
    "declaration-property-value-allowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct DeclarationPropertyValueAllowedList {
        entries: Vec<(OptionMatcher, Vec<OptionMatcher>)>,
    }
}

impl DeclarationPropertyValueAllowedList {
    pub fn from_config(primary: &serde_json::Value) -> Option<Self> {
        let object = primary.as_object()?;
        let mut entries = Vec::with_capacity(object.len());
        for (key, value) in object {
            let mut allowed = Vec::new();
            match value {
                serde_json::Value::String(s) => allowed.push(OptionMatcher::parse(s)),
                serde_json::Value::Array(items) => {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            allowed.push(OptionMatcher::parse(s));
                        }
                    }
                }
                _ => {}
            }
            entries.push((OptionMatcher::parse(key), allowed));
        }
        Some(Self { entries })
    }
}

pub fn rejected_message(property: &str, value: &str) -> String {
    format!("Disallowed value \"{value}\" for property \"{property}\"")
}

impl Rule for DeclarationPropertyValueAllowedList {
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
        let mut allowed = false;
        for (key, values) in &self.entries {
            if key.is_match(prop) {
                matched = true;
                if values.iter().any(|matcher| matcher.is_match(value)) {
                    allowed = true;
                    break;
                }
            }
        }
        if !matched || allowed {
            return;
        }

        ctx.report(self.name(), value_span, rejected_message(prop, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "div { whitespace: nowrap; }",
            "a { transform: scale(1, 1); }",
            "a { -webkit-transform: scale(1, 1); }",
            "a { color: green; }",
            "a { background-color: green; }",
        ];

        let fail = vec![
            reject(
                "div { whitespace: pre; }",
                None,
                rejected_message("whitespace", "pre"),
                (1, 19, 1, 22),
            ),
            reject(
                "a { transform: translate(1, 1); }",
                None,
                rejected_message("transform", "translate(1, 1)"),
                (1, 16, 1, 31),
            ),
            reject(
                "a { -webkit-transform: translate(1, 1); }",
                None,
                rejected_message("-webkit-transform", "translate(1, 1)"),
                (1, 24, 1, 39),
            ),
            reject(
                "a { color: pink; }",
                None,
                rejected_message("color", "pink"),
                (1, 12, 1, 16),
            ),
            reject(
                "a { background-color: pink; }",
                None,
                rejected_message("background-color", "pink"),
                (1, 23, 1, 27),
            ),
        ];

        Tester::new(
            "declaration-property-value-allowed-list",
            json!([{
                "/^(-webkit-)?transform$/": ["/scale/"],
                "whitespace": "nowrap",
                "/color/": ["/^green/"],
            }]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn irrelevant_css() {
        Tester::new(
            "declaration-property-value-allowed-list",
            json!({"position": ["static"]}),
            ["a { font-size: 1em; }"],
            [],
        )
        .test();
    }

    #[test]
    fn wildcard_property() {
        Tester::new(
            "declaration-property-value-allowed-list",
            json!({"/.*/": ["rebeccapurple"], "background-color": ["none"]}),
            ["a { color: rebeccapurple; background-color: none; }"],
            [],
        )
        .test();
    }
}
