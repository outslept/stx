//! Rule `at-rule-disallowed-list`: specify a list of disallowed at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-disallowed-list>

use crate::config::RuleSetting;
use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::{
    OptionMatcher, has_interpolation, options_matches, parse_primary_option_matchers,
};
use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;

/// Enforces that standard-syntax at-rules do not match a configured disallowed
/// list of at-rule names or regular expressions.
///
/// # What it does
///
/// Checks every standard-syntax at-rule in a stylesheet and reports a diagnostic
/// when its name (excluding the leading `@`) matches any entry in the primary
/// option list. Non-standard preprocessor constructs (such as Less mixins and
/// variable declarations) are ignored.
///
/// # Why is this bad?
///
/// Certain at-rules (for example `@extend`, `@import`, or specific vendor-prefixed
/// at-rules) may be forbidden by project conventions, performance guidelines, or
/// target browser policies.
///
/// # Options
///
/// ## Primary Option
///
/// `string | RegExp | Array<string | RegExp>`: disallowed at-rule names (without
/// the leading `@`) or regular expressions.
///
/// ```json
/// {
///   "at-rule-disallowed-list": ["extend", "supports", "/^(-webkit-|-moz-)?keyframes$/i"]
/// }
/// ```
///
/// # Examples
///
/// Given `["extend", "/^(-webkit-|-moz-)?keyframes$/i"]`:
///
/// ## Incorrect
///
/// ```scss
/// a { @extend %placeholder; }
/// @keyframes name { from { top: 10px; } to { top: 20px; } }
/// @-moz-keyframes name { from { top: 10px; } to { top: 20px; } }
/// ```
///
/// ## Correct
///
/// ```scss
/// a { color: pink; }
/// @mixin name($p) {}
/// ```
///
/// # References
///
/// - [Stylelint: `at-rule-disallowed-list`](https://stylelint.io/user-guide/rules/at-rule-disallowed-list)
pub struct AtRuleDisallowedList {
    /// Compiled matchers for disallowed at-rule names (without the leading `@`).
    disallowed: Vec<OptionMatcher>,
}

impl AtRuleDisallowedList {
    /// Creates a new [`AtRuleDisallowedList`] rule instance from the given
    /// [`RuleSetting`].
    ///
    /// Returns `None` if no valid disallowed matchers are configured.
    pub fn from_setting(setting: &RuleSetting) -> Option<Self> {
        let disallowed = parse_primary_option_matchers(&setting.primary);
        if disallowed.is_empty() {
            return None;
        }
        Some(Self { disallowed })
    }

    /// Formats the diagnostic message when an `at_rule` (including the leading
    /// `@`) matches the disallowed list.
    fn rejected_message(at_rule: &str) -> String {
        format!("Disallowed at-rule \"{at_rule}\"")
    }
}

impl Rule for AtRuleDisallowedList {
    fn name(&self) -> &'static str {
        "at-rule-disallowed-list"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let mut pos: u32 = 0;
        let mut at_statement_start = true;
        let mut paren_depth: u32 = 0;

        while pos < len {
            let Some(b) = byte_at(ctx, pos) else {
                break;
            };

            // Skip UTF-8 BOM at the start of the file.
            if pos == 0 && ctx.source.starts_with('\u{feff}') {
                pos += 3;
                continue;
            }

            // Skip whitespace.
            if b.is_ascii_whitespace() {
                pos += 1;
                continue;
            }

            // Skip block comments `/* ... */`.
            if b == b'/' && byte_at(ctx, pos + 1) == Some(b'*') {
                pos = skip_block_comment(ctx, pos + 2, len);
                continue;
            }

            // Skip line comments `// ...` in SCSS/Less.
            if b == b'/' && byte_at(ctx, pos + 1) == Some(b'/') {
                pos = skip_line_comment(ctx, pos + 2, len);
                continue;
            }

            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                pos = skip_quoted_string(ctx, pos + 1, len, b);
                at_statement_start = false;
                continue;
            }

