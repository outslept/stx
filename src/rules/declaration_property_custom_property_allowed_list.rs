use oxc_css_parser::ast::Declaration;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, decl_value_span, ident_name, is_custom_property,
        is_standard_syntax_property, is_standard_syntax_value, tokenize_value,
    },
};

crate::define_stx_lint! {
    /// Restricts which custom properties a declaration may reference with `var()`.
    ///
    /// The primary option maps property names or `/regex/` patterns to one or
    /// more allowed custom property names or `/regex/` patterns. All matching
    /// entries contribute to the allow list. Declarations with no matching
    /// entry are unrestricted; nested `var()` calls are also checked.
    ///
    /// With `{"color": "--accent"}`:
    ///
    /// ```css
    /// a { color: var(--accent); } /* passes */
    /// a { color: var(--other); } /* reported */
    /// ```
    "declaration-property-custom-property-allowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct DeclarationPropertyCustomPropertyAllowedList {
        entries: Vec<(OptionMatcher, Vec<OptionMatcher>)>,
    }
}

impl DeclarationPropertyCustomPropertyAllowedList {
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

pub fn rejected_message(property: &str, custom_property: &str) -> String {
    format!("Disallowed custom property \"{custom_property}\" for property \"{property}\"")
}

impl Rule for DeclarationPropertyCustomPropertyAllowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if !ctx.is_standard_declaration(decl) || !is_standard_syntax_property(prop) {
            return;
        }

        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };

        let mut any_match = false;
        let mut allowed: Vec<&OptionMatcher> = Vec::new();
        for (key, allowed_for_key) in &self.entries {
            if key.is_match(prop) {
                any_match = true;
                allowed.extend(allowed_for_key);
            }
        }
        if !any_match {
            return;
        }

        let tokens = tokenize_value(ctx.source, value_span);
        for (index, token) in tokens.iter().enumerate() {
            if token.kind != ValueTokenKind::Function {
                continue;
            }
            let function_name = token.raw.strip_suffix('(').unwrap_or(token.raw);
            if !function_name.eq_ignore_ascii_case("var") {
                continue;
            }
            let Some(first) = tokens.get(index + 1) else {
                continue;
            };
            let custom_property = first.raw;
            if !is_custom_property(custom_property) || !is_standard_syntax_value(custom_property) {
                continue;
            }
            if allowed
                .iter()
                .any(|matcher| matcher.is_match(custom_property))
            {
                continue;
            }
            ctx.report(
                self.name(),
                first.span,
                rejected_message(prop, custom_property),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    fn rejected(property: &str, custom_property: &str) -> String {
        format!("Disallowed custom property \"{custom_property}\" for property \"{property}\"")
    }

    #[test]
    fn default_config() {
        let pass = [
            "a { color: red; }",
            "a { color: var(); }",
            "a { color: var(--foo-baz); }",
            "a { background-color: var(--foo-baz); }",
            "a { border: var(--foo-baz) solid var(--bar); }",
            "a { top: var(--baz); }",
        ];
        let fail = [
            reject(
                "a { color: var(--bar); }",
                None,
                rejected("color", "--bar"),
                (1, 16, 1, 21),
            ),
            reject(
                "a { background-color: var(--bar); }",
                None,
                rejected("background-color", "--bar"),
                (1, 27, 1, 32),
            ),
            reject_warnings(
                "a { border: var(--baz) solid var(--qux); }",
                None,
                vec![
                    (rejected("border", "--baz"), (1, 17, 1, 22)),
                    (rejected("border", "--qux"), (1, 34, 1, 39)),
                ],
            ),
            reject(
                "a { border: calc(var(--baz) * 2) solid; }",
                None,
                rejected("border", "--baz"),
                (1, 22, 1, 27),
            ),
            reject(
                "a { background-color: var(--foo-baz, var(--qux)); }",
                None,
                rejected("background-color", "--qux"),
                (1, 42, 1, 47),
            ),
            reject(
                "a { color: VAR(--baz); }",
                None,
                rejected("color", "--baz"),
                (1, 16, 1, 21),
            ),
        ];

        Tester::new(
            "declaration-property-custom-property-allowed-list",
            json!([{
                "/color/": ["/^--foo-/"],
                "border": ["/^--foo-/", "--bar"],
            }]),
            pass,
            fail,
        )
        .test();
    }
}
