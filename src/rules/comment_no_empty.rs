use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::token::Comment;

pub struct CommentNoEmpty;

impl Rule for CommentNoEmpty {
    fn name(&self) -> &'static str {
        "comment-no-empty"
    }

    fn check_comment<'a>(&self, comment: &'a Comment<'a>, ctx: &mut LintContext<'a>) {
        if !ctx.slice(comment.span).starts_with("/*") {
            return;
        }
        if comment.content.trim().is_empty() {
            ctx.report(self.name(), comment.span, "Empty comment".to_string());
        }
    }
}
