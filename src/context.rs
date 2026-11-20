use crate::{
    diagnostic::{Diagnostic, Severity, StylelintWarning},
    disable::DisableDirectives,
    fix::Fix,
    line_index::LineIndex,
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, Declaration, KeyframeBlock, QualifiedRule, SimpleBlock, Stylesheet},
    token::Comment,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum AstParent<'a> {
    QualifiedRule(&'a QualifiedRule<'a>),
    AtRule(&'a AtRule<'a>),
    KeyframeBlock(&'a KeyframeBlock<'a>),
    SimpleBlock(&'a SimpleBlock<'a>),
    Declaration(&'a Declaration<'a>),
}

#[allow(dead_code)]
pub struct LintContext<'a> {
    pub source: &'a str,
    pub stylesheet: &'a Stylesheet<'a>,
    pub comments: &'a [Comment<'a>],
    pub line_index: &'a LineIndex,
    pub disables: &'a DisableDirectives,
    pub ancestry: Vec<AstParent<'a>>,
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
            ancestry: Vec::with_capacity(32),
            current_severity: Severity::Error,
            diagnostics: Vec::new(),
        }
    }

    #[inline]
    pub fn slice(&self, span: Span) -> &'a str {
        &self.source[span.start as usize..span.end as usize]
    }

    pub fn in_keyframes(&self) -> bool {
        self.ancestry.iter().rev().any(|parent| match parent {
            AstParent::KeyframeBlock(_) => true,
            AstParent::AtRule(at_rule) => {
                let name = at_rule.name.name;
                name.eq_ignore_ascii_case("keyframes")
                    || name.eq_ignore_ascii_case("-webkit-keyframes")
                    || name.eq_ignore_ascii_case("-moz-keyframes")
                    || name.eq_ignore_ascii_case("-o-keyframes")
            }
            _ => false,
        })
    }

    pub fn report(&mut self, rule: &'static str, span: Span, message: String) {
        self.report_with_fix(rule, span, message, None);
    }

    pub fn report_with_fix(
        &mut self,
        rule: &'static str,
        span: Span,
        message: String,
        fix: Option<Fix>,
    ) {
        let (line, _) = self.line_index.line_col(self.source, span.start);
        if self.disables.is_suppressed(rule, line) {
            return;
        }

        self.diagnostics.push(Diagnostic {
            rule,
            severity: self.current_severity,
            span,
            message,
            fix,
        });
    }

    pub fn take_fixes(&mut self) -> Vec<Fix> {
        self.diagnostics
            .iter_mut()
            .filter_map(|d| d.fix.take())
            .collect()
    }

    pub fn into_warnings(self, quiet: bool) -> Vec<StylelintWarning> {
        let src_len = self.source.len() as u32;
        let mut warnings = Vec::with_capacity(self.diagnostics.len());
        for d in self.diagnostics {
            if quiet && d.severity == Severity::Warning {
                continue;
            }
            let (line, column) = self.line_index.line_col(self.source, d.span.start);
            let (end_line, end_column) = if d.span.end > src_len {
                let (el, ec) = self.line_index.line_col(self.source, src_len);
                (el, ec + (d.span.end - src_len) as usize)
            } else {
                self.line_index.line_col(self.source, d.span.end)
            };
            warnings.push(StylelintWarning {
                line,
                column,
                end_line,
                end_column,
                rule: d.rule,
                severity: d.severity.as_str(),
                text: format!("{} ({})", d.message, d.rule),
            });
        }
        warnings
    }
}
