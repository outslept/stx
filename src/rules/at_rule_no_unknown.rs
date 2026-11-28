//! Disallows unknown at-rules (`at-rule-no-unknown`).
//!
//! # Checked Constructs
//!
//! 1. Standard-syntax CSS at-rules at the root level or nested inside style/at-rule blocks.
//! 2. Recognized standard at-rules are matched case-insensitively against Stylelint's
//!    `atKeywords` reference set (plus `@charset`, which PostCSS represents separately).
//! 3. Vendor-prefixed at-rules (such as `@-webkit-keyframes`) are ignored, matching
//!    Stylelint's `vendor.prefix(name)` check.
//! 4. Non-standard preprocessor constructs—such as Less variable definitions (`@var: 10px`),
//!    Less detached rulesets and mixin calls (`@mixin()`), parameterless blockless directives
//!    (`@content;`), and at-rules with interpolated names (`@#{$var}`, `@unknown-#{$var}`)—are
//!    ignored via `isStandardSyntaxAtRule`.

use std::ops::Deref;

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::{AT_KEYWORDS, in_set_ignore_case};

/// Configuration for [`AtRuleNoUnknown`].
#[derive(Debug, Default)]
pub struct AtRuleNoUnknownConfig {
    /// Compiled matchers for the `ignoreAtRules` secondary option.
    ignore_at_rules: Vec<OptionMatcher>,
}

crate::define_stx_lint! {
    /// Disallows unknown at-rules.
    ///
    /// Reports any standard-syntax at-rule whose name (without the leading `@`) is not
    /// part of the standard CSS specification, does not have a vendor prefix, and is not
    /// exempted by `ignoreAtRules`.
    "at-rule-no-unknown", correctness, recommended = true,
    short_description = "Disallow unknown at-rules.";

    #[derive(Debug, Default)]
    pub struct AtRuleNoUnknown(AtRuleNoUnknownConfig);
}

impl Deref for AtRuleNoUnknown {
    type Target = AtRuleNoUnknownConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AtRuleNoUnknown {
    /// Creates a new [`AtRuleNoUnknown`] rule instance from secondary options.
    #[must_use]
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self(AtRuleNoUnknownConfig {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        })
    }
}

/// Formats the diagnostic message when an unknown at-rule is found.
#[inline]
#[must_use]
fn rejected_message(at_rule: &str) -> String {
    format!("Unknown at-rule \"{at_rule}\"")
}

impl Rule for AtRuleNoUnknown {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let bytes = ctx.source.as_bytes();
        let len = bytes.len();
        let mut pos: usize = 0;

        // Track whether the scanner is at the beginning of a statement where an
        // at-rule can appear (file start, or after `{`, `}`, `;` at top-level parenthesis depth).
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

