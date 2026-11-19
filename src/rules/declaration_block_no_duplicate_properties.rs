use std::collections::HashSet;

use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::{InterpolableIdent, SimpleBlock, Statement};

pub struct DeclarationBlockNoDuplicateProperties;

impl Rule for DeclarationBlockNoDuplicateProperties {
    fn name(&self) -> &'static str {
        "declaration-block-no-duplicate-properties"
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        let mut seen = HashSet::new();

        for stmt in &block.statements {
            if let Statement::Declaration(decl) = stmt {
                if let InterpolableIdent::Literal(ident) = &decl.name {
                    let prop: &str = ident.name;
                    if prop.starts_with("--") {
                        continue;
                    }
                    let lower = prop.to_ascii_lowercase();
                    if !seen.insert(lower) {
                        ctx.report(
                            self.name(),
                            ident.span,
                            format!("Unexpected duplicate \"{prop}\""),
                        );
                    }
                }
            }
        }
    }
}
