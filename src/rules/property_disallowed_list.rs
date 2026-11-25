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
    /// Disallow configured property names.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/property-disallowed-list>
    ///
    /// Set the primary option to a string or an array of strings. Entries are
    /// case-sensitive exact names or slash-delimited regexes. Custom and
    /// vendor-prefixed properties participate; nonstandard syntax is skipped.
    /// An empty list disallows nothing.
    ///
    /// With `["/^margin-/"]`, `margin-top` is reported here:
    ///
    /// ```css
    /// a { margin-top: 0; }
    /// ```
    "property-disallowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct PropertyDisallowedList {
        disallowed: Vec<OptionMatcher>,
    }
}

impl PropertyDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            disallowed: parse_primary_option_matchers(primary),
        }
    }
}

pub fn rejected_message(property: &str) -> String {
    format!("Disallowed property \"{property}\"")
}

impl Rule for PropertyDisallowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if !is_standard_syntax_property(prop) || !options_matches(&self.disallowed, prop) {
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
        let pass = vec!["a { color: red; }", "a { --foo: red; }"];
        Tester::new("property-disallowed-list", json!(""), &pass, []).test();
        Tester::new("property-disallowed-list", json!([]), &pass, []).test();
    }

    #[test]
    fn default_config() {
        let pass = vec![
            "a { color: red; }",
            "a { background: red; }",
            "a { top: 0; color: red; }",
            "a { $scss: 0; }",
            "a { @less: 0; }",
            "a { --bar: 0; }",
        ];
        let fail = vec![
            reject(
                "a { transform: scale(1); }",
                None,
                rejected_message("transform"),
                (1, 5, 1, 14),
            ),
            reject(
                "a { color: red; background-size: cover; }",
                None,
                rejected_message("background-size"),
                (1, 17, 1, 32),
            ),
            reject(
                "a { color: red; -webkit-transform: scale(1); }",
                None,
                rejected_message("-webkit-transform"),
                (1, 17, 1, 34),
            ),
            reject(
                "a { --foo: red; }",
                None,
                rejected_message("--foo"),
                (1, 5, 1, 10),
            ),
        ];
        Tester::new(
            "property-disallowed-list",
            json!(["/^(-webkit-)?transform$/", "background-size", "--foo"]),
            &pass,
            &fail,
        )
        .test();
    }
}
