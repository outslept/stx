//! Disallows invalid preludes for at-rules (`at-rule-prelude-no-invalid`).
//!
//! This rule is a 1:1 behavioral port of Stylelint's official
//! [`at-rule-prelude-no-invalid`](https://stylelint.io/user-guide/rules/at-rule-prelude-no-invalid)
//! rule.
//!
//! # Checked Constructs
//!
//! Standard CSS at-rules with non-empty preludes are validated against their
//! W3C / `css-tree` specification grammars (`lexer.matchAtrulePrelude(name, params)`):
//!
//! 1. **Validated at-rules** (case-insensitive):
//!    - `@layer`: `<layer-name>#` where `<layer-name> = <custom-ident> ('.' <custom-ident>)*`
//!    - `@property`: `<custom-property-name>#` (`<dashed-ident>#`)
//!    - `@scope`: `[ ( <scope-start> ) ]? [ to ( <scope-end> ) ]?`
//!    - `@font-palette-values`, `@position-try`: `<dashed-ident>`
//!    - `@color-profile`: `<dashed-ident> | device-cmyk`
//!    - `@counter-style`: `<counter-style-name>` (`<custom-ident>` excluding `none`)
//!    - `@font-feature-values`: `<family-name>#` (`<string> | <custom-ident>+`)
//!    - `@keyframes` (and vendor-prefixed variants): `<keyframes-name>` (`<custom-ident> | <string>`)
//!    - `@charset`: `<string>`
//!
//! 2. **Ignored constructs**:
//!    - Unknown or custom at-rules without a prelude grammar in `css-tree`
//!      (where `lexer.matchAtrulePrelude` returns `SyntaxReferenceError` rather
//!      than `SyntaxMatchError`).
//!    - Non-standard preprocessor constructs (`isStandardSyntaxAtRule`) and
//!      preludes containing SCSS/Less interpolation (`#{$var}`, `@{var}`).

use std::ops::Deref;

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::has_interpolation;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::is_css_wide_keyword;

/// Configuration for [`AtRulePreludeNoInvalid`].
#[derive(Debug, Default)]
pub struct AtRulePreludeNoInvalidConfig {
    /// Compiled matchers from `{ "ignoreAtRules": [...] }`.
    ignore_at_rules: Vec<OptionMatcher>,
}

crate::define_stx_lint! {
    /// Disallow invalid preludes for at-rules.
    ///
    /// Validates at-rule preludes against W3C / `css-tree` grammars (such as
    /// `@layer`, `@property`, `@scope`, `@font-palette-values`, `@position-try`,
    /// `@color-profile`, `@counter-style`, and `@font-feature-values`).
    "at-rule-prelude-no-invalid", correctness, recommended = true,
    short_description = "Disallow invalid preludes for at-rules.";
    #[derive(Debug, Default)]
    pub struct AtRulePreludeNoInvalid(AtRulePreludeNoInvalidConfig);
}

impl Deref for AtRulePreludeNoInvalid {
    type Target = AtRulePreludeNoInvalidConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Formats the diagnostic message matching Stylelint's `messages.rejected`.
#[inline]
#[must_use]
fn rejected(at_rule: &str, prelude: &str) -> String {
    format!("Invalid prelude \"{prelude}\" for at-rule \"{at_rule}\"")
}

impl AtRulePreludeNoInvalid {
    /// Constructs a configured [`AtRulePreludeNoInvalid`] rule instance from raw
    /// Stylelint secondary options.
    #[must_use]
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self(AtRulePreludeNoInvalidConfig {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        })
    }
}

impl Rule for AtRulePreludeNoInvalid {
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
                        let header_end = find_at_rule_header_end(bytes, name_end);
                        let has_block = bytes.get(header_end) == Some(&b'{');

                        if is_standard_at_rule_header(bytes, name_end, header_end, has_block) {
                            let at_start_u32 = u32::try_from(at_start).unwrap_or(u32::MAX);
                            let name_start_u32 = u32::try_from(name_start).unwrap_or(u32::MAX);
                            let name_end_u32 = u32::try_from(name_end).unwrap_or(u32::MAX);
                            let name = ctx.slice_range(name_start_u32, name_end_u32);

                            let (prelude_start, prelude_end) =
                                trim_prelude_range(bytes, name_end, header_end);
                            let prelude_start_u32 =
                                u32::try_from(prelude_start).unwrap_or(u32::MAX);
                            let prelude_end_u32 = u32::try_from(prelude_end).unwrap_or(u32::MAX);
                            let raw_params = ctx.slice_range(prelude_start_u32, prelude_end_u32);
                            let clean_params = strip_comments(raw_params);
                            let clean_prelude = clean_params.trim();

                            // In Stylelint, `lexer.matchAtrulePrelude(name, params)` only reports
                            // `SyntaxMatchError` when `params` is non-empty; an empty prelude either
                            // matches or returns a non-`SyntaxMatchError` error.
                            if !clean_prelude.is_empty()
                                && !has_interpolation(name)
                                && !has_interpolation(raw_params)
                                && !options_matches(&self.ignore_at_rules, name)
                                && !is_valid_at_rule_prelude(
                                    &name.to_ascii_lowercase(),
                                    clean_prelude,
                                )
                            {
                                let at_name = ctx.slice_range(at_start_u32, name_end_u32);
                                let span = Span {
                                    start: prelude_start_u32,
                                    end: prelude_end_u32,
                                };
                                ctx.report(Self::NAME, span, rejected(at_name, raw_params));
                            }
                        }

