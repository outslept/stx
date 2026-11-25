//! Rule `color-function-alias-notation`: specify alias notation for color-functions
//! with an identical syntax.
//!
//! Reference: <https://stylelint.io/user-guide/rules/color-function-alias-notation>

use crate::context::LintContext;
use crate::fix::Fix;
use oxc_css_parser::{Span, ast::Declaration};
use serde_json::Value;

use crate::rules::Rule;
use crate::utils::ast::decl_value_span;
use crate::utils::reference::{
    WITH_ALPHA_ALIAS_COLOR_FUNCTIONS, WITHOUT_ALPHA_ALIAS_COLOR_FUNCTIONS, in_set_ignore_case,
};

/// Primary option mode for [`ColorFunctionAliasNotation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Require the `a`-suffixed alias (`rgba()`, `hsla()`).
    WithAlpha,
    /// Require the non-alias form without trailing `a` (`rgb()`, `hsl()`).
    WithoutAlpha,
}

crate::define_stx_lint! {
    /// Require consistent names for the `rgb()` and `hsl()` color function aliases.
    ///
    /// Set the primary option to `"with-alpha"` for `rgba()` and `hsla()` or
    /// `"without-alpha"` for `rgb()` and `hsl()`. There are no secondary options.
    /// The rule checks the name, not whether the arguments include alpha. It
    /// skips calls with a top-level hex color or Sass variable argument.
    /// Autofix adds or removes the suffix without changing arguments.
    ///
    /// With `"without-alpha"`:
    /// ```css
    /// a { color: rgba(0 0 0 / 50%); } /* reported */
    /// a { color: rgb(0 0 0 / 50%); }  /* accepted */
    /// ```
    "color-function-alias-notation", style, fix = fix;
    pub struct ColorFunctionAliasNotation {
        mode: Mode,
    }
}

impl Default for ColorFunctionAliasNotation {
    fn default() -> Self {
        Self {
            mode: Mode::WithoutAlpha,
        }
    }
}

impl ColorFunctionAliasNotation {
    /// Creates a new `color-function-alias-notation` rule instance from the primary option.
    ///
    /// Returns `None` if the primary option is neither `"with-alpha"` nor `"without-alpha"`.
    pub fn from_primary(primary: &Value) -> Option<Self> {
        let mode = match primary.as_str()? {
            "with-alpha" => Mode::WithAlpha,
            "without-alpha" => Mode::WithoutAlpha,
            _ => return None,
        };
        Some(Self { mode })
    }

    /// Formats the diagnostic message for a mismatched color function name.
    #[inline]
    fn expected_message(unfixed: &str, fixed: &str) -> String {
        format!("Expected \"{unfixed}\" to be \"{fixed}\"")
    }
}

impl Rule for ColorFunctionAliasNotation {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(val_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let start = val_span.start;
        let end = val_span.end;

        if start >= end {
            return;
        }

        let target_set = match self.mode {
            Mode::WithAlpha => WITHOUT_ALPHA_ALIAS_COLOR_FUNCTIONS,
            Mode::WithoutAlpha => WITH_ALPHA_ALIAS_COLOR_FUNCTIONS,
        };

        let mut i = start;
        while i < end {
            let b = byte_at(ctx, i);

            // Skip block comments `/* ... */`.
            if b == b'/' && i + 1 < end && byte_at(ctx, i + 1) == b'*' {
                i = skip_block_comment(ctx, i, end);
                continue;
            }

            // Skip single-line comments `// ...` when not part of a URL scheme (`://`).
            if b == b'/'
                && i + 1 < end
                && byte_at(ctx, i + 1) == b'/'
                && (i == start || byte_at(ctx, i - 1) != b':')
            {
                i = skip_line_comment(ctx, i, end);
                continue;
            }

            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                i = skip_string(ctx, i, end);
                continue;
            }

            // Scan CSS identifiers and check if followed by `(`.
            if is_ident_start(b) {
                let fn_start = i;
                let mut fn_end = i + 1;
                while fn_end < end && is_ident_char(byte_at(ctx, fn_end)) {
                    fn_end += 1;
                }

                // Check for a function call `<ident>(`.
                if fn_end < end && byte_at(ctx, fn_end) == b'(' {
                    let fn_name = ctx.slice_range(fn_start, fn_end);

                    if in_set_ignore_case(target_set, fn_name)
                        && is_standard_syntax_color_function(ctx, fn_end + 1, end)
                    {
                        let (fixed_name, fix) = match self.mode {
                            Mode::WithAlpha => {
                                let fixed = format!("{fn_name}a");
                                let last_char = ctx.slice_range(fn_end - 1, fn_end);
                                let fix = Fix::single(
                                    Span {
                                        start: fn_end - 1,
                                        end: fn_end,
                                    },
                                    format!("{last_char}a"),
                                );
                                (fixed, fix)
                            }
                            Mode::WithoutAlpha => {
                                let fixed = ctx.slice_range(fn_start, fn_end - 1).to_string();
                                let fix = Fix::single(
                                    Span {
                                        start: fn_end - 1,
                                        end: fn_end,
                                    },
                                    String::new(),
                                );
                                (fixed, fix)
                            }
                        };

                        ctx.report_with_fix(
                            self.name(),
                            Span {
                                start: fn_start,
                                end: fn_end,
                            },
                            Self::expected_message(fn_name, &fixed_name),
                            Some(fix),
                        );
                    }
                }

                i = fn_end;
                continue;
            }

            i += 1;
        }
    }
}

