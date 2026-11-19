use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, value_utils::declaration_value_span},
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, Declaration, InterpolableIdent},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlQuoteMode {
    Always,
    Never,
}

pub struct FunctionUrlQuotes {
    pub mode: UrlQuoteMode,
}

impl Default for FunctionUrlQuotes {
    fn default() -> Self {
        Self {
            mode: UrlQuoteMode::Always,
        }
    }
}

impl FunctionUrlQuotes {
    fn scan_span_for_urls(&self, span: Span, ctx: &mut LintContext<'_>) {
        let start = span.start as usize;
        let end = (span.end as usize).min(ctx.source.len());
        if start >= end {
            return;
        }
        let bytes = ctx.source.as_bytes();
        let mut i = start;

        while i < end {
            if bytes[i] == b'/' && i + 1 < end && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < end && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(end);
                continue;
            }

            if bytes[i] == b'"' || bytes[i] == b'\'' {
                let quote = bytes[i];
                i += 1;
                while i < end {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(end);
                        continue;
                    }
                    if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }

            if i + 4 <= end
                && ctx.source[i..i + 4].eq_ignore_ascii_case("url(")
                && (i == start || !is_ident_char(bytes[i - 1]))
            {
                let arg_open = i + 4;
                let mut j = arg_open;
                let mut in_quote: Option<u8> = None;
                let mut close_paren = None;

                while j < end {
                    if let Some(q) = in_quote {
                        if bytes[j] == b'\\' {
                            j = (j + 2).min(end);
                            continue;
                        }
                        if bytes[j] == q {
                            in_quote = None;
                        }
                        j += 1;
                        continue;
                    }
                    if bytes[j] == b'"' || bytes[j] == b'\'' {
                        in_quote = Some(bytes[j]);
                        j += 1;
                        continue;
                    }
                    if bytes[j] == b')' {
                        close_paren = Some(j);
                        break;
                    }
                    j += 1;
                }

                if let Some(close_idx) = close_paren {
                    self.check_url_arg(arg_open, close_idx, ctx);
                    i = close_idx + 1;
                    continue;
                }
            }

            i += 1;
        }
    }

    fn check_url_arg(&self, arg_open: usize, close_idx: usize, ctx: &mut LintContext<'_>) {
        let raw_arg = &ctx.source[arg_open..close_idx];
        let trimmed = raw_arg.trim();
        if trimmed.is_empty() || trimmed.starts_with('$') || trimmed.starts_with('@') {
            return;
        }

        let trim_offset = raw_arg.find(trimmed).unwrap_or(0);
        let arg_start = (arg_open + trim_offset) as u32;
        let arg_end = arg_start + trimmed.len() as u32;
        let arg_span = Span {
            start: arg_start,
            end: arg_end,
        };

        let is_quoted = (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''));

        match self.mode {
            UrlQuoteMode::Always => {
                if !is_quoted {
                    ctx.report_with_fix(
                        self.name(),
                        arg_span,
                        "Expected quotes around \"url\" function argument".to_string(),
                        Some(Fix::single(arg_span, format!("\"{}\"", trimmed))),
                    );
                }
            }
            UrlQuoteMode::Never => {
                if is_quoted && trimmed.len() >= 2 {
                    let inner = &trimmed[1..trimmed.len() - 1];
                    if !inner.contains(char::is_whitespace) && !inner.contains(')') {
                        ctx.report_with_fix(
                            self.name(),
                            arg_span,
                            "Unexpected quotes around \"url\" function argument".to_string(),
                            Some(Fix::single(arg_span, inner.to_string())),
                        );
                    }
                }
            }
        }
    }
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

impl Rule for FunctionUrlQuotes {
    fn name(&self) -> &'static str {
        "function-url-quotes"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        if let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        {
            self.scan_span_for_urls(val_span, ctx);
        }
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if let Some(prelude) = &at_rule.prelude {
            self.scan_span_for_urls(prelude.span, ctx);
        }
    }
}
