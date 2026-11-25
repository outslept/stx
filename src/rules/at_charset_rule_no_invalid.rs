//! Rule: `at-charset-rule-no-invalid`
//!
//! Disallow invalid `@charset` rules.

use std::borrow::Cow;

use oxc_css_parser::{Span, ast::Stylesheet};

use crate::{context::LintContext, rules::Rule};

const REASON_BYTE_ORDER_MARK: &str = "expected no byte order mark before it";
const REASON_POSITION: &str = "expected it to be at the very start of the stylesheet";
const REASON_NAME: &str = "expected it to be lowercase";
const REASON_SPACE: &str = "expected a single space after it";
const REASON_QUOTES: &str = "expected double quotes around the encoding label";
const REASON_SEMICOLON: &str = "expected a semicolon directly after the closing quote";

fn invalid_at_charset_diagnostic(reason: &str) -> String {
    format!("Invalid \"@charset\" rule, {reason}")
}

crate::define_stx_lint! {
    /// Check the placement and spelling of `@charset` declarations.
    ///
    /// Enable with `true`; there are no rule-specific secondary options. If present,
    /// `@charset` must start the file with no leading BOM, whitespace, or comment.
    /// It must be lowercase, followed by one ASCII space, a double-quoted label,
    /// and an immediate semicolon. The rule checks the form, not the encoding name.
    ///
    /// Accepted at the start of a file:
    ///
    /// ```css
    /// @charset "utf-8";
    /// ```
    "at-charset-rule-no-invalid", correctness;
    #[derive(Debug, Default, Clone, Copy)]
    pub struct AtCharsetRuleNoInvalid;
}

impl Rule for AtCharsetRuleNoInvalid {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        if !ctx.source.contains('@') {
            return;
        }

        let css = ctx.source;
        let has_bom = ctx.has_bom;

        let tokens = tokenize_css(css);
        let mut idx = 0;
        let mut at_stmt_start = true;
        let mut bracket_depth: usize = 0;

