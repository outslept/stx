use super::keyframe_block_no_duplicate_selectors::{
    is_keyframes, selector_span, walk_keyframe_blocks,
};
use crate::{context::LintContext, fix::Fix, rules::Rule};
use oxc_css_parser::ast::AtRule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyframeSelectorMode {
    PercentageUnlessWithinKeywordOnlyBlock,
    Percentage,
    Keyword,
}

crate::define_stx_lint! {
    /// Enforces keyword or percentage notation for keyframe endpoints.
    ///
    /// Primary options:
    /// - `"percentage"`: replace `from` and `to` with `0%` and `100%`.
    /// - `"percentage-unless-within-keyword-only-block"`: do the same unless
    ///   every selector in the `@keyframes` rule is `from` or `to`.
    /// - `"keyword"`: replace bare `0%` and `100%` with `from` and `to`.
    ///
    /// Auto-fix changes only the selector, leaving other percentages and
    /// timeline ranges such as `entry 0%` alone.
    ///
    /// ```css
    /// @keyframes fade { from { opacity: 0; } } /* fixes to 0% with "percentage" */
    /// ```
    "keyframe-selector-notation", style, fix = fix;
    pub struct KeyframeSelectorNotation {
        pub mode: KeyframeSelectorMode,
    }
}

impl Default for KeyframeSelectorNotation {
    fn default() -> Self {
        Self {
            mode: KeyframeSelectorMode::PercentageUnlessWithinKeywordOnlyBlock,
        }
    }
}

fn keyword_percentage(selector: &str) -> Option<&'static str> {
    if selector.eq_ignore_ascii_case("from") {
        Some("0%")
    } else if selector.eq_ignore_ascii_case("to") {
        Some("100%")
    } else {
        None
    }
}

impl Rule for KeyframeSelectorNotation {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !is_keyframes(at_rule) {
            return;
        }
        let Some(block) = &at_rule.block else {
            return;
        };

        let mut selectors = Vec::new();
        walk_keyframe_blocks(block, &mut |frame| {
            selectors.extend(frame.selectors.iter().map(selector_span));
        });
        if self.mode == KeyframeSelectorMode::PercentageUnlessWithinKeywordOnlyBlock
            && selectors
                .iter()
                .all(|&span| keyword_percentage(ctx.slice(span)).is_some())
        {
            return;
        }

        for span in selectors {
            let raw = ctx.slice(span);
            let expected = match self.mode {
                KeyframeSelectorMode::Keyword => match raw {
                    "0%" => Some("from"),
                    "100%" => Some("to"),
                    _ => None,
                },
                _ => keyword_percentage(raw),
            };
            if let Some(expected) = expected {
                ctx.report_with_fix(
                    self.name(),
                    span,
                    format!("Expected \"{raw}\" to be \"{expected}\""),
                    Fix::single(span, expected),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    const RULE_NAME: &str = "keyframe-selector-notation";

    fn expected(original: &str, replacement: &str) -> String {
        format!("Expected \"{original}\" to be \"{replacement}\"")
    }

    #[test]
    fn percentage_fixes_only_the_selectors() {
        let fail = [reject_warnings(
            "@keyframes foo {\n  /* lead, { */\n  FROM, /* between, } */\n  to /* tail, */ {}\n}",
            "@keyframes foo {\n  /* lead, { */\n  0%, /* between, } */\n  100% /* tail, */ {}\n}",
            vec![
                (expected("FROM", "0%"), (3, 3, 3, 7)),
                (expected("to", "100%"), (4, 3, 4, 5)),
            ],
        )];
        Tester::new(RULE_NAME, json!("percentage"), [], fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn keyword_only_exception_uses_the_entire_block() {
        let pass = ["@keyframes foo { /* before, */ from, TO /* after, */ {} }"];
        let fail = [reject(
            "@-o-keyframes foo {\n  from {}\n  entry 0% {}\n}",
            "@-o-keyframes foo {\n  0% {}\n  entry 0% {}\n}",
            expected("from", "0%"),
            (2, 3, 2, 7),
        )];
        Tester::new(
            RULE_NAME,
            json!("percentage-unless-within-keyword-only-block"),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn keyword_only_fixes_bare_endpoint_percentages() {
        let fail = [reject(
            "@keyframes foo {\n  entry 0%, /* keep, */ 100% {}\n}",
            "@keyframes foo {\n  entry 0%, /* keep, */ to {}\n}",
            expected("100%", "to"),
            (2, 25, 2, 29),
        )];
        Tester::new(RULE_NAME, json!("keyword"), [], fail)
            .with_fix(true)
            .test();
    }
}