            // Skip SCSS `#{...}` interpolation blocks so their closing `}` does not
            // falsely reset `at_statement_start`.
            if b == b'#' && bytes.get(pos + 1) == Some(&b'{') {
                at_statement_start = false;
                pos = skip_interpolation(bytes, pos + 2);
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

                            if !options_matches(&self.ignore_at_rules, name)
                                && !has_vendor_prefix(name)
                                && !name.eq_ignore_ascii_case("charset")
                                && !in_set_ignore_case(AT_KEYWORDS, name)
                            {
                                let at_name = ctx.slice_range(at_start_u32, name_end_u32);
                                ctx.report(
                                    Self::NAME,
                                    Span {
                                        start: at_start_u32,
                                        end: name_end_u32,
                                    },
                                    rejected_message(at_name),
                                );
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
/// belongs to a standard-syntax CSS/SCSS at-rule rather than a Less variable,
/// detached ruleset call, mixin call, parameterless blockless directive, or an
/// at-rule with interpolation in its name.
#[must_use]
fn is_standard_at_rule_after_name(bytes: &[u8], name_end: usize) -> bool {
    let len = bytes.len();

    // `@name:` is a Less variable definition (`atRule.variable`), not a standard at-rule.
    if bytes.get(name_end) == Some(&b':') {
        return false;
    }

    // Interpolation attached directly to the at-rule name (`@unknown-#{$var}`, `@unknown@{var}`).
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

/// Skips past the closing `}` of an SCSS `#{...}` interpolation block.
#[inline]
#[must_use]
fn skip_interpolation(bytes: &[u8], start: usize) -> usize {
    let len = bytes.len();
    let mut pos = start;
    let mut depth: u32 = 1;
    while pos < len {
        match bytes[pos] {
            b'"' | b'\'' => {
                pos = skip_quoted_string(bytes, pos + 1, bytes[pos]);
            }
            b'{' => {
                depth += 1;
                pos += 1;
            }
            b'}' => {
                depth -= 1;
                pos += 1;
                if depth == 0 {
                    return pos;
                }
            }
            _ => {
                pos += 1;
            }
        }
    }
    len
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

/// Returns `true` if `s` starts with a vendor prefix (`-<alphanumeric_or_underscore>-`),
#[inline]
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn default_options() {
        let pass = vec![
            "@charset 'UTF-8';",
            "@CHARSET 'UTF-8';",
            "@position-try --foo {}",
            "@starting-style { opacity: 0; }",
            "@container (min-width: 700px)",
            "@CONTAINER (min-width: 500px)",
            "@import url(\"fineprint.css\") print;",
            "@import 'custom.css'",
            "@import url('landscape.css') screen and (orientation:landscape);",
            "@namespace url(http://www.w3.org/1999/xhtml);",
            "@namespace prefix url(XML-namespace-URL);",
            "@media print { body { font-size: 10pt } }",
            "@media (max-width: 960px) { body { font-size: 13px } }",
            "@media screen, print { body { line-height: 1.2 } }",
            "@supports (--foo: green) { body { color: green; } }",
            "@supports ( (perspective: 10px) or (-webkit-perspective: 10px) ) { font-size: 10pt }",
            "@counter-style win-list { system: fixed; symbols: url(gold-medal.svg); suffix: ' ';}",
            "@document url(http://www.w3.org/), url-prefix(http://www.w3.org/Style/), domain(mozilla.org), regexp('https:.*')",
            "@page :left { margin-left: 4cm; }",
            "@page { @top-center { content: none } }",
            "@font-face { font-family: MyHelvetica; src: local(\"Helvetica\"), url(MgOpenModern.ttf); }",
            "@keyframes identifier { 0% { top: 0; left: 0; } 30% { top: 50px; } 68%, 100% { top: 100px; left: 100%; } }",
            "@-webkit-keyframes identifier { 0% { top: 0; left: 0; } 30% { top: 50px; } 68%, 100% { top: 100px; left: 100%; } }",
            "@viewport { min-width: 640px; max-width: 800px; }",
            "@viewport { orientation: landscape; }",
            "@counter-style winners-list { system: fixed; symbols: url(gold-medal.svg); suffix: \" \"; }",
            "@font-feature-values Font One { @styleset { nice-style: 12; } }",
            ".foo { color: red; @nest .parent & { color: blue; } }",
            "@layer framework { h1 { background: white; } }",
            "@scroll-timeline foo {}",
            "@function --foo {}",
        ];

        let fail = vec![reject(
            "@unknown-at-rule { }",
            None,
            rejected_message("@unknown-at-rule"),
            (1, 1, 1, 17),
        )];

        Tester::new(AtRuleNoUnknown::NAME, json!(true), &pass, &fail).test();
    }

    #[test]
    fn ignore_at_rules_list() {
        let pass = vec![
            "@unknown { }",
            "@my-at-rule { }",
            "@your-at-rule { }",
            "@YOUR-at-rule { }",
        ];

        let fail = vec![
            reject(
                "@uNkNoWn { }",
                None,
                rejected_message("@uNkNoWn"),
                (1, 1, 1, 9),
            ),
            reject(
                "@UNKNOWN { }",
                None,
                rejected_message("@UNKNOWN"),
                (1, 1, 1, 9),
            ),
            reject(
                "@unknown-at-rule { }",
                None,
                rejected_message("@unknown-at-rule"),
                (1, 1, 1, 17),
            ),
            reject(
                "@unknown { @unknown-at-rule { font-size: 14px; } }",
                None,
                rejected_message("@unknown-at-rule"),
                (1, 12, 1, 28),
            ),
            reject(
                "@MY-other-at-rule { }",
                None,
                rejected_message("@MY-other-at-rule"),
                (1, 1, 1, 18),
            ),
            reject(
                "@not-my-at-rule {}",
                None,
                rejected_message("@not-my-at-rule"),
                (1, 1, 1, 16),
            ),
        ];

        Tester::new(
            AtRuleNoUnknown::NAME,
            json!([true, { "ignoreAtRules": ["unknown", "/^my-/", "/^YOUR-/i"] }]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn ignore_at_rules_regex() {
        let pass = vec!["@my-at-rule { }"];

        let fail = vec![reject(
            "@unknown-at-rule { }",
            None,
            rejected_message("@unknown-at-rule"),
            (1, 1, 1, 17),
        )];

        Tester::new(
            AtRuleNoUnknown::NAME,
            json!([true, { "ignoreAtRules": ["/^my-/"] }]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn less_syntax() {
        let pass = vec![
            "\n        .some-mixin() { margin: 0; }\n        span { .some-mixin(); }\n      ",
            "\n        @my-variable: #f7f8f9;\n        span { background-color: @my-variable; }\n      ",
        ];

        Tester::new(AtRuleNoUnknown::NAME, json!(true), &pass, &[])
            .with_custom_syntax("postcss-less")
            .test();
    }

    #[test]
    fn utf8_comments_strings_and_preprocessor_edge_cases() {
        let pass = vec![
            "/* Привет мир 🎨 @unknown {} */ a { content: \"@unknown\"; }",
            "@mixin box { @content; }",
            "@#{$unknown} { color: red; }",
            "@unknown-#{$suffix} { color: red; }",
            "#{$selector} { @media print { color: red; } }",
        ];

        let fail = vec![reject_warnings(
            "/* Привет 🎨 */ @unknown(foo) { a { content: \"✓\"; @another-unknown bar; } }",
            None,
            vec![
                (rejected_message("@unknown"), (1, 16, 1, 24)),
                (rejected_message("@another-unknown"), (1, 50, 1, 66)),
            ],
        )];

        Tester::new(
            AtRuleNoUnknown::NAME,
            json!([true, { "ignoreAtRules": ["mixin"] }]),
            &pass,
            &fail,
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
