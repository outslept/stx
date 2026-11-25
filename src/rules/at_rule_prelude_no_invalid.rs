//! Rule `at-rule-prelude-no-invalid`: disallow invalid preludes for at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-prelude-no-invalid>

use std::ops::Deref;

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::has_interpolation;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::is_css_wide_keyword;

#[derive(Default)]
pub struct AtRulePreludeNoInvalidConfig {
    /// Compiled matchers for the `ignoreAtRules` secondary option.
    ignore_at_rules: Vec<OptionMatcher>,
}

crate::define_stx_lint! {
    /// Check the text between selected at-rule names and their opening brace or semicolon.
    ///
    /// Enable with `true`. The secondary `ignoreAtRules` list accepts names or
    /// slash-delimited regexes matched without `@`.
    ///
    /// This checks `@layer`, `@property`, `@scope`, `@font-palette-values`,
    /// `@position-try`, `@color-profile`, `@counter-style`, and `@font-feature-values`.
    /// It also requires empty preludes for `@font-face`, `@starting-style`,
    /// `@view-transition`, and `@viewport`. Other at-rules and interpolated preludes are skipped.
    ///
    /// ```css
    /// @font-face Body { font-family: Body; } /* reported */
    /// @font-face { font-family: Body; }      /* accepted */
    /// ```
    "at-rule-prelude-no-invalid", correctness, recommended = true;
    #[derive(Default)]
    pub struct AtRulePreludeNoInvalid(AtRulePreludeNoInvalidConfig);
}

impl Deref for AtRulePreludeNoInvalid {
    type Target = AtRulePreludeNoInvalidConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AtRulePreludeNoInvalid {
    /// Creates a new `at-rule-prelude-no-invalid` rule instance from secondary options.
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self(AtRulePreludeNoInvalidConfig {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        })
    }
}

/// Formats the diagnostic message when an invalid at-rule prelude is found.
#[inline]
fn rejected_message(at_rule: &str, prelude: &str) -> String {
    format!("Invalid prelude \"{prelude}\" for at-rule \"{at_rule}\"")
}

impl Rule for AtRulePreludeNoInvalid {
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
                i = skip_string(ctx, i, len);
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
                    let header_end = find_at_rule_header_end(ctx, name_end, len);
                    // Ignore Less variable declarations (`@var: ...`) and Less mixin calls (`@mixin(...)`).
                    let next_non_ws = skip_spaces_and_tabs(ctx, name_end, len);
                    let is_less_construct = next_non_ws < len
                        && (byte_at(ctx, next_non_ws) == b':'
                            || (next_non_ws == name_end && byte_at(ctx, next_non_ws) == b'('));
                    if !is_less_construct {
                        let name = ctx.slice_range(name_start, name_end);
                        let (prelude_start, prelude_end) =
                            trim_prelude_range(ctx, name_end, header_end);
                        let prelude = ctx.slice_range(prelude_start, prelude_end);
                        if !has_interpolation(name)
                            && !has_interpolation(prelude)
                            && !options_matches(&self.ignore_at_rules, name)
                            && !is_valid_at_rule_prelude(&name.to_ascii_lowercase(), prelude)
                        {
                            let at_name = ctx.slice_range(at_start, name_end);
                            let span = Span {
                                start: prelude_start,
                                end: prelude_end,
                            };
                            ctx.report(Self::NAME, span, rejected_message(at_name, prelude));
                        }
                    }
                    at_statement_start = false;
                    i = header_end;
                    continue;
                }
            }
            at_statement_start = false;
            i += 1;
        }
    }
}

