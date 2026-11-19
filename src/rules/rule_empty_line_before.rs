use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, value_utils::analyze_preceding_gap},
};
use oxc_css_parser::{Span, ast::QualifiedRule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleEmptyLineMode {
    Always,
    AlwaysMultiLine,
    Never,
    NeverMultiLine,
}

pub struct RuleEmptyLineBefore {
    pub mode: RuleEmptyLineMode,
}

impl Default for RuleEmptyLineBefore {
    fn default() -> Self {
        Self {
            mode: RuleEmptyLineMode::AlwaysMultiLine,
        }
    }
}

impl Rule for RuleEmptyLineBefore {
    fn name(&self) -> &'static str {
        "rule-empty-line-before"
    }

    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        let is_multi_line = ctx.slice(rule.span).contains('\n');
        match self.mode {
            RuleEmptyLineMode::AlwaysMultiLine | RuleEmptyLineMode::NeverMultiLine
                if !is_multi_line =>
            {
                return;
            }
            _ => {}
        }

        let gap = analyze_preceding_gap(ctx.source, ctx.comments, rule.span.start);
        if gap.is_first_in_file || gap.after_comment {
            return;
        }

        let expect_empty = match self.mode {
            RuleEmptyLineMode::Always | RuleEmptyLineMode::AlwaysMultiLine => !gap.is_first_nested,
            RuleEmptyLineMode::Never | RuleEmptyLineMode::NeverMultiLine => false,
        };

        let has_empty = gap.newline_count >= 2;

        if expect_empty && !has_empty {
            let insert_span = Span {
                start: gap.indent_start,
                end: gap.indent_start,
            };
            ctx.report_with_fix(
                self.name(),
                rule.span,
                "Expected empty line before rule".to_string(),
                Some(Fix::single(insert_span, "\n".to_string())),
            );
        } else if !expect_empty && has_empty {
            let replace_span = Span {
                start: gap.gap_start,
                end: rule.span.start,
            };
            ctx.report_with_fix(
                self.name(),
                rule.span,
                "Unexpected empty line before rule".to_string(),
                Some(Fix::single(replace_span, format!("\n{}", gap.indent_slice))),
            );
        }
    }
}
