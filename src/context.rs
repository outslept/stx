use crate::{
    diagnostic::{Diagnostic, Severity, StylelintWarning},
    disable::DisableDirectives,
    fix::Fix,
    line_index::LineIndex,
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, KeyframeBlock, QualifiedRule, Stylesheet},
    token::Comment,
};

#[derive(Debug, Clone, Copy)]
pub enum AstParent<'a> {
    QualifiedRule(&'a QualifiedRule<'a>),
    AtRule(&'a AtRule<'a>),
    KeyframeBlock(&'a KeyframeBlock<'a>),
}

pub struct LintContext<'a> {
    pub source: &'a str,
    pub stylesheet: &'a Stylesheet<'a>,
    pub comments: &'a [Comment<'a>],
    pub line_index: &'a LineIndex,
    pub disables: &'a DisableDirectives,
    pub ancestors: Vec<AstParent<'a>>,
    pub current_severity: Severity,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> LintContext<'a> {
    pub fn new(
        source: &'a str,
        stylesheet: &'a Stylesheet<'a>,
        comments: &'a [Comment<'a>],
        line_index: &'a LineIndex,
        disables: &'a DisableDirectives,
    ) -> Self {
        Self {
            source,
            stylesheet,
            comments,
            line_index,
            disables,
            ancestors: Vec::with_capacity(16),
            current_severity: Severity::Error,
            diagnostics: Vec::new(),
        }
    }

    #[inline]
    pub fn slice(&self, span: Span) -> &'a str {
        let start = (span.start as usize).min(self.source.len());
        let end = (span.end as usize).min(self.source.len());
        if start <= end {
            &self.source[start..end]
        } else {
            ""
        }
    }

    #[inline]
    pub fn slice_range(&self, start: u32, end: u32) -> &'a str {
        self.slice(Span { start, end })
    }

    pub fn comments_in_range(&self, start: u32, end: u32) -> &'a [Comment<'a>] {
        let start_idx = self.comments.partition_point(|c| c.span.start < start);
        let end_idx = self.comments.partition_point(|c| c.span.end <= end);
        if start_idx < end_idx {
            &self.comments[start_idx..end_idx]
        } else {
            &[]
        }
    }

    pub fn in_keyframes(&self) -> bool {
        self.ancestors.iter().any(|p| match p {
            AstParent::AtRule(at_rule) => {
                crate::utils::unprefixed(at_rule.name.name).eq_ignore_ascii_case("keyframes")
            }
            AstParent::KeyframeBlock(_) => true,
            _ => false,
        })
    }

    pub fn report(&mut self, rule: &'static str, span: Span, message: impl Into<String>) {
        let (line, _) = self.line_index.line_col(self.source, span.start);
        if self.disables.is_suppressed(rule, line) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule,
            message: message.into(),
            span,
            severity: self.current_severity,
            fix: None,
        });
    }

    pub fn report_with_fix(
        &mut self,
        rule: &'static str,
        span: Span,
        message: impl Into<String>,
        fix: Fix,
    ) {
        let (line, _) = self.line_index.line_col(self.source, span.start);
        if self.disables.is_suppressed(rule, line) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule,
            message: message.into(),
            span,
            severity: self.current_severity,
            fix: Some(fix),
        });
    }

    pub fn take_fixes(&mut self) -> Vec<Fix> {
        self.diagnostics
            .iter_mut()
            .filter_map(|d| d.fix.take())
            .collect()
    }

    pub fn into_warnings(self, quiet: bool) -> Vec<StylelintWarning> {
        let mut warnings = Vec::with_capacity(self.diagnostics.len());
        for d in self.diagnostics {
            if quiet && d.severity == Severity::Warning {
                continue;
            }
            let (line, column) = self.line_index.line_col(self.source, d.span.start);
            let end_offset = if d.span.end > d.span.start {
                d.span.end
            } else {
                d.span.start + 1
            };
            let (end_line, end_column) = self.line_index.line_col(self.source, end_offset);
            let rule_str = d.rule;
            let has_suffix = d
                .message
                .strip_suffix(')')
                .and_then(|s| s.strip_suffix(rule_str))
                .is_some_and(|s| s.ends_with('('));
            let text = if has_suffix {
                d.message
            } else {
                format!("{} ({})", d.message, rule_str)
            };
            warnings.push(StylelintWarning {
                line,
                column,
                end_line,
                end_column,
                rule: rule_str,
                severity: d.severity.as_str(),
                text,
            });
        }
        warnings
    }
}
