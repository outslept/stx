//! Specifies a list of disallowed at-rules (`at-rule-disallowed-list`).
//!
//! # Checked Constructs
//!
//! 1. Standard-syntax CSS and SCSS at-rules at the root level or nested inside style and
//!    at-rule blocks.
//! 2. At-rule names (without the leading `@`) are matched against the configured primary
//!    option list using `matchesStringOrRegExp` (exact case-sensitive string comparison or
//!    regular expression matching).
//! 3. Non-standard preprocessor constructs—such as Less variable definitions (`@var: 10px`),
//!    Less detached rulesets and mixin calls (`@mixin()`), parameterless blockless directives
//!    (`@content;`), and at-rules with interpolated names (`@#{$var}`, `@prefix-#{$var}`)—are
//!    ignored via `isStandardSyntaxAtRule`.

use std::ops::Deref;

use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;

use crate::config::RuleSetting;
use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::{OptionMatcher, options_matches, parse_primary_option_matchers};

/// Configuration for [`AtRuleDisallowedList`].
#[derive(Debug, Default)]
pub struct AtRuleDisallowedListConfig {
    /// Compiled matchers for disallowed at-rule names (without the leading `@`).
    disallowed: Vec<OptionMatcher>,
}

crate::define_stx_lint! {
    /// Specifies a list of disallowed at-rules.
    ///
    /// Reports any standard-syntax at-rule whose name (without the leading `@`) matches one
    /// of the strings or regular expressions configured in the primary option.
    "at-rule-disallowed-list", restriction,
    short_description = "Specify a list of disallowed at-rules.";

    #[derive(Debug, Default)]
    pub struct AtRuleDisallowedList(AtRuleDisallowedListConfig);
}

impl Deref for AtRuleDisallowedList {
    type Target = AtRuleDisallowedListConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AtRuleDisallowedList {
    /// Creates a new [`AtRuleDisallowedList`] rule instance from the given
    /// [`RuleSetting`].
    ///
    /// Returns `None` if no valid disallowed matchers are configured.
    #[must_use]
    pub fn from_setting(setting: &RuleSetting) -> Option<Self> {
        let disallowed = parse_primary_option_matchers(&setting.primary);
        if disallowed.is_empty() {
            return None;
        }
        Some(Self(AtRuleDisallowedListConfig { disallowed }))
    }
}

/// Formats the diagnostic message when an `at_rule` (including the leading
/// `@`) matches the disallowed list.
#[inline]
#[must_use]
fn rejected_message(at_rule: &str) -> String {
    format!("Disallowed at-rule \"{at_rule}\"")
}

impl Rule for AtRuleDisallowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let bytes = ctx.source.as_bytes();
        let len = bytes.len();
        let mut pos: usize = 0;

        let mut at_statement_start = true;
        let mut paren_depth: u32 = 0;

        while pos < len {
            let b = bytes[pos];

            // Skip ASCII whitespace.
            if b.is_ascii_whitespace() {
                pos += 1;
                continue;
            }

            // Skip block comments `/* ... */`.
            if b == b'/' && bytes.get(pos + 1) == Some(&b'*') {
                pos = skip_block_comment(bytes, pos + 2);
                continue;
            }

            // Skip single-line comments `// ...` in SCSS/Less when not part of `://`.
            if b == b'/'
                && bytes.get(pos + 1) == Some(&b'/')
                && (pos == 0 || bytes[pos - 1] != b':')
            {
                pos = skip_line_comment(bytes, pos + 2);
                continue;
            }

            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                pos = skip_quoted_string(bytes, pos + 1, b);
                at_statement_start = false;
                continue;
            }

