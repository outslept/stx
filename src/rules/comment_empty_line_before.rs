use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, at_rule_empty_line_before::EmptyLineMode, value_utils::analyze_preceding_gap},
};
use oxc_css_parser::{Span, token::Comment};

pub struct CommentEmptyLineBefore {
    pub mode: EmptyLineMode,
}

impl Default for CommentEmptyLineBefore {
    fn default() -> Self {
        Self {
            mode: EmptyLineMode::Always,
        }
    }
}

impl Rule for CommentEmptyLineBefore {
    fn name(&self) -> &'static str {
        "comment-empty-line-before"
    }

    fn check_comment<'a>(&self, comment: &'a Comment<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(comment.span);
        if raw.len() >= 4 {
            let inner = raw[2..raw.len() - 2].trim();
            if inner.starts_with("stylelint-") {
                return;
            }
        }

        let gap = analyze_preceding_gap(ctx.source, ctx.comments, comment.span.start);
        if gap.is_first_in_file || gap.after_comment || gap.newline_count == 0 {
            return;
        }

        let expect_empty = match self.mode {
            EmptyLineMode::Always => !gap.is_first_nested,
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
                comment.span,
                "Expected empty line before comment".to_string(),
                Some(Fix::single(insert_span, "\n".to_string())),
            );
        } else if !expect_empty && has_empty {
            let replace_span = Span {
                start: gap.gap_start,
                end: comment.span.start,
            };
            ctx.report_with_fix(
                self.name(),
                comment.span,
                "Unexpected empty line before comment".to_string(),
                Some(Fix::single(replace_span, format!("\n{}", gap.indent_slice))),
            );
        }
    }
}
