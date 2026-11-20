//! Rule: `at-charset-rule-no-invalid`
//!
//! Disallow invalid `@charset` rules.

use oxc_css_parser::{Span, ast::Stylesheet};

use crate::context::LintContext;
use crate::rules::Rule;

const REASON_BYTE_ORDER_MARK: &str = "expected no byte order mark before it";
const REASON_POSITION: &str = "expected it to be at the very start of the stylesheet";
const REASON_NAME: &str = "expected it to be lowercase";
const REASON_SPACE: &str = "expected a single space after it";
const REASON_QUOTES: &str = "expected double quotes around the encoding label";
const REASON_SEMICOLON: &str = "expected a semicolon directly after the closing quote";

/// ## `at-charset-rule-no-invalid`
///
/// Disallow invalid `@charset` rules according to the CSS Syntax specification.
///
/// ### What it does
///
/// Validates that every `@charset` rule in a stylesheet strictly conforms to the byte-stream
/// requirements defined by the CSS Syntax Module Level 3 specification:
///
/// 1. No UTF-8 Byte Order Mark (`\u{FEFF}`) precedes the rule.
/// 2. The rule starts at the very first character of the stylesheet (no leading whitespace,
///    comments, or preceding statements).
/// 3. The at-rule name is lowercase (`@charset`).
/// 4. Exactly one ASCII space (`' '`) follows `@charset`.
/// 5. The encoding label is wrapped in double quotes (`"..."`).
/// 6. A semicolon (`;`) immediately follows the closing double quote (with no intervening
///    whitespace, extra tokens, or block `{}`).
///
/// ### Why is this bad?
///
/// Unlike normal CSS at-rules, `@charset` is parsed by browsers using a strict byte-sequence
/// check (`@charset "<encoding>";`) at the very beginning of the stylesheet. Any deviation in
/// casing, spacing, quoting, or placement causes the browser to completely ignore the `@charset`
/// declaration.
///
/// ### Options
///
/// #### Primary
///
/// - `true`: Enable the rule.
///
/// ```json
/// {
///   "at-charset-rule-no-invalid": true
/// }
/// ```
///
/// ### Examples
///
/// **Incorrect:**
/// ```css
/// @CHARSET "utf-8";
/// ```
/// ```css
/// @charset 'utf-8';
/// ```
/// ```css
/// @charset  "utf-8";
/// ```
/// ```css
/// /* comment */
/// @charset "utf-8";
/// ```
///
/// **Correct:**
/// ```css
/// @charset "utf-8";
/// ```
///
/// ### References
///
/// - [Stylelint Documentation: `at-charset-rule-no-invalid`](https://stylelint.io/user-guide/rules/at-charset-rule-no-invalid)
/// - [CSS Syntax Module Level 3: Determining the fallback encoding](https://www.w3.org/TR/css-syntax-3/#determine-the-fallback-encoding)
#[derive(Default)]
pub struct AtCharsetRuleNoInvalid;

