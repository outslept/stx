use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        value_utils::{ValueToken, ValueTokenKind, declaration_value_span, tokenize_value},
    },
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamilyQuoteMode {
    AlwaysWhereRecommended,
    AlwaysUnlessKeyword,
    AlwaysWhereRequired,
}

pub struct FontFamilyNameQuotes {
    pub mode: FontFamilyQuoteMode,
}

impl Default for FontFamilyNameQuotes {
    fn default() -> Self {
        Self {
            mode: FontFamilyQuoteMode::AlwaysWhereRecommended,
        }
    }
}

const FONT_FAMILY_KEYWORDS: &[&str] = &[
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
    "math",
    "emoji",
    "fangsong",
    "inherit",
    "initial",
    "unset",
    "revert",
    "revert-layer",
];

fn is_keyword(s: &str) -> bool {
    FONT_FAMILY_KEYWORDS
        .iter()
        .any(|k| k.eq_ignore_ascii_case(s))
}

fn is_recommended_unquoted(s: &str) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| b.is_ascii_alphabetic() || b == b'-')
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
}

fn is_valid_unquoted_ident_sequence(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    for part in s.split_whitespace() {
        let bytes = part.as_bytes();
        if bytes.is_empty() || bytes[0].is_ascii_digit() {
            return false;
        }
        if bytes[0] == b'-' && (bytes.len() == 1 || bytes[1].is_ascii_digit() || bytes[1] == b'-') {
            return false;
        }
        if !bytes
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return false;
        }
    }
    true
}

impl FontFamilyNameQuotes {
    fn check_family_group(&self, group: &[ValueToken<'_>], ctx: &mut LintContext<'_>) {
        if group.is_empty() {
            return;
        }
        if group.iter().any(|t| {
            t.kind == ValueTokenKind::Function
                || t.raw.starts_with('$')
                || t.raw.starts_with('@')
                || t.raw.starts_with("--")
        }) {
            return;
        }

        if group.len() == 1 && group[0].kind == ValueTokenKind::String {
            let tok = &group[0];
            if tok.raw.len() < 2 {
                return;
            }
            let inner = &tok.raw[1..tok.raw.len() - 1];

            if is_keyword(inner) {
                ctx.report_with_fix(
                    self.name(),
                    tok.span,
                    format!("Unexpected quotes around \"{}\"", inner),
                    Some(Fix::single(tok.span, inner.to_string())),
                );
                return;
            }

            let should_unquote = match self.mode {
                FontFamilyQuoteMode::AlwaysWhereRecommended => is_recommended_unquoted(inner),
                FontFamilyQuoteMode::AlwaysUnlessKeyword => false,
                FontFamilyQuoteMode::AlwaysWhereRequired => is_valid_unquoted_ident_sequence(inner),
            };

            if should_unquote {
                ctx.report_with_fix(
                    self.name(),
                    tok.span,
                    format!("Unexpected quotes around \"{}\"", inner),
                    Some(Fix::single(tok.span, inner.to_string())),
                );
            }
            return;
        }

        let first_span = group.first().unwrap().span;
        let last_span = group.last().unwrap().span;
        let full_span = Span {
            start: first_span.start,
            end: last_span.end,
        };
        let raw_family = ctx.slice(full_span).trim();

        if is_keyword(raw_family) {
            return;
        }

        let needs_quotes = match self.mode {
            FontFamilyQuoteMode::AlwaysWhereRecommended => !is_recommended_unquoted(raw_family),
            FontFamilyQuoteMode::AlwaysUnlessKeyword => true,
            FontFamilyQuoteMode::AlwaysWhereRequired => {
                !is_valid_unquoted_ident_sequence(raw_family)
            }
        };

        if needs_quotes {
            let expected = format!("\"{}\"", raw_family);
            ctx.report_with_fix(
                self.name(),
                full_span,
                format!("Expected quotes around \"{}\"", raw_family),
                Some(Fix::single(full_span, expected)),
            );
        }
    }
}

impl Rule for FontFamilyNameQuotes {
    fn name(&self) -> &'static str {
        "font-family-name-quotes"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if !prop.eq_ignore_ascii_case("font-family") {
            return;
        }

        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };

        let tokens = tokenize_value(ctx.source, val_span);
        let mut current = Vec::new();
        let mut depth = 0usize;

        for tok in tokens {
            match tok.kind {
                ValueTokenKind::Function => {
                    depth += 1;
                    current.push(tok);
                }
                ValueTokenKind::CloseParen => {
                    depth = depth.saturating_sub(1);
                    current.push(tok);
                }
                ValueTokenKind::Comma if depth == 0 => {
                    self.check_family_group(&current, ctx);
                    current.clear();
                }
                _ => current.push(tok),
            }
        }
        self.check_family_group(&current, ctx);
    }
}
