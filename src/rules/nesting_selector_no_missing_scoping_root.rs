use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{OptionMatcher, at_rule_prelude_span, options_matches, parse_option_matchers},
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, QualifiedRule},
};

crate::define_stx_lint! {
    /// Disallow the nesting selector `&` when it has no scoping root.
    ///
    /// Enable with `true`. The secondary `ignoreAtRules` option accepts a string
    /// or array of at-rule names without `@` and `/regex/` patterns. A matching ancestor at-rule,
    /// an enclosing style rule, or an enclosing `@scope` satisfies the check.
    /// This checks the text of style selectors and `@scope` preludes, not
    /// declarations or other at-rule preludes.
    ///
    /// Example: `& .item {}` is reported, but `.list { & .item {} }` passes.
    "nesting-selector-no-missing-scoping-root", correctness;
    pub struct NestingSelectorNoMissingScopingRoot {
        ignore_at_rules: Vec<OptionMatcher>,
    }
}

impl NestingSelectorNoMissingScopingRoot {
    pub fn new(s: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        Self {
            ignore_at_rules: parse_option_matchers(s, "ignoreAtRules"),
        }
    }
    fn has_root(&self, ctx: &LintContext<'_>) -> bool {
        ctx.ancestors.iter().any(|p| match p {
            AstParent::QualifiedRule(_) => true,
            AstParent::AtRule(a) => {
                a.name.name.eq_ignore_ascii_case("scope")
                    || options_matches(&self.ignore_at_rules, a.name.name)
            }
            _ => false,
        })
    }
    fn report_ampersands<'a>(&self, raw: &'a str, base: u32, ctx: &mut LintContext<'a>) {
        let bytes = raw.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'&' {
                ctx.report(
                    self.name(),
                    Span {
                        start: base + i as u32,
                        end: base + i as u32 + 1,
                    },
                    "Missing scoping root",
                );
            }
        }
    }
}
impl Rule for NestingSelectorNoMissingScopingRoot {
    fn name(&self) -> &'static str {
        Self::NAME
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(rule.selector.span);
        if !raw.contains('&') || self.has_root(ctx) {
            return;
        }
        self.report_ampersands(raw, rule.selector.span.start, ctx);
    }
    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("scope") {
            return;
        }
        let Some(sp) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let raw = ctx.slice(sp);
        if !raw.contains('&') || self.has_root(ctx) {
            return;
        }
        self.report_ampersands(raw, sp.start, ctx);
    }
}
#[cfg(test)]
mod tests {

    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec![
            "a {}",
            "a /* & */ {}",
            "a { & {} }",
            "a { && {} }",
            "a { & .foo {} }",
            "a { .foo & {} }",
            "a { :not(&) {} }",
            "a { @media all { & {} } }",
            "@scope (a) { & {} }",
            "a { @scope (&) {} }",
        ];
        let fail = vec![
            reject("& {}", None, "Missing scoping root", (1, 1, 1, 2)),
            reject(".foo & {}", None, "Missing scoping root", (1, 6, 1, 7)),
            reject(":not(&) {}", None, "Missing scoping root", (1, 6, 1, 7)),
            reject_warnings(
                "&& {}",
                None,
                vec![
                    ("Missing scoping root".to_string(), (1, 1, 1, 2)),
                    ("Missing scoping root".to_string(), (1, 2, 1, 3)),
                ],
            ),
        ];
        Tester::new(
            "nesting-selector-no-missing-scoping-root",
            json!(true),
            &pass,
            &fail,
        )
        .test();
    }
}
