use crate::{
    context::LintContext,
    rules::Rule,
    utils::{PatternMatcher, SelectorItemKind, scan_selector_items},
};
use oxc_css_parser::ast::SelectorList;

crate::define_stx_lint! {
    /// Requires ID selector names to match a regular expression.
    ///
    /// Set to a regex string, either bare or slash-delimited. Match against the ID
    /// without its leading `#`. Does not fix violations.
    ///
    /// Example: `#foo {}` matches `^[a-z]+$` but does not match `^[A-Z]+$`.
    "selector-id-pattern", style;
    #[derive(Default)]
    pub struct SelectorIdPattern {
        pub matcher: PatternMatcher,
    }
}

impl Rule for SelectorIdPattern {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        let raw_selector = ctx.slice(list.span);
        if raw_selector.contains("#{") || raw_selector.contains("@{") {
            return;
        }

        for item in scan_selector_items(ctx.source, list.span) {
            if item.kind == SelectorItemKind::Id && !self.matcher.is_match(item.name) {
                ctx.report(
                    self.name(),
                    item.span,
                    format!(
                        "Expected \"#{}\" to match pattern \"{}\"",
                        item.name, self.matcher.raw_pattern
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject, reject_start, reject_warnings};

    const NAME: &str = super::SelectorIdPattern::NAME;
    const CSS_PASS: &[&str] = &[
        "a {}",
        ".foo {}",
        "[foo='bar'] {}",
        "#FOO {}",
        "a .foo > [foo='bar'], #FOO {}",
        "a /* #foo */ {}",
        ":root { --foo: 1px; }",
        "html { --foo: 1px; }",
        ":root { --custom-property-set: {} }",
        "html { --custom-property-set: {} }",
        "/* stylelint-disable-next-line selector-id-pattern */\na #foo, /* stylelint-disable-next-line selector-id-pattern */\na #bar,\n/* stylelint-disable-next-line selector-id-pattern */\na #foo\n{}",
    ];

    fn expected(selector: &str, pattern: &str) -> String {
        format!("Expected \"{selector}\" to match pattern \"{pattern}\"")
    }

    #[test]
    fn regex_config() {
        let pattern = "/^[A-Z]+$/";
        let fail = [
            reject("a #foo {}", None, expected("#foo", pattern), (1, 3, 1, 7)),
            reject(
                "#ABABA > #bar {}",
                None,
                expected("#bar", pattern),
                (1, 10, 1, 14),
            ),
            reject_warnings(
                "/* a comment */\na #foo, /* a comment */\na #bar,\n/* a comment */\na #foo\n{}",
                None,
                vec![
                    (expected("#foo", pattern), (2, 3, 2, 7)),
                    (expected("#bar", pattern), (3, 3, 3, 7)),
                    (expected("#foo", pattern), (5, 3, 5, 7)),
                ],
            ),
        ];
        Tester::new(NAME, json!([pattern]), CSS_PASS, fail).test();
    }

    #[test]
    fn string_config() {
        let pattern = "^[A-Z]+$";
        let fail = [
            reject_start("a #foo {}", None, expected("#foo", pattern), (1, 3)),
            reject_start("#ABABA > #bar {}", None, expected("#bar", pattern), (1, 10)),
        ];
        Tester::new(NAME, json!([pattern]), CSS_PASS, fail).test();
    }

    #[test]
    fn scss_syntax() {
        let pattern = "/^[A-Z]+$/";
        let pass = [
            "@for $n from 1 through 5 { #a#{$n} {} }",
            ".#{$a} {}",
            ".foo { .#{$a} {} }",
            "a { #ABABA {} }",
        ];
        let fail = [reject_start(
            "a { #bar {} }",
            None,
            expected("#bar", pattern),
            (1, 5),
        )];
        Tester::new(NAME, json!([pattern]), pass, fail)
            .with_custom_syntax("postcss-scss")
            .test();
    }
}
