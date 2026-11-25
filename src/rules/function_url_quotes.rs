use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{at_rule_prelude_span, declaration_value_span, secondary_has_flag},
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
    except_empty: bool,
}

impl FunctionUrlQuotes {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        Self {
            mode: if primary.as_str() == Some("never") {
                UrlQuoteMode::Never
            } else {
                UrlQuoteMode::Always
            },
            except_empty: secondary_has_flag(secondary, "except", "empty"),
        }
    }
}

impl Default for FunctionUrlQuotes {
    fn default() -> Self {
        Self {
            mode: UrlQuoteMode::Always,
            except_empty: false,
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
        let is_quoted = (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''));
        if raw_arg.to_ascii_lowercase().contains("crossorigin(")
            || (!is_quoted
                && (raw_arg.contains('$') || raw_arg.contains('@') || raw_arg.contains("#{")))
        {
            return;
        }

        let is_empty_exception = self.except_empty && matches!(trimmed, "" | "''" | "\"\"");
        let expect_quotes = matches!(self.mode, UrlQuoteMode::Always) != is_empty_exception;
        if expect_quotes == is_quoted {
            return;
        }
        if !expect_quotes && is_quoted && trimmed.len() >= 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            if requires_url_quotes(inner) {
                return;
            }
        }

        let trim_offset = raw_arg.find(trimmed).unwrap_or(raw_arg.len());
        let arg_start = (arg_open + trim_offset) as u32;
        let arg_span = Span {
            start: arg_start,
            end: close_idx as u32,
        };
        let trailing = &raw_arg[trim_offset + trimmed.len()..];
        let replacement = if expect_quotes {
            format!("\"{trimmed}\"{trailing}")
        } else {
            format!("{}{}", &trimmed[1..trimmed.len() - 1], trailing)
        };
        ctx.report_with_fix(
            self.name(),
            arg_span,
            if expect_quotes {
                "Expected quotes around \"url\" function argument"
            } else {
                "Expected no quotes around \"url\" function argument"
            },
            Some(Fix::single(arg_span, replacement)),
        );
    }
}

fn requires_url_quotes(value: &str) -> bool {
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch.is_whitespace() || matches!(ch, '(' | ')' | '\'' | '"') {
            return true;
        }
    }
    false
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
        if let Some(mut val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        {
            let bytes = ctx.source.as_bytes();
            let mut close = val_span.end as usize;
            while close < bytes.len() && bytes[close].is_ascii_whitespace() {
                close += 1;
            }
            if bytes.get(close) == Some(&b')') {
                while bytes.get(close) == Some(&b')') {
                    close += 1;
                }
                val_span.end = close as u32;
            }
            self.scan_span_for_urls(val_span, ctx);
        }
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) {
            self.scan_span_for_urls(p_span, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn preserves_whitespace_and_ignores_url_modifiers_and_interpolation() {
        let fail = [reject(
            "a { background-image: url( foo.css ); }",
            None,
            "Expected quotes around \"url\" function argument",
            (1, 28, 1, 36),
        )];
        Tester::new(
            "function-url-quotes",
            json!(["always"]),
            [
                "a { background: url('foo.svg' crossorigin(anonymous)); }",
                "a { background: url(\"foo.ttf\" + $variable); }",
            ],
            &fail,
        )
        .test();
    }

    #[test]
    fn supports_empty_exception() {
        let fail = [reject(
            "@import url('');",
            None,
            "Expected no quotes around \"url\" function argument",
            (1, 13, 1, 15),
        )];
        Tester::new(
            "function-url-quotes",
            json!(["always", {"except": ["empty"]}]),
            [],
            &fail,
        )
        .test();
    }

    #[test]
    fn import_url_with_inner_whitespace() {
        let fail = [reject(
            "@import url( foo.css );",
            "@import url( \"foo.css\" );",
            "Expected quotes around \"url\" function argument",
            (1, 14, 1, 22),
        )];
        Tester::new("function-url-quotes", json!(["always"]), [], &fail)
            .with_fix(true)
            .test();
    }
}
