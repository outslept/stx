use crate::{
    context::{AstParent, LintContext},
    fix::Fix,
    rules::{Rule, at_rule_empty_line_before::EmptyLineMode, value_utils::analyze_preceding_gap},
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

pub struct CustomPropertyEmptyLineBefore {
    pub mode: EmptyLineMode,
}

impl Default for CustomPropertyEmptyLineBefore {
    fn default() -> Self {
        Self {
            mode: EmptyLineMode::Always,
        }
    }
}

impl Rule for CustomPropertyEmptyLineBefore {
    fn name(&self) -> &'static str {
        "custom-property-empty-line-before"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if !prop.starts_with("--") {
            return;
        }

        for parent in ctx.ancestry.iter().rev() {
            if let AstParent::SimpleBlock(block) = parent {
                if !ctx.slice(block.span).contains('\n') {
                    return;
                }
                break;
            }
        }

        let gap = analyze_preceding_gap(ctx.source, ctx.comments, decl.span.start);
        if gap.is_first_in_file || gap.after_comment {
            return;
        }

        let after_custom_prop = gap.prev_statement_head.starts_with("--");
        let expect_empty = match self.mode {
            EmptyLineMode::Always => !gap.is_first_nested && !after_custom_prop,
            EmptyLineMode::Never => false,
        };

        let has_empty = gap.newline_count >= 2;

        if expect_empty && !has_empty {
            let insert_span = Span {
                start: gap.indent_start,
                end: gap.indent_start,
            };
            ctx.report_with_fix(
                self.name(),
                decl.span,
                "Expected empty line before custom property".to_string(),
                Some(Fix::single(insert_span, "\n".to_string())),
            );
        } else if !expect_empty && has_empty {
            let replace_span = Span {
                start: gap.gap_start,
                end: decl.span.start,
            };
            ctx.report_with_fix(
                self.name(),
                decl.span,
                "Unexpected empty line before custom property".to_string(),
                Some(Fix::single(replace_span, format!("\n{}", gap.indent_slice))),
            );
        }
    }
}