                        at_statement_start = false;
                        pos = header_end;
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

/// Checks whether the at-rule header in `name_end..header_end` belongs to a
/// standard-syntax CSS/SCSS at-rule (`isStandardSyntaxAtRule` in Stylelint).
#[must_use]
fn is_standard_at_rule_header(
    bytes: &[u8],
    name_end: usize,
    header_end: usize,
    has_block: bool,
) -> bool {
    // `@name:` is a Less variable definition (`atRule.variable`), not a standard at-rule.
    if bytes.get(name_end) == Some(&b':') {
        return false;
    }

    // Interpolation attached directly to the at-rule name (`@layer-#{$var}`).
    if bytes.get(name_end) == Some(&b'$')
        || (bytes.get(name_end) == Some(&b'#') && bytes.get(name_end + 1) == Some(&b'{'))
        || (bytes.get(name_end) == Some(&b'@') && bytes.get(name_end + 1) == Some(&b'{'))
    {
        return false;
    }

    let has_immediate_paren = bytes.get(name_end) == Some(&b'(');
    let (prelude_start, prelude_end) = trim_prelude_range(bytes, name_end, header_end);
    let has_params = prelude_end > prelude_start;

    if has_params && bytes.get(prelude_start) == Some(&b':') {
        return false;
    }

    if !has_block && (!has_params || has_immediate_paren) {
        return false;
    }

    true
}

/// Validates a non-empty `prelude` against the W3C / `css-tree` specification
/// grammar for `at_rule_lower`.
///
/// Returns `true` if valid or if `at_rule_lower` has no prelude grammar in
/// `css-tree` (in which case `css-tree` returns `SyntaxReferenceError` rather
/// than `SyntaxMatchError`).
#[must_use]
fn is_valid_at_rule_prelude(at_rule_lower: &str, prelude: &str) -> bool {
    match at_rule_lower {
        "layer" => is_valid_layer_prelude(prelude),
        "property" => is_valid_property_prelude(prelude),
        "scope" => is_valid_scope_prelude(prelude),
        "font-palette-values" | "position-try" => is_dashed_ident(prelude),
        "color-profile" => prelude.eq_ignore_ascii_case("device-cmyk") || is_dashed_ident(prelude),
        "counter-style" => is_custom_ident(prelude) && !prelude.eq_ignore_ascii_case("none"),
        "font-feature-values" => is_valid_font_feature_values_prelude(prelude),
        "keyframes" | "-webkit-keyframes" | "-moz-keyframes" | "-o-keyframes" => {
            is_custom_ident(prelude) || is_quoted_string(prelude)
        }
        "charset" => is_quoted_string(prelude),
        _ => true,
    }
}

/// `@layer`: empty prelude or `<layer-name>#`, where `<layer-name> = <custom-ident> ('.' <custom-ident>)*`.
#[must_use]
fn is_valid_layer_prelude(prelude: &str) -> bool {
    if prelude.is_empty() {
        return true;
    }
    for item in prelude.split(',') {
        let item = item.trim();
        if item.is_empty() {
            return false;
        }
        for segment in item.split('.') {
            if !is_custom_ident(segment) {
                return false;
            }
        }
    }
    true
}

/// `@property`: `<custom-property-name>#` (one or more comma-separated `<dashed-ident>`s).
#[must_use]
fn is_valid_property_prelude(prelude: &str) -> bool {
    if prelude.is_empty() {
        return false;
    }
    for item in prelude.split(',') {
        if !is_dashed_ident(item.trim()) {
            return false;
        }
    }
    true
}

/// `@scope`: `[ ( <scope-start> ) ]? [ to ( <scope-end> ) ]?`.
#[must_use]
fn is_valid_scope_prelude(prelude: &str) -> bool {
    let mut s = prelude.trim();
    if s.is_empty() {
        return true;
    }

    // Optional `(<scope-start>)`
    if s.starts_with('(') {
        let Some(after_first) = consume_parenthesized_group(s) else {
            return false;
        };
        s = after_first.trim_start();
        if s.is_empty() {
            return true;
        }
    }

    // Optional `to (<scope-end>)`
    if s.len() >= 2 && s[..2].eq_ignore_ascii_case("to") {
        let after_to = s[2..].trim_start();
        if after_to.starts_with('(')
            && let Some(rest) = consume_parenthesized_group(after_to)
        {
            return rest.trim().is_empty();
        }
    }

    false
}

/// Consumes a balanced `(...)` group from the start of `s`, returning the remaining slice.
#[must_use]
fn consume_parenthesized_group(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.first().copied() != Some(b'(') {
        return None;
    }
    let mut depth: u32 = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' || b == b'\'' {
            i = skip_quoted_string(bytes, i + 1, b);
            continue;
        }
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(&s[i + 1..]);
            }
        }
        i += 1;
    }
    None
}

