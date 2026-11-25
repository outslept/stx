use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, function_url_quotes::UrlQuoteMode},
    utils::{SelectorItemKind, scan_selector_items},
};
use oxc_css_parser::{Span, ast::SelectorList};

pub struct SelectorAttributeQuotes {
    pub mode: UrlQuoteMode,
}

impl Default for SelectorAttributeQuotes {
    fn default() -> Self {
        Self {
            mode: UrlQuoteMode::Always,
        }
    }
}

impl Rule for SelectorAttributeQuotes {
    fn name(&self) -> &'static str {
        "selector-attribute-quotes"
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        let items = scan_selector_items(ctx.source, list.span);
        for item in items {
            let SelectorItemKind::Attribute { inner_span } = item.kind else {
                continue;
            };
            let raw_inner = ctx.slice(inner_span);
            let Some(eq_pos) = raw_inner.find('=') else {
                continue;
            };
            let after_eq = &raw_inner[eq_pos + 1..];
            let trimmed_after = after_eq.trim();
            if trimmed_after.is_empty() {
                continue;
            }

            let val_part = if let Some((v, flag)) = trimmed_after.rsplit_once(char::is_whitespace) {
                if flag.eq_ignore_ascii_case("i") || flag.eq_ignore_ascii_case("s") {
                    v.trim()
                } else {
                    trimmed_after
                }
            } else {
                trimmed_after
            };

            if val_part.is_empty() {
                continue;
            }

            let rel_offset = after_eq.find(val_part).unwrap_or(0);
            let val_start = inner_span.start + (eq_pos + 1 + rel_offset) as u32;
            let val_end = val_start + val_part.len() as u32;
            let val_span = Span {
                start: val_start,
                end: val_end,
            };

            let is_quoted = (val_part.starts_with('"') && val_part.ends_with('"'))
                || (val_part.starts_with('\'') && val_part.ends_with('\''));

            match self.mode {
                UrlQuoteMode::Always => {
                    if !is_quoted {
                        ctx.report_with_fix(
                            self.name(),
                            val_span,
                            format!("Expected quotes around \"{}\"", val_part),
                            Some(Fix::single(val_span, format!("\"{}\"", val_part))),
                        );
                    }
                }
                UrlQuoteMode::Never => {
                    if is_quoted && val_part.len() >= 2 {
                        let unquoted = &val_part[1..val_part.len() - 1];
                        if !unquoted.is_empty()
                            && unquoted
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                        {
                            ctx.report_with_fix(
                                self.name(),
                                val_span,
                                format!("Expected no quotes around \"{}\"", unquoted),
                                Some(Fix::single(val_span, unquoted.to_string())),
                            );
                        }
                    }
                }
            }
        }
    }
}
