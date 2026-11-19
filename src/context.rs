use crate::{
    diagnostic::{Diagnostic, Severity, StylelintWarning},
    line_index::LineIndex,
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, Declaration, KeyframeBlock, QualifiedRule, SimpleBlock, Stylesheet},
    token::Comment,
};

#[derive(Debug, Clone, Copy)]
pub enum AstParent<'a> {
    QualifiedRule(&'a QualifiedRule<'a>),
    AtRule(&'a AtRule<'a>),
    KeyframeBlock(&'a KeyframeBlock<'a>),
    SimpleBlock(&'a SimpleBlock<'a>),
    Declaration(&'a Declaration<'a>),
}

pub struct LintContext<'a> {
    pub source: &'a str,
    pub stylesheet: &'a Stylesheet<'a>,
    pub comments: &'a [Comment<'a>],
    pub line_index: &'a LineIndex,
    pub ancestry: Vec<AstParent<'a>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> LintContext<'a> {
    pub fn new(
        source: &'a str,
        stylesheet: &'a Stylesheet<'a>,
        comments: &'a [Comment<'a>],
        line_index: &'a LineIndex,
    ) -> Self {
        Self {
            source,
            stylesheet,
            comments,
            line_index,
            ancestry: Vec::with_capacity(32),
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
        self.diagnostics.push(Diagnostic {
            rule,
            severity: Severity::Error,
            span,
            message,
        });
    }

    pub fn into_warnings(self) -> Vec<StylelintWarning> {
        let mut warnings: Vec<StylelintWarning> = self
            .diagnostics
            .into_iter()
            .map(|d| {
                let (line, column) = self.line_index.line_col(self.source, d.span.start);
                let (end_line, end_column) = self.line_index.line_col(self.source, d.span.end);
                StylelintWarning {
                    line,
                    column,
                    end_line,
                    end_column,
                    rule: d.rule,
                    severity: d.severity.as_str(),
                    text: format!("{} ({})", d.message, d.rule),
                }
            })
            .collect();

        warnings.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.column.cmp(&b.column)));
        warnings
    }
}