/// Checks `isStandardSyntaxColorFunction(node)`: returns `false` if any top-level
/// argument token inside the function call starts with `#` (hex color) or `$`
/// (Sass variable), such as `rgba(#fff, 0.5)` or `rgba($color, 0.5)`.
fn is_standard_syntax_color_function(ctx: &LintContext<'_>, args_start: u32, limit: u32) -> bool {
    let mut pos = args_start;
    let mut paren_depth: u32 = 0;

    while pos < limit {
        let b = byte_at(ctx, pos);

        if b == b'/' && pos + 1 < limit && byte_at(ctx, pos + 1) == b'*' {
            pos = skip_block_comment(ctx, pos, limit);
            continue;
        }
        if b == b'"' || b == b'\'' {
            pos = skip_string(ctx, pos, limit);
            continue;
        }
        if b == b'(' {
            paren_depth += 1;
            pos += 1;
            continue;
        }
        if b == b')' {
            if paren_depth == 0 {
                return true;
            }
            paren_depth -= 1;
            pos += 1;
            continue;
        }

        if paren_depth == 0 && (b == b'#' || b == b'$') {
            return false;
        }

        pos += 1;
    }

    true
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject};

    const NAME: &str = "color-function-alias-notation";

    fn expected(unfixed: &str, fixed: &str) -> String {
        format!("Expected \"{unfixed}\" to be \"{fixed}\"")
    }

    #[test]
    fn with_alpha() {
        let pass = &[
            "a { color: rgba(0 0 0) }",
            "a { color: hsla(4.71239rad 60% 70%) }",
            "a { color: HSLA(270 60% 70%) }",
            "a { color: RGBA(240 0 0 / 50%) }",
            "a {\n\tcolor: hsla(\n\t\t270 /* comment */\n\t\t60%\n\t\t50%\n\t\t/* comment */ / 15%\n\t)\n}",
        ];

        let fail = &[
            reject(
                "a { color: rgb(0 0 0) }",
                "a { color: rgba(0 0 0) }",
                expected("rgb", "rgba"),
                (1, 12, 1, 15),
            )
            .with_edit_info(0, (13, 14, "ba")),
            reject(
                "a { color: hsl(270 60% 70%) }",
                "a { color: hsla(270 60% 70%) }",
                expected("hsl", "hsla"),
                (1, 12, 1, 15),
            )
            .with_edit_info(0, (13, 14, "la")),
            reject(
                "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
                "a {\n\tcolor: HSLa(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
                expected("HSL", "HSLa"),
                (2, 9, 2, 12),
            )
            .with_edit_info(0, (14, 15, "La")),
        ];

        Tester::new(NAME, json!(["with-alpha"]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn without_alpha() {
        let pass = &[
            "a { color: rgb(0 0 0) }",
            "a { color: hsl(4.71239rad 60% 70%) }",
            "a { color: HSL(270 60% 70%) }",
            "a { color: RGB(240 0 0 / 50%) }",
            "a {\n\tcolor: hsl(\n\t\t270 /* comment */\n\t\t60%\n\t\t50%\n\t\t/* comment */ / 15%\n\t)\n}",
        ];

        let fail = &[
            reject(
                "a { color: rgba(0 0 0) }",
                "a { color: rgb(0 0 0) }",
                expected("rgba", "rgb"),
                (1, 12, 1, 16),
            )
            .with_edit_info(0, (14, 15, "")),
            reject(
                "a { color: hsla(270 60% 70%) }",
                "a { color: hsl(270 60% 70%) }",
                expected("hsla", "hsl"),
                (1, 12, 1, 16),
            )
            .with_edit_info(0, (14, 15, "")),
            reject(
                "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
                "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
                expected("HSLA", "HSL"),
                (2, 9, 2, 13),
            )
            .with_edit_info(0, (15, 16, "")),
        ];

        Tester::new(NAME, json!(["without-alpha"]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }
}