            match b {
                b'(' => {
                    paren_depth += 1;
                    at_statement_start = false;
                    pos += 1;
                }
                b')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                    at_statement_start = false;
                    pos += 1;
                }
                b'{' | b'}' | b';' if paren_depth == 0 => {
                    at_statement_start = true;
                    pos += 1;
                }
                b'@' if at_statement_start && paren_depth == 0 => {
                    let at_pos = pos;
                    let name_start = at_pos + 1;
                    let name_end = scan_ident_end(ctx, name_start, len);

                    if name_end > name_start {
                        let name = ctx.slice_range(name_start, name_end);
                        if is_standard_at_rule_after_name(ctx, name_end, len)
                            && options_matches(&self.disallowed, name)
                        {
                            let at_name = ctx.slice_range(at_pos, name_end);
                            ctx.report(
                                self.name(),
                                Span {
                                    start: at_pos,
                                    end: name_end,
                                },
                                Self::rejected_message(at_name),
                            );
                        }
                        pos = name_end;
                    } else {
                        pos += 1;
                    }
                    at_statement_start = false;
                }
                _ => {
                    at_statement_start = false;
                    pos += 1;
                }
            }
        }
    }
}

/// Scans an ASCII identifier (`[a-zA-Z0-9_-]+`) starting at `start` and returns
/// its end byte offset.
fn scan_ident_end(ctx: &LintContext<'_>, start: u32, len: u32) -> u32 {
    let mut pos = start;
    while pos < len {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
            pos += 1;
        } else {
            break;
        }
    }
    pos
}

/// Checks whether the token starting at `name_end` (immediately after `@<name>`)
/// belongs to a standard-syntax CSS/SCSS at-rule rather than a Less variable,
/// detached ruleset, or mixin call.
fn is_standard_at_rule_after_name(ctx: &LintContext<'_>, name_end: u32, len: u32) -> bool {
    // `@name:` or `@name(` is a Less variable/mixin, not a standard at-rule.
    if matches!(byte_at(ctx, name_end), Some(b':' | b'(')) {
        return false;
    }

    let mut pos = name_end;
    let mut paren_depth: u32 = 0;

    while pos < len {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };
        match b {
            b'/' if byte_at(ctx, pos + 1) == Some(b'*') => {
                pos = skip_block_comment(ctx, pos + 2, len);
            }
            b'"' | b'\'' => {
                pos = skip_quoted_string(ctx, pos + 1, len, b);
            }
            b'(' => {
                paren_depth += 1;
                pos += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                pos += 1;
            }
            b'{' | b';' | b'}' if paren_depth == 0 => {
                break;
            }
            _ => {
                pos += 1;
            }
        }
    }

    let prelude = ctx.slice_range(name_end, pos).trim();
    if prelude.starts_with(':') || has_interpolation(prelude) {
        return false;
    }

    true
}

/// Skips past the closing `*/` of a block comment.
fn skip_block_comment(ctx: &LintContext<'_>, start: u32, len: u32) -> u32 {
    let mut pos = start;
    while pos < len {
        if byte_at(ctx, pos) == Some(b'*') && byte_at(ctx, pos + 1) == Some(b'/') {
            return pos + 2;
        }
        pos += 1;
    }
    len
}

/// Skips past the end of a single-line `// ...` comment.
fn skip_line_comment(ctx: &LintContext<'_>, start: u32, len: u32) -> u32 {
    let mut pos = start;
    while pos < len {
        if byte_at(ctx, pos) == Some(b'\n') {
            return pos + 1;
        }
        pos += 1;
    }
    len
}

/// Skips past the closing `quote` character of a string literal, respecting `\` escapes.
fn skip_quoted_string(ctx: &LintContext<'_>, start: u32, len: u32, quote: u8) -> u32 {
    let mut pos = start;
    while pos < len {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };
        if b == b'\\' {
            pos = (pos + 2).min(len);
        } else if b == quote {
            return pos + 1;
        } else {
            pos += 1;
        }
    }
    len
}

/// Returns the byte at `offset` in `ctx.source` using `u32` span indexing.
#[inline]
fn byte_at(ctx: &LintContext<'_>, offset: u32) -> Option<u8> {
    ctx.slice_range(offset, offset.saturating_add(1))
        .as_bytes()
        .first()
        .copied()
}
