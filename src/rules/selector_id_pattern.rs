use crate::{
    context::LintContext,
    rules::{
        Rule,
        value_utils::{PatternMatcher, SelectorItemKind, scan_selector_items},
    },
};
use oxc_css_parser::ast::SelectorList;

#[derive(Default)]
pub struct SelectorIdPattern {
    pub matcher: PatternMatcher,
}

impl Rule for SelectorIdPattern {
    fn name(&self) -> &'static str {
        "selector-id-pattern"
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
