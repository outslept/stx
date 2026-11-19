use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::Stylesheet};

pub struct NoEmptySource;

impl Rule for NoEmptySource {
    fn name(&self) -> &'static str {
        "no-empty-source"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        if ctx.source.trim().is_empty() {
            ctx.report(
                self.name(),
                Span {
                    start: 0,
                    end: ctx.source.len() as u32,
                },
                "Unexpected empty source".to_string(),
            );
        }
    }
}