/// Validates `prelude` against the W3C specification grammar for `at_rule_lower`.
///
/// Returns `true` if valid or if `at_rule_lower` is an unknown/custom at-rule.
fn is_valid_at_rule_prelude(at_rule_lower: &str, prelude: &str) -> bool {
    match at_rule_lower {
        "layer" => is_valid_layer_prelude(prelude),
        "property" => is_valid_property_prelude(prelude),
        "scope" => is_valid_scope_prelude(prelude),
        "font-palette-values" | "position-try" => is_dashed_ident(prelude),
        "color-profile" => prelude.eq_ignore_ascii_case("device-cmyk") || is_dashed_ident(prelude),
        "counter-style" => is_custom_ident(prelude) && !prelude.eq_ignore_ascii_case("none"),
        "font-feature-values" => is_valid_font_feature_values_prelude(prelude),
        "font-face" | "starting-style" | "view-transition" | "viewport" => prelude.is_empty(),
        _ => true,
    }
}

/// `@layer`: empty prelude or `<layer-name>#`, where `<layer-name> = <custom-ident> ('.' <custom-ident>)*`.
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
            let quote = b;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
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
fn is_dashed_ident(s: &str) -> bool {
    s.len() > 2 && s.starts_with("--") && is_css_ident(s)
}

/// Returns `true` if `s` is a valid `<custom-ident>` (valid CSS ident and not a CSS-wide keyword).
#[inline]
fn is_custom_ident(s: &str) -> bool {
    is_css_ident(s) && !is_css_wide_keyword(s)
}

/// Returns `true` if `s` is a valid CSS identifier.
fn is_css_ident(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    if bytes[0] == b'-' {
        if bytes.len() < 2 {
            return false;
        }
        if bytes[1].is_ascii_digit() {
            return false;
        }
    } else if !is_ident_start(bytes[0]) {
        return false;
    }
    bytes.iter().copied().all(is_ident_char)
}

/// Returns `true` if `s` is a single properly quoted string literal (`"..."` or `'...'`).
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

/// Finds the end of an at-rule header (the position of `{`, `;`, `}`, or EOF).
fn find_at_rule_header_end(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    let mut paren_depth: u32 = 0;
    while pos < len {
        let b = byte_at(ctx, pos);
        // Skip block comments.
        if b == b'/' && pos + 1 < len && byte_at(ctx, pos + 1) == b'*' {
            pos += 2;
            while pos + 1 < len && !(byte_at(ctx, pos) == b'*' && byte_at(ctx, pos + 1) == b'/') {
                pos += 1;
            }
            pos = if pos + 1 < len { pos + 2 } else { len };
            continue;
        }
        // Skip quoted strings.
        if b == b'"' || b == b'\'' {
            pos = skip_string(ctx, pos, len);
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

/// Trims leading and trailing ASCII whitespace from `start..end`.
fn trim_prelude_range(ctx: &LintContext<'_>, mut start: u32, mut end: u32) -> (u32, u32) {
    while start < end && byte_at(ctx, start).is_ascii_whitespace() {
        start += 1;
    }
    while end > start && byte_at(ctx, end - 1).is_ascii_whitespace() {
        end -= 1;
    }
    (start, end)
}

/// Skips a quoted string literal starting at `pos`.
fn skip_string(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    let quote = byte_at(ctx, pos);
    pos += 1;
    while pos < len {
        let cur = byte_at(ctx, pos);
        if cur == b'\\' {
            pos = (pos + 2).min(len);
            continue;
        }
        if cur == quote {
            return pos + 1;
        }
        pos += 1;
    }
    len
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
                rejected_message("@layer", "\"foo\""),
                (1, 8, 1, 13),
            ),
            reject(
                "@property  foo {}",
                None,
                rejected_message("@property", "foo"),
                (1, 12, 1, 15),
            ),
            reject(
                "@scope .foo {}",
                None,
                rejected_message("@scope", ".foo"),
                (1, 8, 1, 12),
            ),
            reject(
                "@font-palette-values foo {}",
                None,
                rejected_message("@font-palette-values", "foo"),
                (1, 22, 1, 25),
            ),
            reject(
                "@font-feature-values #foo {}",
                None,
                rejected_message("@font-feature-values", "#foo"),
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
}
