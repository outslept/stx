use crate::{context::LintContext, rules::Rule, utils::scan_direct_declarations};
use oxc_css_parser::ast::SimpleBlock;

pub struct DeclarationBlockSingleLineMaxDeclarations {
    pub max: usize,
}

impl Default for DeclarationBlockSingleLineMaxDeclarations {
    fn default() -> Self {
        Self { max: 1 }
    }
}

impl Rule for DeclarationBlockSingleLineMaxDeclarations {
    fn name(&self) -> &'static str {
        "declaration-block-single-line-max-declarations"
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        if ctx.slice(block.span).contains('\n') {
            return;
        }
        let decls = scan_direct_declarations(ctx.source, block.span);
        if decls.len() > self.max {
            let label = if self.max == 1 {
                "declaration"
            } else {
                "declarations"
            };
            ctx.report(
                self.name(),
                block.span,
                format!("Expected no more than {} {}", self.max, label),
            );
        }
    }
}
