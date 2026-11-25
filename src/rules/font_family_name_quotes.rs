use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{ValueToken, ValueTokenKind, declaration_value_span, font_size_end, tokenize_value},
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

crate::define_stx_lint! {
    /// Enforces a quoting convention for family names in `font-family` and `font`.
    ///
    /// Primary options:
    ///
    /// - `"always-unless-keyword"`: quote all family names other than keywords.
    /// - `"always-where-required"`: leave valid sequences of unquoted identifiers
    ///   unquoted, including multiword names such as `Times New Roman`.
    /// - `"always-where-recommended"`: also quote multiword names and names with
    ///   characters beyond ASCII letters and interior single hyphens.
    ///
    /// Keywords such as `serif` are unquoted in every mode. Auto-fix adds or
    /// removes quotes; violations in names split by comments are not fixed.
    ///
    /// ```css
    /// a { font-family: Times New Roman, serif; } /* quotes Times New Roman with "always-where-recommended" */
    /// ```
    "font-family-name-quotes", style, fix = conditional;
    pub struct FontFamilyNameQuotes {
        pub mode: FontFamilyQuoteMode,
    }
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
        || crate::utils::PREFIXED_SYSTEM_FONTS.contains(&s)
        || s == "BlinkMacSystemFont"
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
                    format!("Expected no quotes around \"{}\"", inner),
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
                    format!("Expected no quotes around \"{}\"", inner),
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

        if raw_family.contains("/*") {
            // Comments join words like spaces; such names are reported without a fix.
            let name = group
                .iter()
                .map(|t| match t.kind {
                    ValueTokenKind::String if t.raw.len() >= 2 => &t.raw[1..t.raw.len() - 1],
                    _ => t.raw,
                })
                .collect::<Vec<_>>()
                .join(" ");
            let message = if group[0].kind == ValueTokenKind::String {
                let unquote = is_keyword(&name)
                    || match self.mode {
                        FontFamilyQuoteMode::AlwaysWhereRecommended => {
                            is_recommended_unquoted(&name)
                        }
                        FontFamilyQuoteMode::AlwaysUnlessKeyword => false,
                        FontFamilyQuoteMode::AlwaysWhereRequired => {
                            is_valid_unquoted_ident_sequence(&name)
                        }
                    };
                unquote.then(|| format!("Expected no quotes around \"{name}\""))
            } else {
                let quote = !is_keyword(&name)
                    && match self.mode {
                        FontFamilyQuoteMode::AlwaysWhereRecommended => {
                            !is_recommended_unquoted(&name)
                        }
                        FontFamilyQuoteMode::AlwaysUnlessKeyword => true,
                        FontFamilyQuoteMode::AlwaysWhereRequired => {
                            !is_valid_unquoted_ident_sequence(&name)
                        }
                    };
                quote.then(|| format!("Expected quotes around \"{name}\""))
            };
            if let Some(message) = message {
                ctx.report(self.name(), first_span, message);
            }
            return;
        }

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
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        let is_shorthand = prop.eq_ignore_ascii_case("font");
        if !is_shorthand && !prop.eq_ignore_ascii_case("font-family") {
            return;
        }

        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(mut val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };
        if is_shorthand {
            let Some(offset) = font_size_end(ctx.slice(val_span)) else {
                return;
            };
            let mut start = val_span.start as usize + offset;
            while ctx
                .source
                .as_bytes()
                .get(start)
                .is_some_and(u8::is_ascii_whitespace)
            {
                start += 1;
            }
            val_span.start = start as u32;
        }

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

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;

    #[test]
    fn checks_font_shorthand_families() {
        let fail = [reject(
            "a { font: 1em Lucida Grande, Arial, sans-serif; }",
            None,
            "Expected quotes around \"Lucida Grande\"",
            (1, 15, 1, 28),
        )];
        Tester::new(
            "font-family-name-quotes",
            json!("always-where-recommended"),
            [],
            &fail,
        )
        .test();
    }

    #[test]
    fn leaves_system_font_names_unquoted() {
        let fail = [reject(
            "a { font-family: '-apple-system'; }",
            None,
            "Expected no quotes around \"-apple-system\"",
            (1, 18, 1, 33),
        )];
        Tester::new(
            "font-family-name-quotes",
            json!("always-unless-keyword"),
            [
                "a { font-family: -apple-system, BlinkMacSystemFont, sans-serif; }",
                "a { font: 1em -apple-system, BlinkMacSystemFont, sans-serif; }",
            ],
            &fail,
        )
        .test();
    }

    #[test]
    fn names_with_comments_are_reported_without_fix() {
        let fail = [
            reject_message(
                "a { font-family: Comic /* foo */ Sans, serif; }",
                None,
                "Expected quotes around \"Comic Sans\"",
            ),
            reject_message(
                "a { font-family: Comic/* foo */Sans, serif; }",
                None,
                "Expected quotes around \"Comic Sans\"",
            ),
        ];
        Tester::new(
            "font-family-name-quotes",
            json!("always-unless-keyword"),
            [],
            fail,
        )
        .with_fix(true)
        .test();
        let fail = [reject_message(
            "a { font-family: \"Comic\" /* foo */ Sans, serif; }",
            None,
            "Expected no quotes around \"Comic Sans\"",
        )];
        Tester::new(
            "font-family-name-quotes",
            json!("always-where-required"),
            [],
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn unquotes_prefixed_system_fonts() {
        let fail = [crate::tester::reject_messages(
            "a { font-family: '-webkit-control', '-moz-button'; }",
            "a { font-family: -webkit-control, -moz-button; }",
            vec![
                "Expected no quotes around \"-webkit-control\"".into(),
                "Expected no quotes around \"-moz-button\"".into(),
            ],
        )];
        Tester::new(
            "font-family-name-quotes",
            json!("always-unless-keyword"),
            [],
            fail,
        )
        .with_fix(true)
        .test();
    }
}
