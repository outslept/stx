//! Rule `color-hex-alpha`: require or disallow alpha channel for hex colors.
//!
//! Reference: <https://stylelint.io/user-guide/rules/color-hex-alpha>

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::Value;

use crate::context::LintContext;
use crate::rules::Rule;

/// Primary option mode for [`ColorHexAlpha`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Require an alpha channel in all hex colors (`#rgba` or `#rrggbbaa`).
    Always,
    /// Disallow an alpha channel in all hex colors (`#rgb` or `#rrggbb`).
    Never,
}

crate::define_stx_lint! {
    /// Require or forbid an explicit alpha channel in hex colors.
    ///
    /// Use `"always"` for four or eight digits and `"never"` for three or six
    /// digits. There are no secondary options. The rule checks valid hex
    /// colors, not malformed hashes, URL fragments, or Sass interpolation.
    ///
    /// With `"always"`, even fully opaque colors need an alpha digit:
    /// ```css
    /// a { color: #123; }  /* reported */
    /// a { color: #123f; } /* accepted */
    /// ```
    "color-hex-alpha", style;
    pub struct ColorHexAlpha {
        mode: Mode,
    }
}

impl Default for ColorHexAlpha {
    fn default() -> Self {
        Self { mode: Mode::Never }
    }
}

impl ColorHexAlpha {
    /// Creates a new `color-hex-alpha` rule instance from the primary option.
    ///
    /// Returns `None` if the primary option is neither `"always"` nor `"never"`.
    pub fn from_primary(primary: &Value) -> Option<Self> {
        let mode = match primary.as_str()? {
            "always" => Mode::Always,
            "never" => Mode::Never,
            _ => return None,
        };
        Some(Self { mode })
    }

    /// Formats the diagnostic message when `"always"` is configured and a hex color lacks alpha.
    #[inline]
    fn expected_message(hex: &str) -> String {
        format!("Expected alpha channel in \"{hex}\"")
    }

    /// Formats the diagnostic message when `"never"` is configured and a hex color has alpha.
    #[inline]
    fn rejected_message(hex: &str) -> String {
        format!("Expected no alpha channel in \"{hex}\"")
    }
}

impl Rule for ColorHexAlpha {
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
        // Check trailing declaration without a semicolon at EOF (e.g. `$var: #fff`).
        if let Some(colon) = colon_pos {
            self.check_value_range(ctx, colon + 1, len);
        }
    }
}

impl ColorHexAlpha {
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
                let word = ctx.slice_range(hex_start, hex_end);
                if is_hex_color(word) {
                    let has_alpha = has_alpha_channel(word);
                    match self.mode {
                        Mode::Always if !has_alpha => {
                            ctx.report(
                                self.name(),
                                Span {
                                    start: hex_start,
                                    end: hex_end,
                                },
                                Self::expected_message(word),
                            );
                        }
                        Mode::Never if has_alpha => {
                            ctx.report(
                                self.name(),
                                Span {
                                    start: hex_start,
                                    end: hex_end,
                                },
                                Self::rejected_message(word),
                            );
                        }
                        _ => {}
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

/// Returns `true` if `hex` has an alpha channel (4-digit `#rgba` or 8-digit `#rrggbbaa`).
#[inline]
fn has_alpha_channel(hex: &str) -> bool {
    hex.len() == 5 || hex.len() == 9
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject, reject_warnings};

    const NAME: &str = "color-hex-alpha";

    fn expected(hex: &str) -> String {
        format!("Expected alpha channel in \"{hex}\"")
    }

    #[test]
    fn always() {
        let pass = &[
            "a { color: #ffff; }",
            "a { color: #ffffffff; }",
            "a { background: linear-gradient(to left, #fffa, #000000aa 100%); }",
            "a { background: url(#fff); }",
        ];

        let fail = &[
            reject("a { color: #fff; }", None, expected("#fff"), (1, 12, 1, 16)),
            reject(
                "a { color: #ffffff; }",
                None,
                expected("#ffffff"),
                (1, 12, 1, 19),
            ),
            reject_warnings(
                "a { background: linear-gradient(to left, #fff, #000000 100%); }",
                None,
                vec![
                    (expected("#fff"), (1, 42, 1, 46)),
                    (expected("#000000"), (1, 48, 1, 55)),
                ],
            ),
        ];

        Tester::new(NAME, json!(["always"]), pass, fail).test();
    }

    #[test]
    fn never() {
        let pass = &["a { color: #fff; }", "a { color: #ffffff; }"];
        let fail = &[reject(
            "a { color: #ffff; }",
            None,
            "Expected no alpha channel in \"#ffff\"",
            (1, 12, 1, 17),
        )];

        Tester::new(NAME, json!(["never"]), pass, fail).test();
    }

    #[test]
    fn always_scss() {
        let pass = &["a { color: #{f}; }", "$var: #ffff;"];
        let fail = &[reject("$var: #fff", None, expected("#fff"), (1, 7, 1, 11))];

        Tester::new(NAME, json!(["always"]), pass, fail)
            .with_custom_syntax("postcss-scss")
            .test();
    }
}
