use std::collections::HashSet;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{ident_span, is_standard_syntax_selector, strip_comments},
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, KeyframeBlock, KeyframeSelector, SimpleBlock, Statement},
};

pub struct KeyframeBlockNoDuplicateSelectors;

pub(super) fn is_keyframes(at_rule: &AtRule<'_>) -> bool {
    [
        "keyframes",
        "-webkit-keyframes",
        "-moz-keyframes",
        "-ms-keyframes",
        "-o-keyframes",
    ]
    .iter()
    .any(|name| at_rule.name.raw.eq_ignore_ascii_case(name))
}

pub(super) fn walk_keyframe_blocks<'a>(
    block: &'a SimpleBlock<'a>,
    visit: &mut impl FnMut(&'a KeyframeBlock<'a>),
) {
    for statement in &block.statements {
        match statement {
            Statement::KeyframeBlock(frame) => {
                visit(frame);
                walk_keyframe_blocks(&frame.block, visit);
            }
            Statement::AtRule(rule) => {
                if let Some(block) = &rule.block {
                    walk_keyframe_blocks(block, visit);
                }
            }
            Statement::QualifiedRule(rule) => walk_keyframe_blocks(&rule.block, visit),
            Statement::SassIfAtRule(rule) => {
                walk_keyframe_blocks(&rule.if_clause.block, visit);
                for clause in &rule.else_if_clauses {
                    walk_keyframe_blocks(&clause.block, visit);
                }
                if let Some(block) = &rule.else_clause {
                    walk_keyframe_blocks(block, visit);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn selector_span(selector: &KeyframeSelector<'_>) -> Span {
    match selector {
        KeyframeSelector::Ident(name) => ident_span(name),
        KeyframeSelector::Percentage(percentage) => percentage.span,
        KeyframeSelector::TimelineRange(range) => range.span,
    }
}

impl Rule for KeyframeBlockNoDuplicateSelectors {
    fn name(&self) -> &'static str {
        "keyframe-block-no-duplicate-selectors"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !is_keyframes(at_rule) {
            return;
        }
        let Some(block) = &at_rule.block else {
            return;
        };

        let mut seen = HashSet::new();
        walk_keyframe_blocks(block, &mut |frame| {
            let header = ctx.slice_range(frame.span.start, frame.block.span.start);
            let stripped;
            let header = if header.contains("/*") {
                stripped = strip_comments(header);
                &stripped
            } else {
                header
            };
            if !is_standard_syntax_selector(header) {
                return;
            }

            for selector in &frame.selectors {
                let span = selector_span(selector);
                let raw = ctx.slice(span);
                if !seen.insert(raw.to_lowercase()) {
                    ctx.report(
                        self.name(),
                        span,
                        format!("Duplicate keyframe selector \"{raw}\""),
                    );
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    const RULE_NAME: &str = "keyframe-block-no-duplicate-selectors";

    fn duplicate(selector: &str) -> String {
        format!("Duplicate keyframe selector \"{selector}\"")
    }

    #[test]
    fn duplicates_across_blocks_and_lists() {
        let pass = [
            "@keyframes foo { from {} 0% {} to {} 100% {} }",
            "@keyframes one { from {} } @keyframes two { FROM {} }",
        ];
        let fail = [
            reject_warnings(
                "@keyframes foo {\n  from {}\n  /* before, { */\n  FROM /* after, } */, /* between, */\n  50%, from,\n  /* before */\n  from {}\n}",
                None,
                vec![
                    (duplicate("FROM"), (4, 3, 4, 7)),
                    (duplicate("from"), (5, 8, 5, 12)),
                    (duplicate("from"), (7, 3, 7, 7)),
                ],
            ),
            reject(
                "@-MS-KEYFRAMES foo {\n  0%, 0% {}\n}",
                None,
                duplicate("0%"),
                (2, 7, 2, 9),
            ),
        ];
        Tester::new(RULE_NAME, json!(true), pass, fail).test();
    }

    #[test]
    fn scss_interpolated_selectors() {
        let pass = [
            "@keyframes foo { #{$bar} {} #{$bar} {} }",
            "@keyframes foo { from, #{$bar} {} from {} }",
        ];
        let fail = [reject(
            "@keyframes foo {\n  #{$bar} {}\n  @if $ok { from {} }\n  FROM {}\n}",
            None,
            duplicate("FROM"),
            (4, 3, 4, 7),
        )];
        Tester::new(RULE_NAME, json!(true), pass, fail)
            .with_custom_syntax("postcss-scss")
            .test();
    }
}
