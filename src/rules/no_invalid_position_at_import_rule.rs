use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, options_matches, parse_option_matchers},
};
use oxc_css_parser::ast::{Statement, Stylesheet};
use serde_json::{Map, Value};

crate::define_stx_lint! {
    /// Require top-level `@import` rules to precede style rules and most at-rules.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/no-invalid-position-at-import-rule>
    ///
    /// Enable with `true`. `@charset`, blockless `@layer`, `@use`, and `@forward`
    /// may precede imports. `ignoreAtRules` accepts exact names or slash-delimited
    /// regexes without `@` for additional at-rules that may precede imports.
    /// Only top-level statements are inspected; declarations and Sass variable
    /// declarations do not mark the end of the import section.
    ///
    /// ```css
    /// a { color: red; }
    /// @import "base.css"; /* Too late */
    /// ```
    "no-invalid-position-at-import-rule", correctness, recommended = true;
    pub struct NoInvalidPositionAtImportRule {
        ignore_at_rules: Vec<OptionMatcher>,
    }
}

impl NoInvalidPositionAtImportRule {
    pub fn from_config(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        }
    }
}

impl Rule for NoInvalidPositionAtImportRule {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let mut seen_disallowed = false;

        for stmt in &stylesheet.statements {
            match stmt {
                Statement::AtRule(at_rule) => {
                    let name: &str = at_rule.name.name;
                    if name.eq_ignore_ascii_case("import") {
                        if seen_disallowed {
                            ctx.report(
                                self.name(),
                                at_rule.span,
                                "Invalid position for @import rule".to_string(),
                            );
                        }
                    } else if !(name.eq_ignore_ascii_case("charset")
                        || name.eq_ignore_ascii_case("use")
                        || name.eq_ignore_ascii_case("forward")
                        || (name.eq_ignore_ascii_case("layer") && at_rule.block.is_none())
                        || options_matches(&self.ignore_at_rules, name))
                    {
                        seen_disallowed = true;
                    }
                }
                Statement::SassVariableDeclaration(_) | Statement::Declaration(_) => {}
                _ => {
                    seen_disallowed = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn ignored_at_rules_do_not_block_or_unblock_imports() {
        Tester::new(
            "no-invalid-position-at-import-rule",
            json!([true, {"ignoreAtRules": ["bar", "/^my-/i"]}]),
            ["@bar 'bar.css';\n@MY-custom {}\n@import 'foo.css';"],
            [
                reject(
                    "@BAR 'bar.css';\n@import 'foo.css';",
                    None,
                    "Invalid position for @import rule",
                    (2, 1, 2, 18),
                ),
                reject(
                    "@baz 'bar.css';\n@my-custom {}\n@import 'foo.css';",
                    None,
                    "Invalid position for @import rule",
                    (3, 1, 3, 18),
                ),
            ],
        )
        .test();
    }

    #[test]
    fn sass_variables_do_not_change_import_position() {
        Tester::new(
            "no-invalid-position-at-import-rule",
            json!(true),
            ["$theme: 1;\n@import 'foo.css';"],
            [reject(
                "a {}\n$theme: 1;\n@import 'foo.css';",
                None,
                "Invalid position for @import rule",
                (3, 1, 3, 18),
            )],
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