impl Rule for AtCharsetRuleNoInvalid {
    fn name(&self) -> &'static str {
        "at-charset-rule-no-invalid"
    }

    fn check_stylesheet<'a>(&self, _stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        // Fast path: skip stylesheets that do not contain `@`.
        if !ctx.source.contains('@') {
            return;
        }

        // PostCSS strips a leading UTF-8 BOM (`\u{FEFF}`, 3 bytes) from the input stream
        // and records `input.hasBOM = true`.
        let (has_bom, css) = match ctx.source.strip_prefix('\u{FEFF}') {
            Some(stripped) => (true, stripped),
            None => (false, ctx.source),
        };

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
                    let mut params: Vec<CssToken<'_>> = Vec::new();
                    let mut inner_brackets: Vec<TokenKind> = Vec::new();
                    let mut open_block = false;
                    let mut ended_without_semi = false;

                    let mut j = idx + 1;
                    while j < tokens.len() {
                        let t = tokens[j];
                        j += 1;

                        match t.kind {
                            TokenKind::OpenParen => inner_brackets.push(TokenKind::CloseParen),
                            TokenKind::OpenBracket => inner_brackets.push(TokenKind::CloseBracket),
                            TokenKind::CloseParen | TokenKind::CloseBracket => {
                                if inner_brackets.last() == Some(&t.kind) {
                                    inner_brackets.pop();
                                }
                            }
                            _ => {}
                        }

                        if inner_brackets.is_empty() {
                            if t.kind == TokenKind::Semicolon {
                                rule_end = t.end;
                                break;
                            } else if t.kind == TokenKind::OpenBrace {
                                open_block = true;
                                break;
                            } else if t.kind == TokenKind::CloseBrace {
                                ended_without_semi = true;
                                break;
                            } else {
                                params.push(t);
                            }
                        } else {
                            params.push(t);
                        }

                        if j == tokens.len() {
                            ended_without_semi = true;
                            break;
                        }
                    }

                    if idx + 1 == tokens.len() {
                        ended_without_semi = true;
                    }

                    // If `@charset` is followed by a `{ ... }` block, find the closing `}`
                    // so the reported span covers the entire at-rule node.
                    if open_block {
                        let mut brace_depth = 1usize;
                        let mut k = j;
                        rule_end = css.len();
                        while k < tokens.len() {
                            match tokens[k].kind {
                                TokenKind::OpenBrace => brace_depth += 1,
                                TokenKind::CloseBrace => {
                                    brace_depth -= 1;
                                    if brace_depth == 0 {
                                        rule_end = tokens[k].end;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            k += 1;
                        }
                    }

                    // Extract trailing spaces/comments into `between` (matching PostCSS `raws.between`).
                    let mut between = String::new();
                    while let Some(last) = params.last() {
                        if last.kind != TokenKind::Space && last.kind != TokenKind::Comment {
                            break;
                        }
                        let popped = params.pop().unwrap();
                        between.insert_str(0, popped.raw);
                    }

                    let mut after_name = String::new();
                    let mut params_str = String::new();

                    if !params.is_empty() {
                        if ended_without_semi {
                            rule_end = params.last().unwrap().end;
                            between.clear();
                        }
                        while let Some(first) = params.first() {
                            if first.kind != TokenKind::Space && first.kind != TokenKind::Comment {
                                break;
                            }
                            let shifted = params.remove(0);
                            after_name.push_str(shifted.raw);
                        }
                        for p in &params {
                            if p.kind != TokenKind::Comment {
                                params_str.push_str(p.raw);
                            }
                        }
                    } else if ended_without_semi {
                        rule_end = tok.end;
                        between.clear();
                    }

                    let span = Span {
                        start: css_offset_to_source_offset(css, rule_start, has_bom),
                        end: css_offset_to_source_offset(css, rule_end, has_bom),
                    };

                    let mut report_reason = |reason: &str| {
                        ctx.report(
                            self.name(),
                            span,
                            format!("Invalid \"@charset\" rule, {reason}"),
                        );
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
                    if !between.is_empty()
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

/// Converts a byte offset in `css` (after stripping any leading `\u{FEFF}` BOM)
/// into a byte offset in `ctx.source` that produces identical `(line, column)`
/// coordinates to PostCSS.
fn css_offset_to_source_offset(css: &str, css_offset: usize, has_bom: bool) -> u32 {
    if !has_bom {
        return css_offset as u32;
    }
    // On line 1, PostCSS column numbers exclude the 1-character BOM (`\u{FEFF}`, 3 bytes).
    if !css[..css_offset].contains('\n') {
        if css_offset == 0 {
            0
        } else {
            let prev_char_idx = css[..css_offset]
                .char_indices()
                .next_back()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            (3 + prev_char_idx) as u32
        }
    } else {
        (3 + css_offset) as u32
    }
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
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' => {
                i += 1;
                while i < len && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'\x0C') {
                    i += 1;
                }
                tokens.push(CssToken {
                    kind: TokenKind::Space,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = if i + 1 < len { i + 2 } else { len };
                tokens.push(CssToken {
                    kind: TokenKind::Comment,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
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
                tokens.push(CssToken {
                    kind: TokenKind::Other,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
            }
            b';' => {
                i += 1;
                tokens.push(CssToken {
                    kind: TokenKind::Semicolon,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
            }
            b'{' => {
                i += 1;
                tokens.push(CssToken {
                    kind: TokenKind::OpenBrace,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
            }
            b'}' => {
                i += 1;
                tokens.push(CssToken {
                    kind: TokenKind::CloseBrace,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
            }
            b'(' => {
                i += 1;
                tokens.push(CssToken {
                    kind: TokenKind::OpenParen,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
            }
            b')' => {
                i += 1;
                tokens.push(CssToken {
                    kind: TokenKind::CloseParen,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
            }
            b'[' => {
                i += 1;
                tokens.push(CssToken {
                    kind: TokenKind::OpenBracket,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
            }
            b']' => {
                i += 1;
                tokens.push(CssToken {
                    kind: TokenKind::CloseBracket,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
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
                let kind = if i > start + 1 {
                    TokenKind::AtWord
                } else {
                    TokenKind::Other
                };
                tokens.push(CssToken {
                    kind,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
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
                tokens.push(CssToken {
                    kind: TokenKind::Other,
                    raw: &css[start..i],
                    start,
                    end: i,
                });
            }
        }
    }

    tokens
}
