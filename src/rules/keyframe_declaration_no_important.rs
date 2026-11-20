use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::Declaration;

pub struct KeyframeDeclarationNoImportant;

impl Rule for KeyframeDeclarationNoImportant {
    fn name(&self) -> &'static str {
        "keyframe-declaration-no-important"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if let Some(important) = &decl.important {
            if ctx.in_keyframes() {
                ctx.report(
                    self.name(),
                    important.span,
                    "Disallowed !important".to_string(),
                );
            }
        }
    }
}
