use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{ast::Declaration, token::Comment};

pub struct NoInvalidDoubleSlashComments;

impl Rule for NoInvalidDoubleSlashComments {
    fn name(&self) -> &'static str {
        "no-invalid-double-slash-comments"
    }

    fn check_comment<'a>(&self, comment: &'a Comment<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(comment.span);
        if raw.starts_with("//") {
            ctx.report(
                self.name(),
                comment.span,
                "Unexpected double-slash CSS comment".to_string(),
            );
        }
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(decl.span).trim_start();
        if raw.starts_with("//") {
            ctx.report(
                self.name(),
                decl.span,
                "Unexpected double-slash CSS comment".to_string(),
            );
        }
    }
}
