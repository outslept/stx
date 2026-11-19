use crate::{
    context::LintContext,
    rules::{
        Rule,
        value_utils::{PatternMatcher, SelectorItemKind, scan_selector_items},
    },
};
use oxc_css_parser::ast::SelectorList;

#[derive(Default)]
pub struct SelectorClassPattern {
    pub matcher: PatternMatcher,
}

impl Rule for SelectorClassPattern {
    fn name(&self) -> &'static str {
        "selector-class-pattern"
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        for item in scan_selector_items(ctx.source, list.span) {
            if item.kind == SelectorItemKind::Class && !self.matcher.is_match(item.name) {
                ctx.report(
                    self.name(),
                    item.span,
                    format!(
                        "Expected class selector \".{}\" to match pattern \"{}\"",
                        item.name, self.matcher.raw_pattern
                    ),
                );
            }
        }
    }
}
