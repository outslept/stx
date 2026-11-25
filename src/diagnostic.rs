use oxc_css_parser::Span;
use serde::Serialize;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    #[default]
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Edit {
    pub span: Span,
    pub replacement: String,
}

#[derive(Debug, Clone)]
pub struct Fix {
    pub edits: Vec<Edit>,
}

impl Fix {
    pub fn single(span: Span, replacement: impl Into<String>) -> Self {
        Self {
            edits: vec![Edit {
                span,
                replacement: replacement.into(),
            }],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub rule: &'static str,
    pub severity: Severity,
    pub span: Span,
    pub message: String,
    pub fix: Option<Fix>,
}

#[derive(Debug, Serialize)]
pub struct StylelintWarning {
    pub line: usize,
    pub column: usize,
    #[serde(rename = "endLine")]
    pub end_line: usize,
    #[serde(rename = "endColumn")]
    pub end_column: usize,
    pub rule: &'static str,
    pub severity: &'static str,
    pub text: String,
    #[serde(skip)]
    pub fix: Option<Fix>,
}

#[derive(Debug, Serialize)]
pub struct StylelintResult<'a> {
    pub source: &'a str,
    pub errored: bool,
    pub warnings: Vec<StylelintWarning>,
}
