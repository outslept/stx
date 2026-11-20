//! Rule `color-hex-length`: specify short or long notation for hex colors.
//!
//! Reference: <https://stylelint.io/user-guide/rules/color-hex-length>

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::Value;

use crate::context::{Fix, LintContext};
use crate::rules::Rule;

/// Primary option mode for [`ColorHexLength`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexLengthMode {
    /// Prefer 3- or 4-digit hex colors (`#rgb`, `#rgba`) where possible.
    Short,
    /// Prefer 6- or 8-digit hex colors (`#rrggbb`, `#rrggbbaa`).
    Long,
}

/// [`color-hex-length`](https://stylelint.io/user-guide/rules/color-hex-length)
///
/// Specify short or long notation for hex colors.
pub struct ColorHexLength {
    pub mode: HexLengthMode,
}

impl Default for ColorHexLength {
    fn default() -> Self {
        Self {
            mode: HexLengthMode::Short,
        }
    }
}

impl ColorHexLength {
    /// Creates a new `color-hex-length` rule instance from the primary option.
    pub fn from_primary(primary: &Value) -> Option<Self> {
        let mode = match primary.as_str()? {
            "short" => HexLengthMode::Short,
            "long" => HexLengthMode::Long,
            _ => return None,
        };
        Some(Self { mode })
    }

    #[inline]
    fn expected_message(actual: &str, expected: &str) -> String {
        format!("Expected \"{actual}\" to be \"{expected}\"")
    }
}

impl Rule for ColorHexLength {
    fn name(&self) -> &'static str {
        "color-hex-length"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let mut pos: u32 = 0;
        let mut paren_depth: u32 = 0;
        let mut colon_pos: Option<u32> = None;

        while pos < len {
            let b = byte_at(ctx, pos);

            // Skip block comments `/* ... */`.
            if b == b'/' && pos + 1 < len && byte_at(ctx, pos + 1) == b'*' {
                pos = skip_block_comment(ctx, pos, len);
                continue;
            }

            // Skip single-line comments `// ...` when not part of a URL scheme (`://`).
            if b == b'/'
                && pos + 1 < len
                && byte_at(ctx, pos + 1) == b'/'
                && (pos == 0 || byte_at(ctx, pos - 1) != b':')
            {
                pos = skip_line_comment(ctx, pos, len);
                continue;
            }

            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                pos = skip_string(ctx, pos, len);
                continue;
            }

            if b == b'(' {
                paren_depth += 1;
                pos += 1;
                continue;
            }

            if b == b')' {
                paren_depth = paren_depth.saturating_sub(1);
                pos += 1;
                continue;
            }

            if paren_depth == 0 {
                if b == b':' && colon_pos.is_none() {
                    colon_pos = Some(pos);
                    pos += 1;
                    continue;
                }

                // Opening brace `{` means any preceding `:` was a pseudo-class in a selector,
                // unless it is a `#{...}` SCSS interpolation inside a declaration value.
                if b == b'{' {
                    if colon_pos.is_some()
                        && pos > 0
                        && byte_at(ctx, pos - 1) == b'#'
                        && let Some(close_brace) = find_matching_brace(ctx, pos, len)
                    {
                        pos = close_brace + 1;
                        continue;
                    }
                    colon_pos = None;
                    pos += 1;
                    continue;
                }

                // End of declaration (`;` or `}`).
                if b == b';' || b == b'}' {
                    if let Some(colon) = colon_pos {
                        self.check_value_range(ctx, colon + 1, pos);
                    }
                    colon_pos = None;
                    pos += 1;
                    continue;
                }
            }

            pos += 1;
        }

        // Check trailing declaration without a semicolon at EOF.
        if let Some(colon) = colon_pos {
            self.check_value_range(ctx, colon + 1, len);
        }
    }
}

