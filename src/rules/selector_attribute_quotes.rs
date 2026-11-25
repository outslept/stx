use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, function_url_quotes::UrlQuoteMode},
    utils::{SelectorItemKind, scan_selector_items},
};
use oxc_css_parser::{Span, ast::SelectorList};

crate::define_stx_lint! {
    /// Enforces whether attribute selector values use quotes.
    ///
    /// Set to `"always"` to require single or double quotes, or `"never"` to unquote
    /// values that are valid CSS identifiers. Presence selectors are not checked.
    /// Auto-fix adds double quotes or removes existing quotes as appropriate.
    ///
    /// Example: `"always"` changes `[href=\'test\'] {}` to `[href="'test'"] {}`.
    "selector-attribute-quotes", style, fix = fix;
    pub struct SelectorAttributeQuotes {
        pub mode: UrlQuoteMode,
    }
}

impl Default for SelectorAttributeQuotes {
    fn default() -> Self {
        Self {
            mode: UrlQuoteMode::Always,
        }
    }
}

// shortcut: only simple `\x` escapes; values with hex escapes are kept raw.
fn unescape(raw: &str) -> std::borrow::Cow<'_, str> {
    if !raw.contains('\\') || has_hex_escape(raw) {
        return raw.into();
    }
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        out.push(if c == '\\' {
            chars.next().unwrap_or(c)
        } else {
            c
        });
    }
    out.into()
}

fn has_hex_escape(raw: &str) -> bool {
    raw.as_bytes()
        .windows(2)
        .any(|w| w[0] == b'\\' && w[1].is_ascii_hexdigit())
}

impl Rule for SelectorAttributeQuotes {
    fn name(&self) -> &'static str {
        Self::NAME
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
                        let value = unescape(val_part);
                        let quoted = match &value {
                            std::borrow::Cow::Owned(v) => {
                                v.replace('\\', "\\\\").replace('"', "\\\"")
                            }
                            std::borrow::Cow::Borrowed(v) => v.to_string(),
                        };
                        ctx.report_with_fix(
                            self.name(),
                            val_span,
                            format!("Expected quotes around \"{value}\""),
                            Some(Fix::single(val_span, format!("\"{quoted}\""))),
                        );
                    }
                }
                UrlQuoteMode::Never => {
                    if is_quoted && val_part.len() >= 2 {
                        let unquoted = &val_part[1..val_part.len() - 1];
                        if crate::utils::is_valid_identifier(unquoted) {
                            ctx.report_with_fix(
                                self.name(),
                                val_span,
                                format!("Expected no quotes around \"{}\"", unescape(unquoted)),
                                Some(Fix::single(val_span, unquoted.to_string())),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn always_unescapes_values_and_requotes_them() {
        let fail = [
            reject(
                r#"[href=te\'s\"t] { }"#,
                r#"[href="te's\"t"] { }"#,
                r#"Expected quotes around "te's"t""#,
                (1, 7, 1, 15),
            ),
            reject(
                r"[href=\'test\'] { }",
                r#"[href="'test'"] { }"#,
                r#"Expected quotes around "'test'""#,
                (1, 7, 1, 15),
            ),
        ];
        Tester::new("selector-attribute-quotes", json!("always"), [], fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn never_keeps_escapes_in_the_fix() {
        let fail = [reject(
            r#"[href="te\"s\"t"] { }"#,
            r#"[href=te\"s\"t] { }"#,
            r#"Expected no quotes around "te"s"t""#,
            (1, 7, 1, 17),
        )];
        Tester::new("selector-attribute-quotes", json!("never"), [], fail)
            .with_fix(true)
            .test();
    }
}
