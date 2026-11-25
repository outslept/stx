//! Rule `at-rule-no-vendor-prefix`: disallow vendor prefixes for at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-no-vendor-prefix>

use std::ops::Deref;

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::declare_stx_lint;
use crate::fix::Fix;
use crate::rules::Rule;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::{PREFIXES, in_set_ignore_case};

/// Standard at-rule names that Autoprefixer / Stylelint considers vendor-prefixable.
const AUTOPREFIXABLE_AT_KEYWORDS: &[&str] = &["keyframes", "viewport", "document"];

declare_stx_lint! {
    /// Disallow vendor prefixes for at-rules.
    AtRuleNoVendorPrefix,
    "at-rule-no-vendor-prefix",
    restriction,
    fix = fix,
    recommended = false,
}

#[derive(Default)]
pub struct AtRuleNoVendorPrefixConfig {
    /// Compiled matchers for the `ignoreAtRules` secondary option.
    ignore_at_rules: Vec<OptionMatcher>,
}

/// [`at-rule-no-vendor-prefix`](https://stylelint.io/user-guide/rules/at-rule-no-vendor-prefix)
///
/// Disallow vendor prefixes for at-rules.
#[derive(Default)]
pub struct AtRuleNoVendorPrefix(AtRuleNoVendorPrefixConfig);

impl Deref for AtRuleNoVendorPrefix {
    type Target = AtRuleNoVendorPrefixConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AtRuleNoVendorPrefix {
    /// Creates a new `at-rule-no-vendor-prefix` rule instance from secondary options.
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self(AtRuleNoVendorPrefixConfig {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        })
    }
}

/// Formats the diagnostic message when a vendor-prefixed at-rule is found.
#[inline]
fn rejected_message(at_rule: &str) -> String {
    format!("Vendor-prefixed at-rule \"{at_rule}\"")
}

impl Rule for AtRuleNoVendorPrefix {
    fn name(&self) -> &'static str {
        Self::NAME
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
                            ctx.report_with_fix(Self::NAME, span, rejected_message(at_name), fix);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_edit};
    use serde_json::json;

    #[test]
    fn default_options() {
        let pass = vec![
            "@keyframes { 0% { top: 0; } }",
            "@viewport { orientation: landscape; }",
        ];

        let fail = vec![
            reject(
                "@-webkit-keyframes { 0% { top: 0; } }",
                "@keyframes { 0% { top: 0; } }",
                rejected_message("@-webkit-keyframes"),
                (1, 1, 1, 19),
            ),
            reject(
                "@-wEbKiT-kEyFrAmEs { 0% { top: 0; } }",
                "@kEyFrAmEs { 0% { top: 0; } }",
                rejected_message("@-wEbKiT-kEyFrAmEs"),
                (1, 1, 1, 19),
            ),
            reject(
                "@-WEBKIT-KEYFRAMES { 0% { top: 0; } }",
                "@KEYFRAMES { 0% { top: 0; } }",
                rejected_message("@-WEBKIT-KEYFRAMES"),
                (1, 1, 1, 19),
            ),
            reject(
                "@-moz-keyframes { 0% { top: 0; } }",
                "@keyframes { 0% { top: 0; } }",
                rejected_message("@-moz-keyframes"),
                (1, 1, 1, 16),
            ),
            reject(
                "@-ms-viewport { orientation: landscape; }",
                "@viewport { orientation: landscape; }",
                rejected_message("@-ms-viewport"),
                (1, 1, 1, 14),
            ),
        ];

        Tester::new(AtRuleNoVendorPrefix::NAME, json!(true), &pass, &fail)
            .with_fix(true)
            .test();

        let fail_edits = vec![
            reject_edit(
                "@-webkit-keyframes { 0% { top: 0; } }",
                rejected_message("@-webkit-keyframes"),
                (1, 1, 1, 19),
                (1, 9, ""),
            ),
            reject_edit(
                "@-wEbKiT-kEyFrAmEs { 0% { top: 0; } }",
                rejected_message("@-wEbKiT-kEyFrAmEs"),
                (1, 1, 1, 19),
                (1, 9, ""),
            ),
            reject_edit(
                "@-WEBKIT-KEYFRAMES { 0% { top: 0; } }",
                rejected_message("@-WEBKIT-KEYFRAMES"),
                (1, 1, 1, 19),
                (1, 9, ""),
            ),
            reject_edit(
                "@-moz-keyframes { 0% { top: 0; } }",
                rejected_message("@-moz-keyframes"),
                (1, 1, 1, 16),
                (1, 6, ""),
            ),
            reject_edit(
                "@-ms-viewport { orientation: landscape; }",
                rejected_message("@-ms-viewport"),
                (1, 1, 1, 14),
                (1, 5, ""),
            ),
        ];

        Tester::new(AtRuleNoVendorPrefix::NAME, json!(true), &[], &fail_edits)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn ignore_at_rules() {
        let pass = vec![
            "@keyframes {}",
            "@-webkit-keyframes {}",
            "@viewport {}",
            "@-MS-VIEWPORT {}",
        ];

        let config = json!([true, { "ignoreAtRules": ["-webkit-keyframes", "/-viewport/i"] }]);

        let fail = vec![reject(
            "@-MOZ-DOCUMENT url(\"https://example.com/\") {}",
            "@DOCUMENT url(\"https://example.com/\") {}",
            rejected_message("@-MOZ-DOCUMENT"),
            (1, 1, 1, 15),
        )];

        Tester::new(AtRuleNoVendorPrefix::NAME, config.clone(), &pass, &fail)
            .with_fix(true)
            .test();

        let fail_edits = vec![reject_edit(
            "@-MOZ-DOCUMENT url(\"https://example.com/\") {}",
            rejected_message("@-MOZ-DOCUMENT"),
            (1, 1, 1, 15),
            (1, 6, ""),
        )];

        Tester::new(AtRuleNoVendorPrefix::NAME, config, &[], &fail_edits)
            .with_compute_edit_info(true)
            .test();
    }
}
