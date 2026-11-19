use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, value_utils::analyze_preceding_gap},
};
use oxc_css_parser::{Span, ast::AtRule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyLineMode {
    Always,
    Never,
}

pub struct AtRuleEmptyLineBefore {
    pub mode: EmptyLineMode,
}

impl Default for AtRuleEmptyLineBefore {
    fn default() -> Self {
        Self {
            mode: EmptyLineMode::Always,
        }
    }
}

impl Rule for AtRuleEmptyLineBefore {
    fn name(&self) -> &'static str {
        "at-rule-empty-line-before"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let gap = analyze_preceding_gap(ctx.source, ctx.comments, at_rule.span.start);
        if gap.is_first_in_file || gap.after_comment {
            return;
        }

        let name: &str = at_rule.name.name;
        let is_blockless = at_rule.block.is_none();
        let prev_same_blockless = is_blockless
            && gap
                .prev_statement_head
                .strip_prefix('@')
                .is_some_and(|prev_name| prev_name.eq_ignore_ascii_case(name));

        let expect_empty = match self.mode {
            EmptyLineMode::Always => !gap.is_first_nested && !prev_same_blockless,
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
                at_rule.span,
                "Expected empty line before at-rule".to_string(),
                Some(Fix::single(insert_span, "\n".to_string())),
            );
        } else if !expect_empty && has_empty {
            let replace_span = Span {
                start: gap.gap_start,
                end: at_rule.span.start,
            };
            ctx.report_with_fix(
                self.name(),
                at_rule.span,
                "Unexpected empty line before at-rule".to_string(),
                Some(Fix::single(replace_span, format!("\n{}", gap.indent_slice))),
            );
        }
    }
}