            // Skip SCSS `#{...}` interpolation blocks so their closing `}` does not
            // falsely reset `at_statement_start`.
            if b == b'#' && bytes.get(pos + 1) == Some(&b'{') {
                pos = skip_interpolation(bytes, pos + 2);
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
                    let name_end = scan_ident_end(bytes, name_start);

                    if name_end > name_start {
                        let start_u32 = u32::try_from(at_pos).unwrap_or(u32::MAX);
                        let name_start_u32 = u32::try_from(name_start).unwrap_or(u32::MAX);
                        let end_u32 = u32::try_from(name_end).unwrap_or(u32::MAX);

                        let name = ctx.slice_range(name_start_u32, end_u32);

                        if is_standard_at_rule_after_name(bytes, name_end)
                            && options_matches(&self.disallowed, name)
                        {
                            let at_name = ctx.slice_range(start_u32, end_u32);
                            ctx.report(
                                Self::NAME,
                                Span {
                                    start: start_u32,
                                    end: end_u32,
                                },
                                rejected_message(at_name),
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
#[inline]
#[must_use]
fn scan_ident_end(bytes: &[u8], start: usize) -> usize {
    let mut pos = start;
    while let Some(&b) = bytes.get(pos) {
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
/// detached ruleset call, mixin call, parameterless `@content;`, or an at-rule
/// with interpolation in its name.
#[must_use]
fn is_standard_at_rule_after_name(bytes: &[u8], name_end: usize) -> bool {
    let len = bytes.len();

    // `@name:` is a Less variable definition (`atRule.variable`), not a standard at-rule.
    if bytes.get(name_end) == Some(&b':') {
        return false;
    }

    // Interpolation attached directly to the at-rule name (e.g. `@prefix-#{$var}`,
    // `@prefix@{var}`, `@prefix$var`) means `hasInterpolation(atRule.name)` is true.
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
    // 1. `if (!atRule.nodes && atRule.params === '') return false;` (e.g. `@content;`)
    // 2. `if (!atRule.nodes && atRule.raws.afterName === '' && atRule.params[0] === '(') return false;`
    //    (e.g. Less detached ruleset call `@detached-ruleset();`)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn array_config() {
        let pass = vec!["a { color: pink; }", "@mixin name ($p) {}"];
        let fail = vec![
            reject(
                "a { @extend %placeholder; }",
                None,
                rejected_message("@extend"),
                (1, 5, 1, 12),
            ),
            reject(
                "\n      a {\n        @extend\n        %placeholder;\n      }\n    ",
                None,
                rejected_message("@extend"),
                (3, 9, 3, 16),
            ),
            reject(
                "\n      @keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
                None,
                rejected_message("@keyframes"),
                (2, 7, 2, 17),
            ),
            reject(
                "\n      @Keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
                None,
                rejected_message("@Keyframes"),
                (2, 7, 2, 17),
            ),
            reject(
                "\n      @-moz-keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
                None,
                rejected_message("@-moz-keyframes"),
                (2, 7, 2, 22),
            ),
            reject(
                "\n      @-WEBKIT-KEYFRAMES name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
                None,
                rejected_message("@-WEBKIT-KEYFRAMES"),
                (2, 7, 2, 25),
            ),
        ];

        Tester::new(
            AtRuleDisallowedList::NAME,
            json!(["extend", "supports", "/^(-webkit-|-moz-)?keyframes$/i"]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn string_regex_config() {
        let pass = vec!["a { color: pink; }", "@mixin name ($p) {}"];
        let fail = vec![
            reject(
                "\n      @keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
                None,
                rejected_message("@keyframes"),
                (2, 7, 2, 17),
            ),
            reject(
                "\n      @Keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
                None,
                rejected_message("@Keyframes"),
                (2, 7, 2, 17),
            ),
            reject(
                "\n      @-moz-keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
                None,
                rejected_message("@-moz-keyframes"),
                (2, 7, 2, 22),
            ),
            reject(
                "\n      @-WEBKIT-KEYFRAMES name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
                None,
                rejected_message("@-WEBKIT-KEYFRAMES"),
                (2, 7, 2, 25),
            ),
        ];

        Tester::new(
            AtRuleDisallowedList::NAME,
            json!("/^(-webkit-|-moz-)?keyframes$/i"),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn less_syntax() {
        let pass =
            vec!["\n        .keyframes() { margin: 0; }\n        span { .keyframes(); }\n      "];
        let fail = vec![];

        Tester::new(
            AtRuleDisallowedList::NAME,
            json!(["keyframes"]),
            &pass,
            &fail,
        )
        .with_custom_syntax("postcss-less")
        .test();
    }

    #[test]
    fn single_string_option_and_case_sensitivity() {
        let pass = vec![
            "@media (min-width: 700px) { a { color: red; } }",
            "@IMPORT \"foo.css\";",
        ];
        let fail = vec![reject(
            "@import \"foo.css\";",
            None,
            rejected_message("@import"),
            (1, 1, 1, 8),
        )];

        Tester::new(AtRuleDisallowedList::NAME, json!("import"), &pass, &fail).test();
    }

    #[test]
    fn less_variables_and_interpolated_at_rules() {
        let pass = vec![
            "@keyframes: 10px;\n.box { margin: @keyframes; }",
            "@{keyframes} { .box { margin: 0; } }",
        ];
        let fail = vec![reject(
            "@keyframes fade { from { opacity: 0; } to { opacity: 1; } }",
            None,
            rejected_message("@keyframes"),
            (1, 1, 1, 11),
        )];

        Tester::new(
            AtRuleDisallowedList::NAME,
            json!(["keyframes"]),
            &pass,
            &fail,
        )
        .with_custom_syntax("postcss-less")
        .test();
    }

    #[test]
    fn scss_interpolation_and_content_directive() {
        let pass = vec![
            "@#{$keyframes} { a { color: red; } }",
            "@prefix-#{$keyframes} { a { color: red; } }",
            "@mixin box { @content; }",
            "// @extend %placeholder;\na { color: red; }",
        ];
        let fail = vec![
            reject(
                "a { @extend %#{$placeholder}; }",
                None,
                rejected_message("@extend"),
                (1, 5, 1, 12),
            ),
            reject(
                "@media #{$query} { a { color: red; } }",
                None,
                rejected_message("@media"),
                (1, 1, 1, 7),
            ),
        ];

        Tester::new(
            AtRuleDisallowedList::NAME,
            json!(["extend", "media", "content", "keyframes", "prefix-"]),
            &pass,
            &fail,
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }

    #[test]
    fn utf8_comments_strings_and_immediate_parens() {
        let pass = vec!["/* @extend %foo; */ a { content: \"@supports (display: grid)\"; }"];
        let fail = vec![reject_warnings(
            "/* Привет мир 🎨 */ @supports(display: grid) { a { content: \"✓\"; @extend %foo; } }",
            None,
            vec![
                (rejected_message("@supports"), (1, 20, 1, 29)),
                (rejected_message("@extend"), (1, 65, 1, 72)),
            ],
        )];

        Tester::new(
            AtRuleDisallowedList::NAME,
            json!(["extend", "supports"]),
            &pass,
            &fail,
        )
        .test();
    }
}