impl ColorHexLength {
    /// Scans a declaration value range `val_start..val_end` for hex color tokens.
    fn check_value_range<'a>(&self, ctx: &mut LintContext<'a>, val_start: u32, val_end: u32) {
        let mut i = val_start;
        while i < val_end {
            let b = byte_at(ctx, i);

            // Skip block comments.
            if b == b'/' && i + 1 < val_end && byte_at(ctx, i + 1) == b'*' {
                i = skip_block_comment(ctx, i, val_end);
                continue;
            }

            // Skip single-line comments.
            if b == b'/'
                && i + 1 < val_end
                && byte_at(ctx, i + 1) == b'/'
                && (i == val_start || byte_at(ctx, i - 1) != b':')
            {
                i = skip_line_comment(ctx, i, val_end);
                continue;
            }

            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                i = skip_string(ctx, i, val_end);
                continue;
            }

            // Skip `url(...)` functions (`isUrlFunction(node)`).
            if is_ident_start(b) {
                let ident_start = i;
                let mut ident_end = i + 1;
                while ident_end < val_end && is_ident_char(byte_at(ctx, ident_end)) {
                    ident_end += 1;
                }
                if ident_end < val_end
                    && byte_at(ctx, ident_end) == b'('
                    && ctx
                        .slice_range(ident_start, ident_end)
                        .eq_ignore_ascii_case("url")
                {
                    if let Some(close_paren) = find_matching_paren(ctx, ident_end, val_end) {
                        i = close_paren + 1;
                    } else {
                        i = val_end;
                    }
                    continue;
                }
                i = ident_end;
                continue;
            }

            // Check `#...` tokens.
            if b == b'#' {
                // Skip SCSS interpolation `#{...}`.
                if i + 1 < val_end && byte_at(ctx, i + 1) == b'{' {
                    if let Some(close_brace) = find_matching_brace(ctx, i + 1, val_end) {
                        i = close_brace + 1;
                    } else {
                        i = val_end;
                    }
                    continue;
                }

                let hex_start = i;
                let mut hex_end = i + 1;
                while hex_end < val_end && is_word_char(byte_at(ctx, hex_end)) {
                    hex_end += 1;
                }

                let hex_value = ctx.slice_range(hex_start, hex_end);
                if is_hex_color(hex_value) {
                    match self.mode {
                        HexLengthMode::Long => {
                            if hex_value.len() == 4 || hex_value.len() == 5 {
                                let expected_hex = longer(hex_value);
                                let span = Span {
                                    start: hex_start,
                                    end: hex_end,
                                };
                                ctx.report_with_fix(
                                    self.name(),
                                    span,
                                    Self::expected_message(hex_value, &expected_hex),
                                    Some(Fix::single(span, expected_hex)),
                                );
                            }
                        }
                        HexLengthMode::Short => {
                            if hex_value.len() >= 6 && can_shrink(hex_value) {
                                let expected_hex = shorter(hex_value);
                                let span = Span {
                                    start: hex_start,
                                    end: hex_end,
                                };
                                ctx.report_with_fix(
                                    self.name(),
                                    span,
                                    Self::expected_message(hex_value, &expected_hex),
                                    Some(Fix::single(span, expected_hex)),
                                );
                            }
                        }
                    }
                }

                i = hex_end;
                continue;
            }

            i += 1;
        }
    }
}

/// Returns `true` if `word` is a valid CSS hex color (`#` followed by 3, 4, 6, or 8 hex digits).
#[inline]
fn is_hex_color(word: &str) -> bool {
    let bytes = word.as_bytes();
    matches!(bytes.len(), 4 | 5 | 7 | 9)
        && bytes[0] == b'#'
        && bytes[1..].iter().copied().all(|b| b.is_ascii_hexdigit())
}

/// Returns `true` if a 6- or 8-digit hex color can be shortened to 3 or 4 digits.
#[inline]
fn can_shrink(hex: &str) -> bool {
    let b = hex.as_bytes();
    (b.len() == 7 || b.len() == 9)
        && b[1].eq_ignore_ascii_case(&b[2])
        && b[3].eq_ignore_ascii_case(&b[4])
        && b[5].eq_ignore_ascii_case(&b[6])
        && (b.len() == 7 || b[7].eq_ignore_ascii_case(&b[8]))
}

