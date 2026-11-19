use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::Declaration;

pub struct DeclarationNoImportant;

impl Rule for DeclarationNoImportant {
    fn name(&self) -> &'static str {
        "declaration-no-important"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if let Some(important) = &decl.important {
            ctx.report(
                self.name(),
                important.span,
                "Unexpected !important".to_string(),
            );
        }
    }
}
