//! Disallows deprecated at-rules (`at-rule-no-deprecated`).
//!
//! # Checked Constructs
//!
//! 1. Standard-syntax CSS at-rules at the root level or nested inside style/at-rule blocks.
//! 2. Deprecated at-rules (`@document`, `@nest`, `@viewport`, and their vendor-prefixed
//!    variants such as `@-moz-document` and `@-ms-viewport`) are matched case-insensitively
//!    against Stylelint's `deprecatedAtKeywords` reference set.
//! 3. When `@nest` is used with a block (e.g., `a { @nest .foo & {} }`), an autofix and
//!    `EditInfo` remove `@nest` and its trailing `raws.afterName` whitespace to convert it
//!    into a standard nested style rule.
//! 4. Non-standard preprocessor constructs—such as Less variable definitions
//!    (`@viewport: 100vw`), Less mixin calls (`@document()`), parameterless blockless
//!    directives, and at-rules with interpolated names (`@nest-#{$var}`)—are ignored via
//!    `isStandardSyntaxAtRule`.

use std::ops::Deref;

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::{Fix, LintContext};
use crate::rules::Rule;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::{DEPRECATED_AT_KEYWORDS, in_set_ignore_case};

/// Configuration for [`AtRuleNoDeprecated`].
#[derive(Debug, Default)]
pub struct AtRuleNoDeprecatedConfig {
    /// Compiled matchers for the `ignoreAtRules` secondary option.
    ignore_at_rules: Vec<OptionMatcher>,
}

crate::define_stx_lint! {
    /// Disallows deprecated at-rules.
    ///
    /// Reports any standard-syntax at-rule in Stylelint's `deprecatedAtKeywords` set
    /// (`@document`, `@nest`, `@viewport`, and their vendor-prefixed forms) unless exempted
    /// by `ignoreAtRules`. Provides an autofix for `@nest` rules by converting them into
    /// standard nested style rules.
    "at-rule-no-deprecated", correctness, fix = conditional, recommended = true,
    short_description = "Disallow deprecated at-rules.";

    #[derive(Debug, Default)]
    pub struct AtRuleNoDeprecated(AtRuleNoDeprecatedConfig);
}

impl Deref for AtRuleNoDeprecated {
    type Target = AtRuleNoDeprecatedConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AtRuleNoDeprecated {
    /// Creates a new [`AtRuleNoDeprecated`] rule instance from secondary options.
    #[must_use]
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self(AtRuleNoDeprecatedConfig {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        })
    }
}

/// Formats the diagnostic message when `at_rule` (including `@`) is deprecated.
#[inline]
#[must_use]
fn rejected_message(at_rule: &str) -> String {
    format!("Deprecated at-rule \"{at_rule}\"")
}

impl Rule for AtRuleNoDeprecated {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let bytes = ctx.source.as_bytes();
        let len = bytes.len();
        let mut pos: usize = 0;

        // Track whether the scanner is positioned where a statement / at-rule may start
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

