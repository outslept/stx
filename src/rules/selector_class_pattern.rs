use crate::{
    context::LintContext,
    rules::Rule,
    utils::{PatternMatcher, SelectorItemKind, scan_selector_items},
};
use oxc_css_parser::ast::SelectorList;

crate::define_stx_lint! {
    /// Requires class selector names to match a regular expression.
    ///
    /// Set to a regex string such as `"^[A-Z]+$"` or `"/^[A-Z]+$/"`. Match against the
    /// class name without its leading dot. Does not fix violations.
    ///
    /// Example: `.foo {}` does not match `^[A-Z]+$`.
    "selector-class-pattern", style;
    #[derive(Default)]
    pub struct SelectorClassPattern {
        pub matcher: PatternMatcher,
    }
}

impl Rule for SelectorClassPattern {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        for item in scan_selector_items(ctx.source, list.span) {
            if item.kind == SelectorItemKind::Class && !self.matcher.is_match(item.name) {
                ctx.report(
                    self.name(),
                    item.span,
                    format!(
                        "Expected \".{}\" to match pattern \"{}\"",
                        item.name, self.matcher.raw_pattern
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn pattern_messages() {
        Tester::new(
            "selector-class-pattern",
            json!("/^[A-Z]+$/"),
            ["a #foo > [foo='bar'], .FOO {}", "a /* .foo */ {}"],
            [reject(
                "a,\n\t.foo {}",
                None,
                "Expected \".foo\" to match pattern \"/^[A-Z]+$/\"",
                (2, 2, 2, 6),
            )],
        )
        .test();
        Tester::new(
            "selector-class-pattern",
            json!("^[A-Z]+$"),
            [".FOO {}"],
            [reject(
                ".ABABA > .bar {}",
                None,
                "Expected \".bar\" to match pattern \"^[A-Z]+$\"",
                (1, 10, 1, 14),
            )],
        )
        .test();
    }
}
