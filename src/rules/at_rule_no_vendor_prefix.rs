//! Rule `at-rule-no-vendor-prefix`: disallow vendor prefixes for at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-no-vendor-prefix>

use crate::context::LintContext;
use crate::fix::Fix;
use crate::rules::Rule;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::{PREFIXES, in_set_ignore_case};
use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

/// Standard at-rule names that Autoprefixer / Stylelint considers vendor-prefixable.
const AUTOPREFIXABLE_AT_KEYWORDS: &[&str] = &["keyframes", "viewport", "document"];

/// [`at-rule-no-vendor-prefix`](https://stylelint.io/user-guide/rules/at-rule-no-vendor-prefix)
///
/// Disallow vendor prefixes for at-rules.
///
/// ### What it does
///
/// Reports standard CSS at-rules that use a vendor prefix (such as
/// `@-webkit-keyframes`, `@-moz-keyframes`, `@-ms-viewport`, or
/// `@-moz-document`).
///
/// ### Why is this bad?
///
/// Vendor prefixes for standard at-rules like `@keyframes` are no longer needed
/// in modern browsers or should be managed automatically by a build tool such
/// as Autoprefixer rather than maintained by hand in source stylesheets.
///
/// ### Autofix
///
/// This rule automatically removes the vendor prefix from the at-rule name
/// while preserving the original casing of the unprefixed at-rule identifier.
///
/// ### Primary Option
///
/// - `true`: enable the rule.
///
/// ### Secondary Options
///
/// ```json
/// {
///   "ignoreAtRules": ["-webkit-keyframes", "/-viewport/i"]
/// }
/// ```
///
/// - `ignoreAtRules`: an array of strings or regular expressions (wrapped in
///   `/.../` or `/.../i`) matching vendor-prefixed at-rule names (without the
///   leading `@`) that should be ignored.
///
/// ### Examples
///
/// #### Incorrect
///
/// ```css
/// @-webkit-keyframes slide { 0% { top: 0; } }
/// @-ms-viewport { orientation: landscape; }
/// @-moz-document url("https://example.com/") {}
/// ```
///
/// #### Correct
///
/// ```css
/// @keyframes slide { 0% { top: 0; } }
/// @viewport { orientation: landscape; }
/// @document url("https://example.com/") {}
/// ```
pub struct AtRuleNoVendorPrefix {
    /// Compiled matchers for the `ignoreAtRules` secondary option.
    ignore_at_rules: Vec<OptionMatcher>,
}

impl Default for AtRuleNoVendorPrefix {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AtRuleNoVendorPrefix {
    /// Creates a new `at-rule-no-vendor-prefix` rule instance from secondary options.
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        }
    }

    /// Formats the diagnostic message when a vendor-prefixed at-rule is found.
    #[inline]
    fn rejected_message(at_rule: &str) -> String {
        format!("Vendor-prefixed at-rule \"{at_rule}\"")
    }
}

impl Rule for AtRuleNoVendorPrefix {
    fn name(&self) -> &'static str {
        "at-rule-no-vendor-prefix"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let mut i: u32 = 0;

        // Track whether the scanner is at a position where a statement / at-rule can start.
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

            // Statement delimiters allow an at-rule to follow.
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

                        if name.starts_with('-')
                            && !options_matches(&self.ignore_at_rules, name)
                            && let Some(prefix_len) = autoprefixable_vendor_prefix_len(name)
                        {
                            let at_name = ctx.slice_range(at_start, name_end);
                            let span = Span {
                                start: at_start,
                                end: name_end,
                            };
                            let fix = Some(Fix::single(
                                Span {
                                    start: name_start,
                                    end: name_start + prefix_len,
                                },
                                String::new(),
                            ));

                            ctx.report_with_fix(
                                self.name(),
                                span,
                                Self::rejected_message(at_name),
                                fix,
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

/// Returns the length in bytes of the vendor prefix if `name` is an autoprefixable
/// vendor-prefixed at-rule name (matching `isAutoprefixable.atRuleName(name)`).
#[inline]
fn autoprefixable_vendor_prefix_len(name: &str) -> Option<u32> {
    for &prefix in PREFIXES {
        if name.len() > prefix.len() && name[..prefix.len()].eq_ignore_ascii_case(prefix) {
            let unprefixed = &name[prefix.len()..];
            if in_set_ignore_case(AUTOPREFIXABLE_AT_KEYWORDS, unprefixed) {
                return u32::try_from(prefix.len()).ok();
            }
        }
    }
    None
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