/// Converts a 6- or 8-digit hex color into its 3- or 4-digit form.
fn shorter(hex: &str) -> String {
    let b = hex.as_bytes();
    let mut out = String::with_capacity(1 + (b.len() - 1) / 2);
    out.push('#');
    let mut i = 1;
    while i < b.len() {
        out.push(b[i] as char);
        i += 2;
    }
    out
}

/// Converts a 3- or 4-digit hex color into its 6- or 8-digit form.
fn longer(hex: &str) -> String {
    let b = hex.as_bytes();
    let mut out = String::with_capacity(1 + (b.len() - 1) * 2);
    out.push('#');
    for &ch in &b[1..] {
        out.push(ch as char);
        out.push(ch as char);
    }
    out
}

/// Finds the matching `)` for an opening `(` at `open_paren_pos`.
fn find_matching_paren(ctx: &LintContext<'_>, open_paren_pos: u32, len: u32) -> Option<u32> {
    let mut depth: u32 = 0;
    let mut pos = open_paren_pos;
    while pos < len {
        let b = byte_at(ctx, pos);
        if b == b'/' && pos + 1 < len && byte_at(ctx, pos + 1) == b'*' {
            pos = skip_block_comment(ctx, pos, len);
            continue;
        }
        if b == b'"' || b == b'\'' {
            pos = skip_string(ctx, pos, len);
            continue;
        }
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(pos);
            }
        }
        pos += 1;
    }
    None
}

/// Finds the matching `}` for an opening `{` at `open_brace_pos`.
fn find_matching_brace(ctx: &LintContext<'_>, open_brace_pos: u32, len: u32) -> Option<u32> {
    let mut depth: u32 = 0;
    let mut pos = open_brace_pos;
    while pos < len {
        let b = byte_at(ctx, pos);
        if b == b'"' || b == b'\'' {
            pos = skip_string(ctx, pos, len);
            continue;
        }
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(pos);
            }
        }
        pos += 1;
    }
    None
}

/// Skips a `/* ... */` block comment starting at `pos`.
fn skip_block_comment(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    pos += 2;
    while pos + 1 < len && !(byte_at(ctx, pos) == b'*' && byte_at(ctx, pos + 1) == b'/') {
        pos += 1;
    }
    if pos + 1 < len { pos + 2 } else { len }
}

/// Skips a `// ...` line comment starting at `pos`.
fn skip_line_comment(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    pos += 2;
    while pos < len && byte_at(ctx, pos) != b'\n' {
        pos += 1;
    }
    pos
}

/// Skips a quoted string literal starting at `pos`.
fn skip_string(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    let quote = byte_at(ctx, pos);
    pos += 1;
    while pos < len {
        let cur = byte_at(ctx, pos);
        if cur == b'\\' {
            pos = (pos + 2).min(len);
            continue;
        }
        if cur == quote {
            return pos + 1;
        }
        pos += 1;
    }
    len
}

/// Returns the byte at `pos` in the source string.
#[inline]
fn byte_at(ctx: &LintContext<'_>, pos: u32) -> u8 {
    ctx.slice_range(pos, pos + 1).as_bytes()[0]
}

/// Returns `true` if `b` can start a CSS identifier.
#[inline]
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-' || b >= 0x80
}

/// Returns `true` if `b` can continue a CSS identifier.
#[inline]
fn is_ident_char(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

/// Returns `true` if `b` is part of a `postcss-value-parser` word token.
#[inline]
fn is_word_char(b: u8) -> bool {
    !b.is_ascii_whitespace()
        && !matches!(
            b,
            b',' | b'/' | b'(' | b')' | b'{' | b'}' | b';' | b':' | b'"' | b'\''
        )
}
