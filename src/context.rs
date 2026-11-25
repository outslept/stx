pub use crate::fix::Fix;
use crate::{
    diagnostic::{Diagnostic, Severity, StylelintWarning},
    disable::DisableDirectives,
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
    pub has_bom: bool,
    pub stylesheet: &'a Stylesheet<'a>,
    pub comments: &'a [Comment<'a>],
    pub line_index: &'a LineIndex,
    pub disables: &'a DisableDirectives,
    pub ancestors: Vec<AstParent<'a>>,
    pub current_severity: Severity,
    current_custom_message: Option<std::sync::Arc<str>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> LintContext<'a> {
    pub fn new(
        source: &'a str,
        stylesheet: &'a Stylesheet<'a>,
        comments: &'a [Comment<'a>],
        line_index: &'a LineIndex,
        disables: &'a DisableDirectives,
        has_bom: bool,
    ) -> Self {
        Self {
            source,
            has_bom,
            stylesheet,
            comments,
            line_index,
            disables,
            ancestors: Vec::with_capacity(16),
            current_severity: Severity::Error,
            current_custom_message: None,
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

    pub fn set_rule(&mut self, rule: &crate::rules::ConfiguredRule) {
        self.current_severity = rule.severity;
        self.current_custom_message = rule.custom_message.clone();
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
            message: self
                .current_custom_message
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| message.into()),
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
        fix: impl Into<Option<Fix>>,
    ) {
        let (line, _) = self.line_index.line_col(self.source, span.start);
        if self.disables.is_suppressed(rule, line) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule,
            message: self
                .current_custom_message
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| message.into()),
            span,
            severity: self.current_severity,
            fix: fix.into(),
        });
    }

    pub fn fixes(&self) -> Vec<Fix> {
        self.diagnostics
            .iter()
            .filter_map(|d| d.fix.clone())
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
            let (end_line, end_column) = if end_offset as usize == self.source.len() + 1 {
                let (line, column) = self
                    .line_index
                    .line_col(self.source, self.source.len() as u32);
                (line, column + 1)
            } else {
                self.line_index.line_col(self.source, end_offset)
            };
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
                fix: d.fix,
            });
        }
        warnings
    }
}
