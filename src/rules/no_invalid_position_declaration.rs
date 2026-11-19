use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::{Statement, Stylesheet};

pub struct NoInvalidPositionDeclaration;

impl Rule for NoInvalidPositionDeclaration {
    fn name(&self) -> &'static str {
        "no-invalid-position-declaration"
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        for stmt in &stylesheet.statements {
            if let Statement::Declaration(decl) = stmt {
                ctx.report(
                    self.name(),
                    decl.span,
                    "Unexpected invalid position declaration".to_string(),
                );
            }
        }
    }
}
