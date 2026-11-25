use oxc_css_parser::{Span, ast::Declaration};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, ident_name, is_standard_syntax_property, options_matches,
        parse_primary_option_matchers,
    },
};

crate::define_stx_lint! {
    /// Allow only configured property names.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/property-allowed-list>
    ///
    /// Set the primary option to a string or an array of strings. Entries are
    /// case-sensitive exact names or slash-delimited regexes. Custom and
    /// vendor-prefixed properties participate; nonstandard syntax is skipped.
    /// An empty list allows none of the names this rule checks.
    ///
    /// With `["color"]`, `margin` is reported here:
    ///
    /// ```css
    /// a { color: red; margin: 0; }
    /// ```
    "property-allowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct PropertyAllowedList {
        allowed: Vec<OptionMatcher>,
    }
}

impl PropertyAllowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            allowed: parse_primary_option_matchers(primary),
        }
    }
}

pub fn rejected_message(property: &str) -> String {
    format!("Disallowed property \"{property}\"")
}

impl Rule for PropertyAllowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if !is_standard_syntax_property(prop) {
            return;
        }
        if options_matches(&self.allowed, prop) {
            return;
        }
        ctx.report(
            self.name(),
            Span {
                start: decl.span.start,
                end: decl.span.start + prop.len() as u32,
            },
            rejected_message(prop),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn empty_config() {
        let fail = vec![
            reject(
                "a { color: red; }",
                None,
                rejected_message("color"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { --foo: red; }",
                None,
                rejected_message("--foo"),
                (1, 5, 1, 10),
            ),
        ];
        Tester::new("property-allowed-list", json!(""), [], &fail).test();
        Tester::new("property-allowed-list", json!([]), [], &fail).test();
    }

    #[test]
    fn default_config() {
        let pass = vec![
            "a { background-size: cover; }",
            "a { transform: scale(1); }",
            "a { --foo: red; }",
            "a { -webkit-transform: scale(1); }",
            "a { transform: scale(1); background-size: cover; }",
            "a { transform: scale(1); -webkit-transform: scale(1); background-size: cover; }",
        ];
        let fail = vec![reject(
            "a { color: red; }",
            None,
            rejected_message("color"),
            (1, 5, 1, 10),
        )];
        Tester::new(
            "property-allowed-list",
            json!(["/^(-webkit-)?transform$/", "background-size", "--foo"]),
            &pass,
            &fail,
        )
        .test();
    }
}