/// `@font-feature-values`: `<family-name>#`, where `<family-name> = <string> | <custom-ident>+`.
#[must_use]
fn is_valid_font_feature_values_prelude(prelude: &str) -> bool {
    if prelude.is_empty() {
        return false;
    }
    for family in prelude.split(',') {
        let family = family.trim();
        if family.is_empty() {
            return false;
        }
        if is_quoted_string(family) {
            continue;
        }
        let mut count = 0;
        for word in family.split_ascii_whitespace() {
            if !is_custom_ident(word) {
                return false;
            }
            count += 1;
        }
        if count == 0 {
            return false;
        }
    }
    true
}

/// Returns `true` if `s` is a valid `<dashed-ident>` (starts with `--` and is a valid CSS ident).
#[inline]
#[must_use]
fn is_dashed_ident(s: &str) -> bool {
    s.len() > 2 && s.starts_with("--") && is_css_ident(s)
}

/// Returns `true` if `s` is a valid `<custom-ident>` (valid CSS ident and not a CSS-wide keyword).
#[inline]
#[must_use]
fn is_custom_ident(s: &str) -> bool {
    is_css_ident(s) && !is_css_wide_keyword(s)
}

/// Returns `true` if `s` is a valid CSS identifier.
#[must_use]
fn is_css_ident(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    if bytes[0] == b'-' {
        if bytes.len() < 2 || bytes[1].is_ascii_digit() {
            return false;
        }
    } else if !is_ident_start(bytes[0]) {
        return false;
    }
    bytes.iter().copied().all(is_ident_char)
}

/// Returns `true` if `s` is a single properly quoted string literal (`"..."` or `'...'`).
#[must_use]
fn is_quoted_string(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    let quote = bytes[0];
    if quote != b'"' && quote != b'\'' {
        return false;
    }
    let mut i = 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == quote {
            return i + 1 == bytes.len();
        }
        i += 1;
    }
    false
}

/// Strips block comments (`/* ... */`) from `input`, preserving quoted strings.
#[must_use]
fn strip_comments(input: &str) -> String {
    if !input.contains("/*") {
        return input.to_string();
    }
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut pos = 0;
    let mut chunk_start = 0;
    while pos < len {
        let b = bytes[pos];
        if b == b'"' || b == b'\'' {
            pos = skip_quoted_string(bytes, pos + 1, b);
            continue;
        }
        if b == b'/' && bytes.get(pos + 1) == Some(&b'*') {
            out.push_str(&input[chunk_start..pos]);
            pos = skip_block_comment(bytes, pos + 2);
            chunk_start = pos;
            continue;
        }
        pos += 1;
    }
    out.push_str(&input[chunk_start..len]);
    out
}

/// Finds the end of an at-rule header (the position of `{`, `;`, `}`, or EOF).
#[must_use]
fn find_at_rule_header_end(bytes: &[u8], mut pos: usize) -> usize {
    let len = bytes.len();
    let mut paren_depth: u32 = 0;
    while pos < len {
        let b = bytes[pos];
        // Skip block comments.
        if b == b'/' && bytes.get(pos + 1) == Some(&b'*') {
            pos = skip_block_comment(bytes, pos + 2);
            continue;
        }
        // Skip single-line comments.
        if b == b'/' && bytes.get(pos + 1) == Some(&b'/') && (pos == 0 || bytes[pos - 1] != b':') {
            pos = skip_line_comment(bytes, pos + 2);
            continue;
        }
        // Skip quoted strings.
        if b == b'"' || b == b'\'' {
            pos = skip_quoted_string(bytes, pos + 1, b);
            continue;
        }
        if b == b'(' {
            paren_depth += 1;
        } else if b == b')' {
            paren_depth = paren_depth.saturating_sub(1);
        } else if paren_depth == 0 && matches!(b, b'{' | b';' | b'}') {
            return pos;
        }
        pos += 1;
    }
    len
}

