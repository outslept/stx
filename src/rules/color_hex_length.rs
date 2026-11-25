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

crate::define_stx_lint! {
    /// Require short or long notation for hex colors without changing their value.
    ///
    /// Use `"short"` to collapse repeated digit pairs into three or four digits
    /// where possible, or `"long"` to expand them to six or eight digits. There
    /// are no secondary options. Invalid hex colors and `url()` contents are
    /// skipped. Autofix rewrites the digits, retaining any alpha channel.
    ///
    /// With `"short"`:
    /// ```css
    /// a { color: #aabbcc; } /* reported */
    /// a { color: #abc; }    /* accepted */
    /// a { color: #123456; } /* accepted: cannot be shortened */
    /// ```
    "color-hex-length", style, fix = fix;
    pub struct ColorHexLength {
        pub mode: HexLengthMode,
    }
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
        Self::NAME
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
    ctx.source.as_bytes()[pos as usize]
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject, reject_message};

    const NAME: &str = "color-hex-length";

    fn expected(actual: &str, expected: &str) -> String {
        format!("Expected \"{actual}\" to be \"{expected}\"")
    }

    #[test]
    fn short() {
        let pass = &[
            "a { color: pink; }",
            "a { color: #000; }",
            "a { color: #FFF; }",
            "a { color: #f00; }",
            "a { color: #0000; }",
            "a { color: #FFFF; }",
            "a { color: #f000; }",
            "a { something: #000, #fff, #aba; }",
            "a { color: #ff; }",
            "a { color: #ffffffa; }",
            "a { color: #fffffffffff; }",
            "a { padding: 000000; }",
            "a::before { content: \"#ABABAB\"; }",
            "a { color: white /* #FFFFFF */; }",
            "a { background: url(somefile.swvg#abcdef)}",
        ];

        let fail = &[
            reject(
                "a { color: #FFFFFF; }",
                "a { color: #FFF; }",
                expected("#FFFFFF", "#FFF"),
                (1, 12, 1, 19),
            )
            .with_edit_info(0, (15, 18, "")),
            reject(
                "a { color: #FfaAFF; }",
                "a { color: #FaF; }",
                expected("#FfaAFF", "#FaF"),
                (1, 12, 1, 19),
            )
            .with_edit_info(0, (13, 17, "a")),
            reject(
                "a { color: #00aa00aa; }",
                "a { color: #0a0a; }",
                expected("#00aa00aa", "#0a0a"),
                (1, 12, 1, 21),
            )
            .with_edit_info(0, (13, 19, "a0")),
            reject(
                "a { something: #fff, #aba, #00ffAAaa; }",
                "a { something: #fff, #aba, #0fAa; }",
                expected("#00ffAAaa", "#0fAa"),
                (1, 28, 1, 37),
            )
            .with_edit_info(0, (29, 35, "fA")),
        ];

        Tester::new(NAME, json!(["short"]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn long() {
        let pass = &[
            "a { color: pink; }",
            "a { color: #000000; }",
            "a { color: #FFFFFF; }",
            "a { color: #f00000; }",
            "a { color: #00000000; }",
            "a { color: #FFFFFFFF; }",
            "a { color: #f0000000; }",
            "a { something: #000000, #ffffff, #ababab; }",
            "a { color: #ff; }",
            "a { color: #ffffffa; }",
            "a { color: #fffffffffff; }",
            "a { padding: 000; }",
            "a::before { content: \"#ABA\"; }",
            "a { color: white /* #FFF */; }",
            "a { background: url(somefile.swvg#abc)}",
            "a { background: url('data:image/svg+xml;utf8,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><path fill=\"#000\" d=\"M32 2a24 24 0 0 0-24 24c0 16.2 21.6 34.8 22.5 35.5a2.4 2.4 0 0 0 3 0C34.4 60.8 56 42.2 56 26A24 24 0 0 0 32 2zm0 32a8 8 0 1 1 8-8 8 8 0 0 1-8 8z\"/></svg>'); }",
        ];

        let fail = &[
            reject(
                "a { color: #FFF; }",
                "a { color: #FFFFFF; }",
                expected("#FFF", "#FFFFFF"),
                (1, 12, 1, 16),
            )
            .with_edit_info(0, (14, 15, "FFFF")),
            reject(
                "a { color: #Ffa; }",
                "a { color: #FFffaa; }",
                expected("#Ffa", "#FFffaa"),
                (1, 12, 1, 16),
            )
            .with_edit_info(0, (13, 14, "Fffa")),
            reject(
                "a { color: #0a0a; }",
                "a { color: #00aa00aa; }",
                expected("#0a0a", "#00aa00aa"),
                (1, 12, 1, 17),
            )
            .with_edit_info(0, (13, 15, "0aa00a")),
            reject(
                "a { something: #000000, #ffffff, #aba; }",
                "a { something: #000000, #ffffff, #aabbaa; }",
                expected("#aba", "#aabbaa"),
                (1, 34, 1, 38),
            )
            .with_edit_info(0, (35, 36, "abba")),
            reject(
                "a { background: url('data:image/svg+xml;utf8,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><path fill=\"#000\" d=\"M32 2a24 24 0 0 0-24 24c0 16.2 21.6 34.8 22.5 35.5a2.4 2.4 0 0 0 3 0C34.4 60.8 56 42.2 56 26A24 24 0 0 0 32 2zm0 32a8 8 0 1 1 8-8 8 8 0 0 1-8 8z\"/></svg>') #fff; }",
                "a { background: url('data:image/svg+xml;utf8,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><path fill=\"#000\" d=\"M32 2a24 24 0 0 0-24 24c0 16.2 21.6 34.8 22.5 35.5a2.4 2.4 0 0 0 3 0C34.4 60.8 56 42.2 56 26A24 24 0 0 0 32 2zm0 32a8 8 0 1 1 8-8 8 8 0 0 1-8 8z\"/></svg>') #ffffff; }",
                expected("#fff", "#ffffff"),
                (1, 283, 1, 287),
            )
            .with_edit_info(0, (285, 286, "ffff")),
        ];

        Tester::new(NAME, json!(["long"]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    #[ignore = "naive-css-in-js is not supported"]
    fn short_css_in_js() {
        let pass = &["export default <svg style={{ color: \"#fff\" }} />;"];
        let fail = &[reject_message(
            "export default <svg style={{ color: \"#ffffff\" }} />;",
            "export default <svg style={{ color: \"#fff\" }} />;",
            expected("#ffffff", "#fff"),
        )];

        Tester::new(NAME, json!(["short"]), pass, fail)
            .with_custom_syntax("naive-css-in-js")
            .with_fix(true)
            .test();
    }
}
