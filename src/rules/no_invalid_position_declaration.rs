use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        OptionMatcher, options_matches, parse_option_matchers,
        reference::{
            DECLARATION_CONTAINING_AT_KEYWORDS, NESTING_SUPPORTED_AT_KEYWORDS, in_set_ignore_case,
        },
    },
};
use oxc_css_parser::ast::{AtRule, Declaration};
use serde_json::{Map, Value};

crate::define_stx_lint! {
    /// Report declarations outside a rule or at-rule that can contain them.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/no-invalid-position-declaration>
    ///
    /// Enable with `true`. Declarations inside style rules, keyframes, and
    /// supported at-rule contexts such as `@font-face` and `@page` are allowed.
    /// `ignoreAtRules` accepts exact names or slash-delimited regexes without
    /// `@`; a matching ancestor exempts the declaration.
    ///
    /// ```css
    /// @media print { color: red; } /* Needs a style rule inside @media */
    /// ```
    "no-invalid-position-declaration", correctness, recommended = true;
    pub struct NoInvalidPositionDeclaration {
        ignore_at_rules: Vec<OptionMatcher>,
    }
}

impl NoInvalidPositionDeclaration {
    pub fn from_config(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        }
    }
}

fn allows_declarations(at_rule: &AtRule<'_>) -> bool {
    matches!(
        at_rule.name.name.to_ascii_lowercase().as_str(),
        "counter-style"
            | "font-face"
            | "font-palette-values"
            | "page"
            | "property"
            | "viewport"
            | "-ms-viewport"
    )
}

impl Rule for NoInvalidPositionDeclaration {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if ctx.in_keyframes()
            || ctx
                .ancestors
                .iter()
                .any(|ancestor| matches!(ancestor, AstParent::QualifiedRule(_)))
        {
            return;
        }
        if ctx.ancestors.iter().any(|ancestor| {
            matches!(ancestor, AstParent::AtRule(at_rule) if options_matches(&self.ignore_at_rules, at_rule.name.name))
        }) {
            return;
        }
        if ctx.ancestors.iter().any(|ancestor| {
            matches!(ancestor, AstParent::AtRule(at_rule) if at_rule.name.name.eq_ignore_ascii_case("page"))
        }) && ctx.ancestors.iter().any(|ancestor| {
            matches!(ancestor, AstParent::AtRule(at_rule) if matches!(at_rule.name.name.to_ascii_lowercase().as_str(), "top-left-corner" | "top-left" | "top-center" | "top-right" | "top-right-corner" | "right-top" | "right-middle" | "right-bottom" | "bottom-right-corner" | "bottom-right" | "bottom-center" | "bottom-left" | "bottom-left-corner" | "left-bottom" | "left-middle" | "left-top"))
        }) {
            return;
        }
        if let Some(AstParent::AtRule(parent)) = ctx.ancestors.last()
            && (allows_declarations(parent)
                || in_set_ignore_case(DECLARATION_CONTAINING_AT_KEYWORDS, parent.name.name)
                || (in_set_ignore_case(NESTING_SUPPORTED_AT_KEYWORDS, parent.name.name)
                    && ctx.ancestors.iter().any(|ancestor| {
                        matches!(ancestor, AstParent::AtRule(at_rule) if in_set_ignore_case(DECLARATION_CONTAINING_AT_KEYWORDS, at_rule.name.name))
                    })))
        {
            return;
        }
        let span = decl.span();
        let end = span.end + u32::from(ctx.source.as_bytes().get(span.end as usize) == Some(&b';'));
        ctx.report(
            self.name(),
            oxc_css_parser::Span { end, ..span },
            "Invalid position for declaration",
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_warnings};
    use serde_json::json;

    #[test]
    fn declaration_contexts_and_span() {
        let fail = [
            reject_warnings(
                "color: red;\n@layer { color: blue; }",
                None,
                vec![
                    ("Invalid position for declaration".into(), (1, 1, 1, 12)),
                    ("Invalid position for declaration".into(), (2, 10, 2, 22)),
                ],
            ),
            reject_warnings(
                "color: red",
                None,
                vec![("Invalid position for declaration".into(), (1, 1, 1, 11))],
            ),
        ];
        Tester::new(
            "no-invalid-position-declaration",
            json!(true),
            [
                "a { color: red; }",
                "@font-face { font-family: x; }",
                "@page { margin: 1px; }",
                "@page { @top-right { content: \"foo\"; } }",
                "@keyframes foo { 0% { color: red; } }",
                "@property --x { initial-value: 1px; }",
            ],
            &fail,
        )
        .test();
    }

    #[test]
    fn mixin_and_scope_allow_direct_and_nested_declarations() {
        Tester::new(
            "no-invalid-position-declaration",
            json!(true),
            [
                "@mixin --foo() { color: red; }",
                "@scope (.foo) { color: red; }",
                "@mixin --foo() { @supports (text-wrap: pretty) { text-wrap: pretty; } }",
                "@scope (.foo) { @media all { color: red; } }",
            ],
            [reject_warnings(
                "@media all { color: red; }",
                None,
                vec![("Invalid position for declaration".into(), (1, 14, 1, 25))],
            )],
        )
        .test();
    }

    #[test]
    fn ignores_configured_at_rules() {
        Tester::new(
            "no-invalid-position-declaration",
            json!([true, {"ignoreAtRules": ["layer", "/^custom-/"]}]),
            ["@layer { color: red; } @custom-rule { color: blue; }"],
            &[],
        )
        .test();
    }
}
