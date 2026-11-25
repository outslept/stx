//! Rule `at-rule-no-unknown`: disallow unknown at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-no-unknown>

use std::ops::Deref;

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::{AT_KEYWORDS, in_set_ignore_case};

#[derive(Default)]
pub struct AtRuleNoUnknownConfig {
    /// Compiled matchers for the `ignoreAtRules` secondary option.
    ignore_at_rules: Vec<OptionMatcher>,
}

crate::define_stx_lint! {
    /// Flag at-rule names that are not recognized CSS keywords.
    ///
    /// Enable with `true`. Use the secondary `ignoreAtRules` list for custom names
    /// or slash-delimited regexes, matched without `@`. Vendor-prefixed names,
    /// `@charset`, bare blockless at-words, and Less variable or mixin syntax are skipped.
    ///
    /// ```css
    /// @medai print { a { color: black; } } /* reported */
    /// @media print { a { color: black; } } /* accepted */
    /// ```
    "at-rule-no-unknown", correctness, recommended = true;
    #[derive(Default)]
    pub struct AtRuleNoUnknown(AtRuleNoUnknownConfig);
}

impl Deref for AtRuleNoUnknown {
    type Target = AtRuleNoUnknownConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AtRuleNoUnknown {
    /// Creates a new `at-rule-no-unknown` rule instance from secondary options.
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self(AtRuleNoUnknownConfig {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        })
    }
}

/// Formats the diagnostic message when an unknown at-rule is found.
#[inline]
fn rejected_message(at_rule: &str) -> String {
    format!("Unknown at-rule \"{at_rule}\"")
}

impl Rule for AtRuleNoUnknown {
    fn name(&self) -> &'static str {
        Self::NAME
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
                    let blockless_empty = next_non_ws >= len || byte_at(ctx, next_non_ws) == b';';
                    if !is_less_construct {
                        let name = ctx.slice_range(name_start, name_end);
                        if !options_matches(&self.ignore_at_rules, name)
                            && !has_vendor_prefix(name)
                            && !name.eq_ignore_ascii_case("charset")
                            && !blockless_empty
                            && !in_set_ignore_case(AT_KEYWORDS, name)
                        {
                            let at_name = ctx.slice_range(at_start, name_end);
                            ctx.report(
                                Self::NAME,
                                Span {
                                    start: at_start,
                                    end: name_end,
                                },
                                rejected_message(at_name),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
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
}
