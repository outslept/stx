//! Disallows vendor prefixes for at-rules (`at-rule-no-vendor-prefix`).
//!
//! # Checked Constructs
//!
//! Standard CSS at-rules whose names start with `-` are inspected:
//!
//! 1. **Autoprefixable vendor-prefixed at-rules** (case-insensitive, matching
//!    `isAutoprefixable.atRuleName(name)`):
//!    - `@-webkit-keyframes`, `@-moz-keyframes`, `@-o-keyframes`, `@-ms-keyframes`
//!    - `@-ms-viewport`, `@-o-viewport`
//!    - `@-moz-document`
//!
//! 2. **Ignored constructs**:
//!    - Custom or non-autoprefixable vendor-prefixed at-rules.
//!    - Non-standard preprocessor constructs (`isStandardSyntaxAtRule`): Less
//!      variable definitions (`@-webkit-keyframes: ...`), Less mixin calls
//!      (`@-webkit-keyframes()`), and SCSS/Less interpolated at-rule names
//!      (`@-webkit-keyframes-#{$var}`).

use std::ops::Deref;

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::fix::Fix;
use crate::rules::Rule;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::{PREFIXES, in_set_ignore_case};

/// Standard at-rule names that Autoprefixer / Stylelint considers vendor-prefixable
/// (`isAutoprefixable.atRuleName` in Stylelint).
const AUTOPREFIXABLE_AT_KEYWORDS: &[&str] = &["keyframes", "viewport", "document"];

/// Configuration for [`AtRuleNoVendorPrefix`].
#[derive(Debug, Default)]
pub struct AtRuleNoVendorPrefixConfig {
    /// Compiled matchers from `{ "ignoreAtRules": [...] }`.
    ignore_at_rules: Vec<OptionMatcher>,
}

crate::define_stx_lint! {
    /// Disallow vendor prefixes for at-rules.
    ///
    /// Flags recognized vendor prefixes on autoprefixable at-rules
    /// (`@keyframes`, `@viewport`, and `@document`) and removes the vendor
    /// prefix when autofixing.
    "at-rule-no-vendor-prefix", restriction, fix = fix,
    short_description = "Disallow vendor prefixes for at-rules.";
    #[derive(Debug, Default)]
    pub struct AtRuleNoVendorPrefix(AtRuleNoVendorPrefixConfig);
}

impl Deref for AtRuleNoVendorPrefix {
    type Target = AtRuleNoVendorPrefixConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Formats the diagnostic message matching Stylelint's `messages.rejected`.
#[inline]
#[must_use]
fn rejected(at_rule: &str) -> String {
    format!("Vendor-prefixed at-rule \"{at_rule}\"")
}

impl AtRuleNoVendorPrefix {
    /// Constructs a configured [`AtRuleNoVendorPrefix`] rule instance from raw
    /// Stylelint secondary options.
    #[must_use]
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self(AtRuleNoVendorPrefixConfig {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        })
    }
}

impl Rule for AtRuleNoVendorPrefix {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let bytes = ctx.source.as_bytes();
        let len = bytes.len();
        let mut pos: usize = 0;
        // Track whether the scanner is at a position where a statement / at-rule can start
        // (start of file, or after `{`, `}`, `;` at top-level parenthesis depth).
        let mut at_statement_start = true;
        let mut paren_depth: u32 = 0;

        while pos < len {
            let b = bytes[pos];

            // Whitespace preserves `at_statement_start`.
            if b.is_ascii_whitespace() {
                pos += 1;
                continue;
            }

            // Skip block comments `/* ... */` (preserving `at_statement_start`).
            if b == b'/' && bytes.get(pos + 1) == Some(&b'*') {
                pos = skip_block_comment(bytes, pos + 2);
                continue;
            }

            // Skip single-line comments `// ...` when not part of a URL scheme (`://`).
            if b == b'/'
                && bytes.get(pos + 1) == Some(&b'/')
                && (pos == 0 || bytes[pos - 1] != b':')
            {
                pos = skip_line_comment(bytes, pos + 2);
                continue;
            }

            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                at_statement_start = false;
                pos = skip_quoted_string(bytes, pos + 1, b);
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
                    let at_start = pos;
                    let name_start = pos + 1;
                    if name_start < len && is_ident_start(bytes[name_start]) {
                        let mut name_end = name_start + 1;
                        while name_end < len && is_ident_char(bytes[name_end]) {
                            name_end += 1;
                        }

                        if is_standard_at_rule_after_name(bytes, name_end) {
                            let at_start_u32 = u32::try_from(at_start).unwrap_or(u32::MAX);
                            let name_start_u32 = u32::try_from(name_start).unwrap_or(u32::MAX);
                            let name_end_u32 = u32::try_from(name_end).unwrap_or(u32::MAX);
                            let name = ctx.slice_range(name_start_u32, name_end_u32);

                            if name.starts_with('-')
                                && !options_matches(&self.ignore_at_rules, name)
                                && let Some(prefix_len) = autoprefixable_vendor_prefix_len(name)
                            {
                                let at_name = ctx.slice_range(at_start_u32, name_end_u32);
                                let span = Span {
                                    start: at_start_u32,
                                    end: name_end_u32,
                                };
                                let fix = Some(Fix::single(
                                    Span {
                                        start: name_start_u32,
                                        end: name_start_u32 + prefix_len,
                                    },
                                    String::new(),
                                ));
                                ctx.report_with_fix(Self::NAME, span, rejected(at_name), fix);
                            }
                        }

                        at_statement_start = false;
                        pos = name_end;
                    } else {
                        at_statement_start = false;
                        pos += 1;
                    }
                }
                _ => {
                    at_statement_start = false;
                    pos += 1;
                }
            }
        }
    }
}

