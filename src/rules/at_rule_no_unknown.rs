//! Rule `at-rule-no-unknown`: disallow unknown at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-no-unknown>

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::{AT_KEYWORDS, in_set_ignore_case};

/// [`at-rule-no-unknown`](https://stylelint.io/user-guide/rules/at-rule-no-unknown)
///
/// Disallow unknown at-rules.
///
/// ### What it does
///
/// Reports standard CSS at-rules that are not defined in the W3C / CSSWG
/// specifications (as cataloged in [`AT_KEYWORDS`]).
///
/// This rule ignores:
/// - Vendor-prefixed at-rules (such as `@-webkit-keyframes` or `@-moz-document`).
/// - Non-standard preprocessor constructs such as Less variables (`@my-var: 10px;`)
///   and Less mixins (`.mixin()`).
///
/// ### Why is this bad?
///
/// Unknown at-rules are typically typos, leftover preprocessor directives in plain
/// CSS files, or invented syntax that browsers will ignore at parse time.
///
/// ### Primary Option
///
/// - `true`: enable the rule.
///
/// ### Secondary Options
///
/// ```json
/// {
///   "ignoreAtRules": ["unknown", "/^my-/", "/^YOUR-/i"]
/// }
/// ```
///
/// - `ignoreAtRules`: an array of strings or regular expressions (wrapped in
///   `/.../` or `/.../i`) matching at-rule names (without the leading `@`) that
///   should not be reported.
///
/// ### Examples
///
/// #### Incorrect
///
/// ```css
/// @unknown-at-rule {}
/// ```
///
/// #### Correct
///
/// ```css
/// @charset "UTF-8";
/// @media (max-width: 960px) {}
/// @-webkit-keyframes slide {}
/// ```
pub struct AtRuleNoUnknown {
    /// Compiled matchers for the `ignoreAtRules` secondary option.
    ignore_at_rules: Vec<OptionMatcher>,
}

impl Default for AtRuleNoUnknown {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AtRuleNoUnknown {
    /// Creates a new `at-rule-no-unknown` rule instance from secondary options.
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        }
    }

    /// Formats the diagnostic message when an unknown at-rule is found.
    #[inline]
    fn rejected_message(at_rule: &str) -> String {
        format!("Unknown at-rule \"{at_rule}\"")
    }
}

impl Rule for AtRuleNoUnknown {
    fn name(&self) -> &'static str {
        "at-rule-no-unknown"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let mut i: u32 = 0;

        // Track whether the scanner is at the beginning of a statement where an
        // at-rule can appear (file start, or after `{`, `}`, `;`).
        let mut at_statement_start = true;

        while i < len {
            let b = byte_at(ctx, i);

            // Whitespace preserves `at_statement_start`.
            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            // Skip block comments `/* ... */` (preserving `at_statement_start`).
            if b == b'/' && i + 1 < len && byte_at(ctx, i + 1) == b'*' {
                i += 2;
                while i + 1 < len && !(byte_at(ctx, i) == b'*' && byte_at(ctx, i + 1) == b'/') {
                    i += 1;
                }
                i = if i + 1 < len { i + 2 } else { len };
                continue;
            }

            // Skip single-line comments `// ...` when not part of a URL scheme (`://`).
            if b == b'/'
                && i + 1 < len
                && byte_at(ctx, i + 1) == b'/'
                && (i == 0 || byte_at(ctx, i - 1) != b':')
            {
                i += 2;
                while i < len && byte_at(ctx, i) != b'\n' {
                    i += 1;
                }
                continue;
            }

            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                at_statement_start = false;
                let quote = b;
                i += 1;
                while i < len {
                    let cur = byte_at(ctx, i);
                    if cur == b'\\' {
                        i = (i + 2).min(len);
                        continue;
                    }
                    if cur == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }

            // Statement delimiters allow an at-rule to start next.
            if matches!(b, b'{' | b'}' | b';') {
                at_statement_start = true;
                i += 1;
                continue;
            }

            // Check for an at-rule at the start of a statement.
            if b == b'@' && at_statement_start {
                let at_start = i;
                let name_start = i + 1;

                if name_start < len && is_ident_start(byte_at(ctx, name_start)) {
                    let mut name_end = name_start + 1;
                    while name_end < len && is_ident_char(byte_at(ctx, name_end)) {
                        name_end += 1;
                    }

                    // Ignore Less variable declarations (`@var: ...`) and Less mixin calls (`@mixin(...)`).
                    let next_non_ws = skip_spaces_and_tabs(ctx, name_end, len);
                    let is_less_construct = next_non_ws < len
                        && (byte_at(ctx, next_non_ws) == b':'
                            || (next_non_ws == name_end && byte_at(ctx, next_non_ws) == b'('));

                    if !is_less_construct {
                        let name = ctx.slice_range(name_start, name_end);

                        if !options_matches(&self.ignore_at_rules, name)
                            && !has_vendor_prefix(name)
                            && !in_set_ignore_case(AT_KEYWORDS, name)
                        {
                            let at_name = ctx.slice_range(at_start, name_end);
                            ctx.report(
                                self.name(),
                                Span {
                                    start: at_start,
                                    end: name_end,
                                },
                                Self::rejected_message(at_name),
                            );
                        }
                    }

                    at_statement_start = false;
                    i = name_end;
                    continue;
                }
            }

            at_statement_start = false;
            i += 1;
        }
    }
}

/// Returns the byte at `pos` in the source string.
#[inline]
fn byte_at(ctx: &LintContext<'_>, pos: u32) -> u8 {
    ctx.slice_range(pos, pos + 1).as_bytes()[0]
}

/// Returns `true` if `b` can start a CSS identifier (ASCII letter, `_`, `-`, or non-ASCII byte).
#[inline]
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-' || b >= 0x80
}

/// Returns `true` if `b` can continue a CSS identifier.
#[inline]
fn is_ident_char(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

/// Advances past horizontal ASCII whitespace (`' '` and `'\t'`).
#[inline]
fn skip_spaces_and_tabs(ctx: &LintContext<'_>, mut idx: u32, len: u32) -> u32 {
    while idx < len && matches!(byte_at(ctx, idx), b' ' | b'\t') {
        idx += 1;
    }
    idx
}

/// Returns `true` if `s` starts with a vendor prefix (`-<alphanumeric_or_underscore>-`),
/// matching Stylelint's `vendor.prefix(s)` utility.
#[inline]
fn has_vendor_prefix(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 3 || bytes[0] != b'-' {
        return false;
    }
    let mut idx = 1;
    while idx < bytes.len() && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'_') {
        idx += 1;
    }
    idx > 1 && idx < bytes.len() && bytes[idx] == b'-'
}