                        if let Some(has_block) = is_standard_at_rule_after_name(bytes, name_end) {
                            let at_start_u32 = u32::try_from(at_start).unwrap_or(u32::MAX);
                            let name_start_u32 = u32::try_from(name_start).unwrap_or(u32::MAX);
                            let name_end_u32 = u32::try_from(name_end).unwrap_or(u32::MAX);

                            let name = ctx.slice_range(name_start_u32, name_end_u32);

                            if !options_matches(&self.ignore_at_rules, name)
                                && in_set_ignore_case(DEPRECATED_AT_KEYWORDS, name)
                            {
                                let at_name = ctx.slice_range(at_start_u32, name_end_u32);
                                let span = Span {
                                    start: at_start_u32,
                                    end: name_end_u32,
                                };

                                // `@nest` with a block is autofixable by stripping `@nest` and
                                // its trailing `raws.afterName` whitespace/comments.
                                let fix = if name.eq_ignore_ascii_case("nest") && has_block {
                                    let fix_end = skip_after_name(bytes, name_end);
                                    let fix_end_u32 = u32::try_from(fix_end).unwrap_or(u32::MAX);
                                    Some(Fix::single(
                                        Span {
                                            start: at_start_u32,
                                            end: fix_end_u32,
                                        },
                                        String::new(),
                                    ))
                                } else {
                                    None
                                };

                                ctx.report_with_fix(
                                    Self::NAME,
                                    span,
                                    rejected_message(at_name),
                                    fix,
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
///
/// Returns `Some(has_block)` if the at-rule is standard syntax, or `None` if it
/// should be ignored.
#[must_use]
fn is_standard_at_rule_after_name(bytes: &[u8], name_end: usize) -> Option<bool> {
    let len = bytes.len();

    // `@name:` is a Less variable definition (`atRule.variable`), not a standard at-rule.
    if bytes.get(name_end) == Some(&b':') {
        return None;
    }

    // Interpolation attached directly to the at-rule name (`@nest#{$var}`, `@nest@{var}`).
    if bytes.get(name_end) == Some(&b'$')
        || (bytes.get(name_end) == Some(&b'#') && bytes.get(name_end + 1) == Some(&b'{'))
        || (bytes.get(name_end) == Some(&b'@') && bytes.get(name_end + 1) == Some(&b'{'))
    {
        return None;
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
        return None;
    }

    let has_block = bytes.get(pos) == Some(&b'{');

    // In Stylelint's `isStandardSyntaxAtRule`:
    // 1. `if (!atRule.nodes && atRule.params === '') return false;`
    // 2. `if (!atRule.nodes && atRule.raws.afterName === '' && atRule.params[0] === '(') return false;`
    if !has_block && (!has_params || has_immediate_paren) {
        return None;
    }

    Some(has_block)
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

/// Skips `raws.afterName` (ASCII whitespace and block comments) immediately
/// following `@nest`.
#[inline]
#[must_use]
fn skip_after_name(bytes: &[u8], start: usize) -> usize {
    let len = bytes.len();
    let mut pos = start;
    while pos < len {
        if bytes[pos].is_ascii_whitespace() {
            pos += 1;
        } else if bytes[pos] == b'/' && bytes.get(pos + 1) == Some(&b'*') {
            pos = skip_block_comment(bytes, pos + 2);
        } else {
            break;
        }
    }
    pos
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
    use crate::tester::{Tester, reject, reject_edit, reject_edit_warnings, reject_warnings};
    use serde_json::json;

    #[test]
    fn default_options() {
        let pass = vec![
            "@position-try --foo {}",
            "@starting-style {}",
            "@container (min-width: 700px) {}",
            "@CONTAINER (min-width: 500px) {}",
            "a { @apply --foo; }",
        ];

        let fail = vec![
            reject(
                "@document url(http://www.w3.org/);",
                None,
                rejected_message("@document"),
                (1, 1, 1, 10),
            ),
            reject(
                "@VIEWPORT { orientation: landscape; }",
                None,
                rejected_message("@VIEWPORT"),
                (1, 1, 1, 10),
            ),
            reject(
                "a { @nest .foo & {} }",
                "a { .foo & {} }",
                rejected_message("@nest"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { @NEST .foo & {} }",
                "a { .foo & {} }",
                rejected_message("@NEST"),
                (1, 5, 1, 10),
            ),
            reject(
                "a { @nest .foo /* comment */ & {} }",
                "a { .foo /* comment */ & {} }",
                rejected_message("@nest"),
                (1, 5, 1, 10),
            ),
            reject_warnings(
                "a { @nest .foo {} @nest .bar {} }",
                "a { .foo {} .bar {} }",
                vec![
                    (rejected_message("@nest"), (1, 5, 1, 10)),
                    (rejected_message("@nest"), (1, 19, 1, 24)),
                ],
            ),
        ];

        Tester::new(AtRuleNoDeprecated::NAME, json!(true), &pass, &fail)
            .with_fix(true)
            .test();

        let fail_edits = vec![
            reject_edit_warnings(
                "@document url(http://www.w3.org/);",
                vec![(rejected_message("@document"), (1, 1, 1, 10), None)],
            ),
            reject_edit_warnings(
                "@VIEWPORT { orientation: landscape; }",
                vec![(rejected_message("@VIEWPORT"), (1, 1, 1, 10), None)],
            ),
            reject_edit(
                "a { @nest .foo & {} }",
                rejected_message("@nest"),
                (1, 5, 1, 10),
                (4, 10, ""),
            ),
            reject_edit(
                "a { @NEST .foo & {} }",
                rejected_message("@NEST"),
                (1, 5, 1, 10),
                (4, 10, ""),
            ),
            reject_edit(
                "a { @nest .foo /* comment */ & {} }",
                rejected_message("@nest"),
                (1, 5, 1, 10),
                (4, 10, ""),
            ),
            reject_edit_warnings(
                "a { @nest .foo {} @nest .bar {} }",
                vec![
                    (rejected_message("@nest"), (1, 5, 1, 10), Some((4, 10, ""))),
                    (
                        rejected_message("@nest"),
                        (1, 19, 1, 24),
                        Some((18, 24, "")),
                    ),
                ],
            ),
        ];

        Tester::new(AtRuleNoDeprecated::NAME, json!(true), [], &fail_edits)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn ignore_at_rules() {
        let pass = vec![
            "@document url(http://www.w3.org/);",
            "@viewport { orientation: landscape; }",
        ];

        let fail = vec![reject(
            "a { @nest .foo & {} }",
            None,
            rejected_message("@nest"),
            (1, 5, 1, 10),
        )];

        Tester::new(
            AtRuleNoDeprecated::NAME,
            json!([true, { "ignoreAtRules": ["document", "/^view/"] }]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn less_and_scss_non_standard_at_rules() {
        let pass = vec![
            "@viewport: 100vw;\n.box { width: @viewport; }",
            "@document();",
            "@{nest} { color: red; }",
        ];
        let fail = vec![reject(
            "@viewport { width: device-width; }",
            None,
            rejected_message("@viewport"),
            (1, 1, 1, 10),
        )];

        Tester::new(AtRuleNoDeprecated::NAME, json!(true), &pass, &fail)
            .with_custom_syntax("postcss-less")
            .test();

        let scss_pass = vec![
            "@nest-#{$var} { color: red; }",
            "// @viewport { width: 100px; }\na { color: red; }",
            "a { #{$sel} & { color: red; } }",
        ];
        let scss_fail = vec![reject(
            "a { @nest /* comment */ .foo & {} }",
            "a { .foo & {} }",
            rejected_message("@nest"),
            (1, 5, 1, 10),
        )];

        Tester::new(
            AtRuleNoDeprecated::NAME,
            json!(true),
            &scss_pass,
            &scss_fail,
        )
        .with_custom_syntax("postcss-scss")
        .with_fix(true)
        .test();
    }
}