/// Checks whether the token starting at `name_end` (immediately after `@<name>`)
/// belongs to a standard-syntax CSS/SCSS at-rule (`isStandardSyntaxAtRule` in Stylelint).
#[must_use]
fn is_standard_at_rule_after_name(bytes: &[u8], name_end: usize) -> bool {
    let len = bytes.len();

    // `@name:` is a Less variable definition (`atRule.variable`), not a standard at-rule.
    if bytes.get(name_end) == Some(&b':') {
        return false;
    }

    // Interpolation attached directly to the at-rule name (`@-webkit-keyframes#{$var}`).
    if bytes.get(name_end) == Some(&b'$')
        || (bytes.get(name_end) == Some(&b'#') && bytes.get(name_end + 1) == Some(&b'{'))
        || (bytes.get(name_end) == Some(&b'@') && bytes.get(name_end + 1) == Some(&b'{'))
    {
        return false;
    }

    let has_immediate_paren = bytes.get(name_end) == Some(&b'(');
    let mut pos = name_end;
    let mut paren_depth: u32 = 0;
    let mut has_params = false;
    let mut first_param_byte: Option<u8> = None;

    while pos < len {
        let b = bytes[pos];
        if b.is_ascii_whitespace() {
            pos += 1;
            continue;
        }
        if b == b'/' && bytes.get(pos + 1) == Some(&b'*') {
            pos = skip_block_comment(bytes, pos + 2);
            continue;
        }
        if b == b'/' && bytes.get(pos + 1) == Some(&b'/') && (pos == 0 || bytes[pos - 1] != b':') {
            pos = skip_line_comment(bytes, pos + 2);
            continue;
        }
        if (b == b'{' || b == b';' || b == b'}') && paren_depth == 0 {
            break;
        }
        if first_param_byte.is_none() {
            first_param_byte = Some(b);
        }
        has_params = true;
        match b {
            b'"' | b'\'' => {
                pos = skip_quoted_string(bytes, pos + 1, b);
            }
            b'(' => {
                paren_depth += 1;
                pos += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                pos += 1;
            }
            _ => {
                pos += 1;
            }
        }
    }

    // `@name : value;` or `@name : { ... };` is a Less variable / detached ruleset definition.
    if first_param_byte == Some(b':') {
        return false;
    }

    let has_block = bytes.get(pos) == Some(&b'{');
    // In Stylelint's `isStandardSyntaxAtRule`:
    // 1. `if (!atRule.nodes && atRule.params === '') return false;`
    // 2. `if (!atRule.nodes && atRule.raws.afterName === '' && atRule.params[0] === '(') return false;`
    if !has_block && (!has_params || has_immediate_paren) {
        return false;
    }

    true
}

/// Returns the length in bytes of the vendor prefix if `name` is an autoprefixable
/// vendor-prefixed at-rule name (`isAutoprefixable.atRuleName(name)` in Stylelint).
#[inline]
#[must_use]
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

/// Skips past the closing `*/` of a block comment.
#[inline]
#[must_use]
fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
    let len = bytes.len();
    let mut pos = start;
    while pos < len {
        if bytes[pos] == b'*' && bytes.get(pos + 1) == Some(&b'/') {
            return pos + 2;
        }
        pos += 1;
    }
    len
}

/// Skips past the end of a single-line `// ...` comment.
#[inline]
#[must_use]
fn skip_line_comment(bytes: &[u8], start: usize) -> usize {
    let len = bytes.len();
    let mut pos = start;
    while pos < len {
        if bytes[pos] == b'\n' {
            return pos + 1;
        }
        pos += 1;
    }
    len
}