/// Trims leading (`raws.afterName`) and trailing (`raws.between`) ASCII
/// whitespace and block comments from `start..end`, matching PostCSS `atRule.params`
/// and `atRuleParamIndex(atRule)`.
#[must_use]
fn trim_prelude_range(bytes: &[u8], mut start: usize, mut end: usize) -> (usize, usize) {
    // Trim leading whitespace and block comments (`raws.afterName`).
    loop {
        while start < end && bytes[start].is_ascii_whitespace() {
            start += 1;
        }
        if start + 1 < end && bytes[start] == b'/' && bytes[start + 1] == b'*' {
            let comment_end = skip_block_comment(bytes, start + 2);
            if comment_end <= end {
                start = comment_end;
                continue;
            }
        }
        break;
    }

    // Scan through `start..end` to find the end of the last non-whitespace,
    // non-comment token (excluding `raws.between`).
    let mut pos = start;
    let mut last_token_end = start;
    while pos < end {
        let b = bytes[pos];
        if b.is_ascii_whitespace() {
            pos += 1;
            continue;
        }
        if b == b'/' && pos + 1 < end && bytes[pos + 1] == b'*' {
            pos = skip_block_comment(bytes, pos + 2).min(end);
            continue;
        }
        if b == b'"' || b == b'\'' {
            pos = skip_quoted_string(bytes, pos + 1, b).min(end);
            last_token_end = pos;
            continue;
        }
        pos += 1;
        last_token_end = pos;
    }
    end = last_token_end;
    (start, end)
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
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn default_options() {
        let pass = vec![
            "@layer;",
            "@layer foo;",
            "@media (min-width: 100px) {}",
            "@counter-style foo {}",
            "@property --foo {}",
            "@property --foo, --bar {}",
            "@foo bar;",
        ];

        let fail = vec![
            reject(
                "@layer \"foo\";",
                None,
                rejected("@layer", "\"foo\""),
                (1, 8, 1, 13),
            ),
            reject(
                "@property  foo {}",
                None,
                rejected("@property", "foo"),
                (1, 12, 1, 15),
            ),
            reject(
                "@scope .foo {}",
                None,
                rejected("@scope", ".foo"),
                (1, 8, 1, 12),
            ),
            reject(
                "@font-palette-values foo {}",
                None,
                rejected("@font-palette-values", "foo"),
                (1, 22, 1, 25),
            ),
            reject(
                "@font-feature-values #foo {}",
                None,
                rejected("@font-feature-values", "#foo"),
                (1, 22, 1, 26),
            ),
        ];

        Tester::new(AtRulePreludeNoInvalid::NAME, json!(true), &pass, &fail).test();
    }

    #[test]
    fn ignore_at_rules() {
        let pass = vec![
            "@property foo;",
            "@font-palette-values foo {}",
            "@font-feature-values #foo {}",
        ];

        Tester::new(
            AtRulePreludeNoInvalid::NAME,
            json!([true, { "ignoreAtRules": ["property", "/^font-/"] }]),
            &pass,
            &[],
        )
        .test();
    }

    #[test]
    fn scope_color_profile_counter_style_and_comments() {
        let pass = vec![
            "@layer foo.bar, baz.qux;",
            "@layer foo /* comment */, bar;",
            "@scope (.card) to (.content) {}",
            "@scope (.card) {}",
            "@scope to (.content) {}",
            "@scope {}",
            "@color-profile --swop5c {}",
            "@color-profile device-cmyk {}",
            "@position-try --custom-pos {}",
            "@font-feature-values \"Font One\", Font Two {}",
            "/* Привет 🎨 */ @property --моё-свойство {}",
        ];

        let fail = vec![
            reject(
                "@counter-style none {}",
                None,
                rejected("@counter-style", "none"),
                (1, 16, 1, 20),
            ),
            reject(
                "@color-profile swop5c {}",
                None,
                rejected("@color-profile", "swop5c"),
                (1, 16, 1, 22),
            ),
            reject(
                "@position-try custom-pos {}",
                None,
                rejected("@position-try", "custom-pos"),
                (1, 15, 1, 25),
            ),
            reject(
                "/* Привет 🎨 */ @property /* comment */ foo /* comment */ {}",
                None,
                rejected("@property", "foo"),
                (1, 40, 1, 43),
            ),
        ];

        Tester::new(AtRulePreludeNoInvalid::NAME, json!(true), &pass, &fail).test();
    }

    #[test]
    fn scss_and_less_non_standard_constructs() {
        let scss_pass = vec!["@property #{$name} {}", "@layer #{$layer-name};"];
        Tester::new(AtRulePreludeNoInvalid::NAME, json!(true), &scss_pass, &[])
            .with_custom_syntax("postcss-scss")
            .test();

        let less_pass = vec!["@property: 10px;\n.box { width: @property; }", "@layer();"];
        Tester::new(AtRulePreludeNoInvalid::NAME, json!(true), &less_pass, &[])
            .with_custom_syntax("postcss-less")
            .test();
    }
}
