use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::token::Comment;

pub struct CommentNoEmpty;

impl Rule for CommentNoEmpty {
    fn name(&self) -> &'static str {
        "comment-no-empty"
    }

    fn check_comment<'a>(&self, comment: &'a Comment<'a>, ctx: &mut LintContext<'a>) {
        if comment.content.trim().is_empty() {
            ctx.report(
                self.name(),
                comment.span,
                "Unexpected empty comment".to_string(),
            );
        }
    }
}