/// Skips past the closing `quote` character of a string literal, respecting `\` escapes.
#[inline]
#[must_use]
fn skip_quoted_string(bytes: &[u8], start: usize, quote: u8) -> usize {
    let len = bytes.len();
    let mut pos = start;
    while pos < len {
        let b = bytes[pos];
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

/// Returns `true` if `b` can start a CSS identifier (ASCII letter, `_`, `-`, or non-ASCII byte).
#[inline]
#[must_use]
const fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-' || b >= 0x80
}

/// Returns `true` if `b` can continue a CSS identifier.
#[inline]
#[must_use]
const fn is_ident_char(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
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
                rejected("@-webkit-keyframes"),
                (1, 1, 1, 19),
            ),
            reject(
                "@-wEbKiT-kEyFrAmEs { 0% { top: 0; } }",
                "@kEyFrAmEs { 0% { top: 0; } }",
                rejected("@-wEbKiT-kEyFrAmEs"),
                (1, 1, 1, 19),
            ),
            reject(
                "@-WEBKIT-KEYFRAMES { 0% { top: 0; } }",
                "@KEYFRAMES { 0% { top: 0; } }",
                rejected("@-WEBKIT-KEYFRAMES"),
                (1, 1, 1, 19),
            ),
            reject(
                "@-moz-keyframes { 0% { top: 0; } }",
                "@keyframes { 0% { top: 0; } }",
                rejected("@-moz-keyframes"),
                (1, 1, 1, 16),
            ),
            reject(
                "@-ms-viewport { orientation: landscape; }",
                "@viewport { orientation: landscape; }",
                rejected("@-ms-viewport"),
                (1, 1, 1, 14),
            ),
        ];

        Tester::new(AtRuleNoVendorPrefix::NAME, json!(true), &pass, &fail)
            .with_fix(true)
            .test();

        let fail_edits = vec![
            reject_edit(
                "@-webkit-keyframes { 0% { top: 0; } }",
                rejected("@-webkit-keyframes"),
                (1, 1, 1, 19),
                (1, 9, ""),
            ),
            reject_edit(
                "@-wEbKiT-kEyFrAmEs { 0% { top: 0; } }",
                rejected("@-wEbKiT-kEyFrAmEs"),
                (1, 1, 1, 19),
                (1, 9, ""),
            ),
            reject_edit(
                "@-WEBKIT-KEYFRAMES { 0% { top: 0; } }",
                rejected("@-WEBKIT-KEYFRAMES"),
                (1, 1, 1, 19),
                (1, 9, ""),
            ),
            reject_edit(
                "@-moz-keyframes { 0% { top: 0; } }",
                rejected("@-moz-keyframes"),
                (1, 1, 1, 16),
                (1, 6, ""),
            ),
            reject_edit(
                "@-ms-viewport { orientation: landscape; }",
                rejected("@-ms-viewport"),
                (1, 1, 1, 14),
                (1, 5, ""),
            ),
        ];

        Tester::new(AtRuleNoVendorPrefix::NAME, json!(true), [], &fail_edits)
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
            rejected("@-MOZ-DOCUMENT"),
            (1, 1, 1, 15),
        )];

        Tester::new(AtRuleNoVendorPrefix::NAME, config.clone(), &pass, &fail)
            .with_fix(true)
            .test();

        let fail_edits = vec![reject_edit(
            "@-MOZ-DOCUMENT url(\"https://example.com/\") {}",
            rejected("@-MOZ-DOCUMENT"),
            (1, 1, 1, 15),
            (1, 6, ""),
        )];

        Tester::new(AtRuleNoVendorPrefix::NAME, config, [], &fail_edits)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn utf8_comments_strings_and_non_standard_at_rules() {
        let pass = vec![
            "/* Привет мир 🎨 @-webkit-keyframes {} */ a { content: \"@-webkit-keyframes\"; }",
            "@-custom-at-rule { color: red; }",
            "@-webkit-keyframes-#{$suffix} { 0% { top: 0; } }",
        ];
        let fail = vec![reject(
            "/* Привет 🎨 */ @-webkit-keyframes fade { 0% { top: 0; } }",
            "/* Привет 🎨 */ @keyframes fade { 0% { top: 0; } }",
            rejected("@-webkit-keyframes"),
            (1, 16, 1, 34),
        )];

        Tester::new(AtRuleNoVendorPrefix::NAME, json!(true), &pass, &fail)
            .with_custom_syntax("postcss-scss")
            .with_fix(true)
            .test();

        let less_pass = vec![
            "@-webkit-keyframes: 10px;\n.box { width: @-webkit-keyframes; }",
            "@-webkit-keyframes();",
        ];
        Tester::new(AtRuleNoVendorPrefix::NAME, json!(true), &less_pass, &[])
            .with_custom_syntax("postcss-less")
            .test();
    }
}
