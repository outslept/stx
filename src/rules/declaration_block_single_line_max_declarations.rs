use crate::{context::LintContext, rules::Rule, utils::scan_direct_declarations};
use oxc_css_parser::ast::SimpleBlock;

crate::define_stx_lint! {
    /// Limits how many declarations may appear in a single-line block.
    ///
    /// Set the primary option to a non-negative integer. Only declarations
    /// directly in the block count; blocks spanning multiple lines are exempt.
    ///
    /// ```css
    /// a { color: red; background: blue; } /* reported with 1 */
    /// ```
    "declaration-block-single-line-max-declarations", style;
    pub struct DeclarationBlockSingleLineMaxDeclarations {
        pub max: usize,
    }
}

impl Default for DeclarationBlockSingleLineMaxDeclarations {
    fn default() -> Self {
        Self { max: 1 }
    }
}

impl Rule for DeclarationBlockSingleLineMaxDeclarations {
    fn name(&self) -> &'static str {
        Self::NAME
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