        while idx < tokens.len() {
            let tok = tokens[idx];
            match tok.kind {
                TokenKind::Space | TokenKind::Comment => {
                    idx += 1;
                }
                TokenKind::OpenParen | TokenKind::OpenBracket => {
                    bracket_depth += 1;
                    at_stmt_start = false;
                    idx += 1;
                }
                TokenKind::CloseParen | TokenKind::CloseBracket => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                    at_stmt_start = false;
                    idx += 1;
                }
                TokenKind::Semicolon | TokenKind::OpenBrace | TokenKind::CloseBrace
                    if bracket_depth == 0 =>
                {
                    at_stmt_start = true;
                    idx += 1;
                }
                TokenKind::AtWord if at_stmt_start && bracket_depth == 0 => {
                    let name = &tok.raw[1..];
                    if !name.eq_ignore_ascii_case("charset") {
                        at_stmt_start = false;
                        idx += 1;
                        continue;
                    }

                    let rule_start = tok.start;
                    let mut rule_end = tok.end;
                    let mut inner_brackets: Vec<TokenKind> = Vec::new();
                    let mut open_block = false;
                    let mut ended_without_semi = true;
                    let mut j = idx + 1;
                    let mut params_end = j;

                    while j < tokens.len() {
                        let t = tokens[j];
                        j += 1;
                        match t.kind {
                            TokenKind::OpenParen => inner_brackets.push(TokenKind::CloseParen),
                            TokenKind::OpenBracket => inner_brackets.push(TokenKind::CloseBracket),
                            TokenKind::CloseParen | TokenKind::CloseBracket
                                if inner_brackets.last() == Some(&t.kind) =>
                            {
                                inner_brackets.pop();
                            }
                            _ => {}
                        }
                        if inner_brackets.is_empty() {
                            match t.kind {
                                TokenKind::Semicolon => {
                                    ended_without_semi = false;
                                    rule_end = t.end;
                                    break;
                                }
                                TokenKind::OpenBrace => {
                                    ended_without_semi = false;
                                    open_block = true;
                                    break;
                                }
                                TokenKind::CloseBrace => break,
                                _ => params_end = j,
                            }
                        } else {
                            params_end = j;
                        }
                    }

                    // If `@charset` is followed by a `{ ... }` block, find the closing `}`
                    // so the reported span covers the entire at-rule node.
                    if open_block {
                        let mut brace_depth = 1usize;
                        rule_end = css.len();
                        for t in &tokens[j..] {
                            match t.kind {
                                TokenKind::OpenBrace => brace_depth += 1,
                                TokenKind::CloseBrace => {
                                    brace_depth -= 1;
                                    if brace_depth == 0 {
                                        rule_end = t.end;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    let mut params = &tokens[idx + 1..params_end];
                    let mut has_between = false;
                    while let Some((last, rest)) = params.split_last() {
                        if !matches!(last.kind, TokenKind::Space | TokenKind::Comment) {
                            break;
                        }
                        has_between = true;
                        params = rest;
                    }

                    let mut after_name = "";
                    if let Some(last) = params.last() {
                        if ended_without_semi {
                            rule_end = last.end;
                            has_between = false;
                        }
                        let after_name_start = tok.end;
                        let mut after_name_end = tok.end;
                        while let Some((first, rest)) = params.split_first() {
                            if !matches!(first.kind, TokenKind::Space | TokenKind::Comment) {
                                break;
                            }
                            after_name_end = first.end;
                            params = rest;
                        }
                        after_name = &css[after_name_start..after_name_end];
                    } else if ended_without_semi {
                        rule_end = tok.end;
                        has_between = false;
                    }

                    let params_str: Cow<'_, str> =
                        if params.iter().any(|p| p.kind == TokenKind::Comment) {
                            params
                                .iter()
                                .filter(|p| p.kind != TokenKind::Comment)
                                .map(|p| p.raw)
                                .collect::<String>()
                                .into()
                        } else if let Some((first, last)) = params.first().zip(params.last()) {
                            Cow::Borrowed(&css[first.start..last.end])
                        } else {
                            Cow::Borrowed("")
                        };

                    let span = Span {
                        start: rule_start as u32,
                        end: rule_end as u32,
                    };

                    let mut report_reason = |reason: &str| {
                        ctx.report(Self::NAME, span, invalid_at_charset_diagnostic(reason));
                    };

                    if has_bom {
                        report_reason(REASON_BYTE_ORDER_MARK);
                    }
                    if rule_start != 0 {
                        report_reason(REASON_POSITION);
                    }
                    if name != "charset" {
                        report_reason(REASON_NAME);
                    }
                    if after_name != " " {
                        report_reason(REASON_SPACE);
                    }

                    let label = match_encoding_label(&params_str);
                    if label.is_none() {
                        report_reason(REASON_QUOTES);
                    }

                    let has_extra_content_in_prelude =
                        label.is_some_and(|l| l.len() != params_str.len());

                    if has_between
                        || has_extra_content_in_prelude
                        || open_block
                        || ended_without_semi
                    {
                        report_reason(REASON_SEMICOLON);
                    }

                    at_stmt_start = true;
                    idx = j;
                }
                _ => {
                    at_stmt_start = false;
                    idx += 1;
                }
            }
        }
    }
}

/// Matches `/^"[^"]*"/` at the beginning of `params`.
fn match_encoding_label(params: &str) -> Option<&str> {
    let rest = params.strip_prefix('"')?;
    let closing_quote = rest.find('"')?;
    Some(&params[..closing_quote + 2])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    AtWord,
    Space,
    Comment,
    Semicolon,
    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    Other,
}

#[derive(Debug, Clone, Copy)]
struct CssToken<'a> {
    kind: TokenKind,
    raw: &'a str,
    start: usize,
    end: usize,
}

/// Tokenizes `css` following PostCSS's tokenizer rules.
fn tokenize_css(css: &str) -> Vec<CssToken<'_>> {
    let bytes = css.as_bytes();
    let len = bytes.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < len {
        let start = i;
        let kind = match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' => {
                i += 1;
                while i < len && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'\x0C') {
                    i += 1;
                }
                TokenKind::Space
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                TokenKind::Comment
            }
            b'"' | b'\'' => {
                let quote = bytes[i];
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(len);
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                TokenKind::Other
            }
            b';' => {
                i += 1;
                TokenKind::Semicolon
            }
            b'{' => {
                i += 1;
                TokenKind::OpenBrace
            }
            b'}' => {
                i += 1;
                TokenKind::CloseBrace
            }
            b'(' => {
                i += 1;
                TokenKind::OpenParen
            }
            b')' => {
                i += 1;
                TokenKind::CloseParen
            }
            b'[' => {
                i += 1;
                TokenKind::OpenBracket
            }
            b']' => {
                i += 1;
                TokenKind::CloseBracket
            }
            b'@' => {
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(len);
                    } else if matches!(
                        bytes[i],
                        b' ' | b'\t'
                            | b'\n'
                            | b'\r'
                            | b'\x0C'
                            | b'"'
                            | b'#'
                            | b'\''
                            | b'('
                            | b')'
                            | b'/'
                            | b';'
                            | b'['
                            | b']'
                            | b'{'
                            | b'}'
                    ) {
                        break;
                    } else {
                        i += 1;
                    }
                }
                if i > start + 1 {
                    TokenKind::AtWord
                } else {
                    TokenKind::Other
                }
            }
            _ => {
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(len);
                    } else if matches!(
                        bytes[i],
                        b' ' | b'\t'
                            | b'\n'
                            | b'\r'
                            | b'\x0C'
                            | b'"'
                            | b'\''
                            | b'('
                            | b')'
                            | b';'
                            | b'['
                            | b']'
                            | b'{'
                            | b'}'
                    ) || (bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*')
                    {
                        break;
                    } else {
                        i += 1;
                    }
                }
                TokenKind::Other
            }
        };

        tokens.push(CssToken {
            kind,
            raw: &css[start..i],
            start,
            end: i,
        });
    }

    tokens
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AtCharsetRuleNoInvalid, invalid_at_charset_diagnostic as rejected};
    use crate::tester::{Tester, reject, reject_warnings};

    #[test]
    fn default_config() {
        let pass = &[
            "@charset \"utf-8\";",
            "@charset \"UTF-8\";",
            "@charset \"iso-8859-15\";",
            "@charset \"utf-8\";\na {}",
        ];

        let fail = &[
            reject(
                "\u{FEFF}@charset \"utf-8\";",
                None,
                rejected("expected no byte order mark before it"),
                (1, 1, 1, 18),
            ),
            reject(
                " @charset \"utf-8\";",
                None,
                rejected("expected it to be at the very start of the stylesheet"),
                (1, 2, 1, 19),
            ),
            reject(
                "\n@charset \"utf-8\";",
                None,
                rejected("expected it to be at the very start of the stylesheet"),
                (2, 1, 2, 18),
            ),
            reject(
                "a {}\n@charset \"utf-8\";",
                None,
                rejected("expected it to be at the very start of the stylesheet"),
                (2, 1, 2, 18),
            ),
            reject(
                "/* comment */\n@charset \"utf-8\";",
                None,
                rejected("expected it to be at the very start of the stylesheet"),
                (2, 1, 2, 18),
            ),
            reject(
                "@CHARSET \"utf-8\";",
                None,
                rejected("expected it to be lowercase"),
                (1, 1, 1, 18),
            ),
            reject(
                "@Charset \"utf-8\";",
                None,
                rejected("expected it to be lowercase"),
                (1, 1, 1, 18),
            ),
            reject(
                "@charset  \"utf-8\";",
                None,
                rejected("expected a single space after it"),
                (1, 1, 1, 19),
            ),
            reject(
                "@charset\t\"utf-8\";",
                None,
                rejected("expected a single space after it"),
                (1, 1, 1, 18),
            ),
            reject(
                "@charset\n\"utf-8\";",
                None,
                rejected("expected a single space after it"),
                (1, 1, 2, 9),
            ),
            reject(
                "@charset\"utf-8\";",
                None,
                rejected("expected a single space after it"),
                (1, 1, 1, 17),
            ),
            reject(
                "@charset 'utf-8';",
                None,
                rejected("expected double quotes around the encoding label"),
                (1, 1, 1, 18),
            ),
            reject(
                "@charset utf-8;",
                None,
                rejected("expected double quotes around the encoding label"),
                (1, 1, 1, 16),
            ),
            reject(
                "@charset \"utf-8\" ;",
                None,
                rejected("expected a semicolon directly after the closing quote"),
                (1, 1, 1, 19),
            ),
            reject(
                "@charset \"utf-8\"",
                None,
                rejected("expected a semicolon directly after the closing quote"),
                (1, 1, 1, 17),
            ),
            reject(
                "@charset \"utf-8\" {}",
                None,
                rejected("expected a semicolon directly after the closing quote"),
                (1, 1, 1, 20),
            ),
            reject(
                "@charset \"utf-8\" extra;",
                None,
                rejected("expected a semicolon directly after the closing quote"),
                (1, 1, 1, 24),
            ),
            reject_warnings(
                "@charset;",
                None,
                vec![
                    (rejected("expected a single space after it"), (1, 1, 1, 10)),
                    (
                        rejected("expected double quotes around the encoding label"),
                        (1, 1, 1, 10),
                    ),
                ],
            ),
            reject_warnings(
                " @CHARSET\t'utf-8' ;",
                None,
                vec![
                    (
                        rejected("expected it to be at the very start of the stylesheet"),
                        (1, 2, 1, 20),
                    ),
                    (rejected("expected it to be lowercase"), (1, 2, 1, 20)),
                    (rejected("expected a single space after it"), (1, 2, 1, 20)),
                    (
                        rejected("expected double quotes around the encoding label"),
                        (1, 2, 1, 20),
                    ),
                    (
                        rejected("expected a semicolon directly after the closing quote"),
                        (1, 2, 1, 20),
                    ),
                ],
            ),
        ];

        Tester::new(AtCharsetRuleNoInvalid::NAME, json!([true]), pass, fail).test();
    }
}
