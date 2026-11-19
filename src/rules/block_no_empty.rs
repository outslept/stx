use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::SimpleBlock;

pub struct BlockNoEmpty;

impl Rule for BlockNoEmpty {
    fn name(&self) -> &'static str {
        "block-no-empty"
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        if !block.statements.is_empty() {
            return;
        }

        let has_comment_inside = ctx
            .comments
            .binary_search_by(|c| {
                if c.span.end <= block.span.start {
                    std::cmp::Ordering::Less
                } else if c.span.start >= block.span.end {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_ok();

        if has_comment_inside {
            return;
        }

        ctx.report(
            self.name(),
            block.span,
            "Unexpected empty block".to_string(),
        );
    }
}
