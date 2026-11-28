//! Specifies a list of required properties or descriptors for an at-rule
//! (`at-rule-property-required-list`).
//!
//! # Checked Constructs
//!
//! Standard CSS at-rules with `{ ... }` blocks are inspected:
//!
//! 1. **Configured at-rules** (ASCII case-insensitive):
//!    - Each configured at-rule block (such as `@font-face` or `@page`) must
//!      contain all required property or descriptor declarations as direct
//!      children (`for (const node of nodes) { if (!isDeclaration(node)) continue; }`).
//!    - Declarations inside nested child blocks (e.g. `@top-left` inside
//!      `@page`) do not satisfy the parent at-rule's required list.
//!
//! 2. **Ignored at-rules**:
//!    - Blockless at-rules (`if (!nodes) return;` in Stylelint).
//!    - Non-standard preprocessor constructs (`isStandardSyntaxAtRule`): Less
//!      variable declarations (`@var: ...`), Less mixin calls (`@mixin()`),
//!      and SCSS/Less interpolated at-rule names (`@#{$var}`, `@{var}`).

use std::ops::Deref;

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::options::parse_string_set_map;

/// Configuration for [`AtRulePropertyRequiredList`].
#[derive(Debug, Default)]
pub struct AtRulePropertyRequiredListConfig {
    /// Required lowercased property/descriptor names keyed by lowercased at-rule name.
    required_by_at_rule: Vec<(String, Vec<String>)>,
}

crate::define_stx_lint! {
    /// Specify a list of required properties for an at-rule.
    ///
    /// Enforces that configured at-rule blocks (such as `@font-face` or `@page`)
    /// directly contain all required property or descriptor declarations.
    "at-rule-property-required-list", restriction,
    short_description = "Specify a list of required properties for an at-rule.";
    #[derive(Debug, Default)]
    pub struct AtRulePropertyRequiredList(AtRulePropertyRequiredListConfig);
}

impl Deref for AtRulePropertyRequiredList {
    type Target = AtRulePropertyRequiredListConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Formats the diagnostic message matching Stylelint's `messages.expected`.
#[inline]
#[must_use]
fn expected(at_rule: &str, property: &str) -> String {
    format!("Expected property or descriptor \"{property}\" for at-rule \"{at_rule}\"")
}

impl AtRulePropertyRequiredList {
    /// Constructs a configured [`AtRulePropertyRequiredList`] rule instance from
    /// raw Stylelint JSON options.
    #[must_use]
    pub fn new(primary: &Value, secondary: Option<&Map<String, Value>>) -> Self {
        Self(AtRulePropertyRequiredListConfig {
            required_by_at_rule: parse_string_set_map(primary, secondary),
        })
    }

    /// Looks up the list of required properties/descriptors for `at_rule_name`
    /// (case-insensitively).
    #[inline]
    #[must_use]
    fn required_props_for(&self, at_rule_name: &str) -> Option<&[String]> {
        self.required_by_at_rule
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(at_rule_name))
            .map(|(_, props)| props.as_slice())
    }
}

impl Rule for AtRulePropertyRequiredList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        if self.required_by_at_rule.is_empty() {
            return;
        }

        let bytes = ctx.source.as_bytes();
        let len = bytes.len();
        let mut i: usize = 0;
        // Track whether the scanner is positioned where a statement / at-rule can start.
        let mut at_statement_start = true;

        while i < len {
            let b = bytes[i];

            // Whitespace preserves `at_statement_start`.
            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            // Skip block comments `/* ... */` (preserving `at_statement_start`).
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                i = skip_block_comment(bytes, i);
                continue;
            }

            // Skip single-line comments `// ...` when not part of a URL scheme (`://`).
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' && (i == 0 || bytes[i - 1] != b':')
            {
                i = skip_line_comment(bytes, i);
                continue;
            }

            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                at_statement_start = false;
                i = skip_string(bytes, i);
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

                // Skip SCSS/Less interpolated at-rule names starting immediately at `@` (`@#{$var}`, `@{var}`).
                if name_start < len
                    && (bytes[name_start] == b'{'
                        || (matches!(bytes[name_start], b'#' | b'$')
                            && name_start + 1 < len
                            && bytes[name_start + 1] == b'{'))
                {
                    at_statement_start = false;
                    let brace_pos = if bytes[name_start] == b'{' {
                        name_start
                    } else {
                        name_start + 1
                    };
                    i = find_matching_brace(bytes, brace_pos).map_or(len, |close| close + 1);
                    continue;
                }

                if name_start < len && is_ident_start(bytes[name_start]) {
                    let mut name_end = name_start + 1;
                    while name_end < len && is_ident_char(bytes[name_end]) {
                        name_end += 1;
                    }

                    // If the identifier is immediately followed by interpolation (`@font-#{$face}`),
                    // `isStandardSyntaxAtRule` returns `false`.
                    if name_end < len
                        && (bytes[name_end] == b'#'
                            || (matches!(bytes[name_end], b'@' | b'$')
                                && name_end + 1 < len
                                && bytes[name_end + 1] == b'{'))
                    {
                        at_statement_start = false;
                        i = find_at_rule_header_end(bytes, name_end);
                        continue;
                    }

                    // Ignore Less variable declarations (`@var: ...`) and Less mixin calls (`@mixin(...)`).
                    let next_non_ws = skip_spaces_and_tabs(bytes, name_end);
                    let is_less_construct = next_non_ws < len
                        && (bytes[next_non_ws] == b':'
                            || (next_non_ws == name_end && bytes[next_non_ws] == b'('));

                    let header_end = find_at_rule_header_end(bytes, name_end);
                    if !is_less_construct
                        && header_end < len
                        && bytes[header_end] == b'{'
                        && let Some(close_brace) = find_matching_brace(bytes, header_end)
                    {
                        let name = &ctx.source[name_start..name_end];
                        if let Some(required_props) = self.required_props_for(name) {
                            let present_props = collect_direct_declaration_props(
                                ctx.source,
                                header_end + 1,
                                close_brace,
                            );
                            let at_name = &ctx.source[at_start..name_end];
                            if let (Ok(start), Ok(end)) =
                                (u32::try_from(at_start), u32::try_from(name_end))
                            {
                                let span = Span { start, end };
                                for req in required_props {
                                    if !present_props.iter().any(|p| p.eq_ignore_ascii_case(req)) {
                                        ctx.report(Self::NAME, span, expected(at_name, req));
                                    }
                                }
                            }
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

/// Collects lowercased property/descriptor names of all direct child declarations
/// inside `block_start..block_end` (ignoring nested blocks).
#[must_use]
fn collect_direct_declaration_props(
    source: &str,
    block_start: usize,
    block_end: usize,
) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut props = Vec::new();
    let mut pos = block_start;
    let mut stmt_start = block_start;
    let mut paren_depth: u32 = 0;
    let mut colon_pos: Option<usize> = None;

    while pos < block_end {
        let b = bytes[pos];

        // Skip block comments.
        if b == b'/' && pos + 1 < block_end && bytes[pos + 1] == b'*' {
            pos = skip_block_comment_bounded(bytes, pos, block_end);
            continue;
        }

        // Skip single-line comments.
        if b == b'/'
            && pos + 1 < block_end
            && bytes[pos + 1] == b'/'
            && (pos == 0 || bytes[pos - 1] != b':')
        {
            pos = skip_line_comment_bounded(bytes, pos, block_end);
            continue;
        }

        // Skip strings.
        if b == b'"' || b == b'\'' {
            pos = skip_string_bounded(bytes, pos, block_end);
            continue;
        }

        if b == b'(' {
            paren_depth += 1;
            pos += 1;
            continue;
        }

        if b == b')' {
            paren_depth = paren_depth.saturating_sub(1);
            pos += 1;
            continue;
        }

        // Skip nested `{ ... }` blocks and reset statement state.
        if b == b'{' && paren_depth == 0 {
            if let Some(nested_close) = find_matching_brace_bounded(bytes, pos, block_end) {
                pos = nested_close + 1;
            } else {
                pos = block_end;
            }
            stmt_start = pos;
            colon_pos = None;
            continue;
        }

        if b == b':' && paren_depth == 0 && colon_pos.is_none() {
            colon_pos = Some(pos);
            pos += 1;
            continue;
        }

        if b == b';' && paren_depth == 0 {
            if let Some(colon) = colon_pos
                && let Some(prop) = extract_prop_name(source, stmt_start, colon)
            {
                props.push(prop);
            }
            pos += 1;
            stmt_start = pos;
            colon_pos = None;
            continue;
        }

        pos += 1;
    }

    // Handle trailing declaration without a semicolon before `}`.
    if let Some(colon) = colon_pos
        && let Some(prop) = extract_prop_name(source, stmt_start, colon)
    {
        props.push(prop);
    }

    props
}

/// Extracts and lowercases the property identifier from `start..end` (before `:`).
#[must_use]
fn extract_prop_name(source: &str, mut start: usize, end: usize) -> Option<String> {
    let bytes = source.as_bytes();

    // Advance past leading whitespace and comments.
    while start < end {
        let b = bytes[start];
        if b.is_ascii_whitespace() {
            start += 1;
            continue;
        }
        if b == b'/' && start + 1 < end && bytes[start + 1] == b'*' {
            start = skip_block_comment_bounded(bytes, start, end);
            continue;
        }
        if b == b'/' && start + 1 < end && bytes[start + 1] == b'/' {
            start = skip_line_comment_bounded(bytes, start, end);
            continue;
        }
        break;
    }

    if start >= end || bytes[start] == b'@' {
        return None;
    }

    // Find the end of the property identifier before any trailing whitespace or comments.
    let mut prop_end = start;
    while prop_end < end {
        let b = bytes[prop_end];
        if b.is_ascii_whitespace()
            || (b == b'/' && prop_end + 1 < end && matches!(bytes[prop_end + 1], b'*' | b'/'))
        {
            break;
        }
        prop_end += 1;
    }

    // Ensure everything between `prop_end` and `end` is only whitespace or comments.
    let mut cursor = prop_end;
    while cursor < end {
        let b = bytes[cursor];
        if b.is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if b == b'/' && cursor + 1 < end && bytes[cursor + 1] == b'*' {
            cursor = skip_block_comment_bounded(bytes, cursor, end);
            continue;
        }
        if b == b'/' && cursor + 1 < end && bytes[cursor + 1] == b'/' {
            cursor = skip_line_comment_bounded(bytes, cursor, end);
            continue;
        }
        return None;
    }

    let raw = source.get(start..prop_end)?.trim();
    if raw.is_empty() {
        None
    } else {
        Some(raw.to_ascii_lowercase())
    }
}

/// Finds the end of an at-rule header (the position of `{`, `;`, `}`, or EOF).
#[must_use]
fn find_at_rule_header_end(bytes: &[u8], mut pos: usize) -> usize {
    let len = bytes.len();
    let mut paren_depth: u32 = 0;

    while pos < len {
        let b = bytes[pos];
        if b == b'/' && pos + 1 < len && bytes[pos + 1] == b'*' {
            pos = skip_block_comment(bytes, pos);
            continue;
        }
        if b == b'/'
            && pos + 1 < len
            && bytes[pos + 1] == b'/'
            && (pos == 0 || bytes[pos - 1] != b':')
        {
            pos = skip_line_comment(bytes, pos);
            continue;
        }
        if b == b'"' || b == b'\'' {
            pos = skip_string(bytes, pos);
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

/// Finds the matching `}` for an opening `{` at `open_brace_pos`.
#[must_use]
fn find_matching_brace(bytes: &[u8], open_brace_pos: usize) -> Option<usize> {
    find_matching_brace_bounded(bytes, open_brace_pos, bytes.len())
}

/// Finds the matching `}` for an opening `{` at `open_brace_pos` within `..limit`.
#[must_use]
fn find_matching_brace_bounded(bytes: &[u8], open_brace_pos: usize, limit: usize) -> Option<usize> {
    let mut depth: u32 = 0;
    let mut pos = open_brace_pos;

    while pos < limit {
        let b = bytes[pos];
        if b == b'/' && pos + 1 < limit && bytes[pos + 1] == b'*' {
            pos = skip_block_comment_bounded(bytes, pos, limit);
            continue;
        }
        if b == b'/'
            && pos + 1 < limit
            && bytes[pos + 1] == b'/'
            && (pos == 0 || bytes[pos - 1] != b':')
        {
            pos = skip_line_comment_bounded(bytes, pos, limit);
            continue;
        }
        if b == b'"' || b == b'\'' {
            pos = skip_string_bounded(bytes, pos, limit);
            continue;
        }
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(pos);
            }
        }
        pos += 1;
    }

    None
}

/// Skips a `/* ... */` block comment starting at `pos`.
#[inline]
#[must_use]
fn skip_block_comment(bytes: &[u8], pos: usize) -> usize {
    skip_block_comment_bounded(bytes, pos, bytes.len())
}

/// Skips a `/* ... */` block comment starting at `pos` within `..limit`.
#[must_use]
fn skip_block_comment_bounded(bytes: &[u8], mut pos: usize, limit: usize) -> usize {
    pos += 2;
    while pos + 1 < limit && !(bytes[pos] == b'*' && bytes[pos + 1] == b'/') {
        pos += 1;
    }
    if pos + 1 < limit { pos + 2 } else { limit }
}

/// Skips a `// ...` line comment starting at `pos`.
#[inline]
#[must_use]
fn skip_line_comment(bytes: &[u8], pos: usize) -> usize {
    skip_line_comment_bounded(bytes, pos, bytes.len())
}

/// Skips a `// ...` line comment starting at `pos` within `..limit`.
#[must_use]
fn skip_line_comment_bounded(bytes: &[u8], mut pos: usize, limit: usize) -> usize {
    pos += 2;
    while pos < limit && bytes[pos] != b'\n' {
        pos += 1;
    }
    pos
}

/// Skips a quoted string literal starting at `pos`.
#[inline]
#[must_use]
fn skip_string(bytes: &[u8], pos: usize) -> usize {
    skip_string_bounded(bytes, pos, bytes.len())
}

/// Skips a quoted string literal starting at `pos` within `..limit`.
#[must_use]
fn skip_string_bounded(bytes: &[u8], mut pos: usize, limit: usize) -> usize {
    let quote = bytes[pos];
    pos += 1;
    while pos < limit {
        let cur = bytes[pos];
        if cur == b'\\' {
            pos = (pos + 2).min(limit);
            continue;
        }
        if cur == quote {
            return pos + 1;
        }
        pos += 1;
    }
    limit
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

/// Advances past horizontal ASCII whitespace (`' '` and `'\t'`).
#[inline]
#[must_use]
fn skip_spaces_and_tabs(bytes: &[u8], mut idx: usize) -> usize {
    let len = bytes.len();
    while idx < len && matches!(bytes[idx], b' ' | b'\t') {
        idx += 1;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn font_face_and_page_required_props() {
        let pass = vec![
            "@font-face { font-display: auto; font-family: 'Arvo'; }",
            "@font-face { font-display: auto; font-family: 'Arvo'; src: url('abc'); /* IE9 */ }",
            "@FONT-FACE { FONT-DISPLAY: AUTO; FONT-FAMILY: 'ARVO'; }",
            "@page { padding: 0.5cm; margin: 1cm; }",
            "@counter-style counter { system: cyclic; }",
            "@mixin invalid-at-rule { @content; }",
        ];

        let fail = vec![
            reject(
                "@font-face { font-display: auto; }",
                None,
                expected("@font-face", "font-family"),
                (1, 1, 1, 11),
            ),
            reject(
                "@FONT-FACE { FONT-DISPLAY: AUTO; }",
                None,
                expected("@FONT-FACE", "font-family"),
                (1, 1, 1, 11),
            ),
            reject(
                "@font-face { font-family: 'Arvo'; font-weight: normal }",
                None,
                expected("@font-face", "font-display"),
                (1, 1, 1, 11),
            ),
            reject(
                "@page { padding: 0.5cm }",
                None,
                expected("@page", "margin"),
                (1, 1, 1, 6),
            ),
        ];

        Tester::new(
            AtRulePropertyRequiredList::NAME,
            json!([{
                "font-face": ["font-display", "font-family"],
                "page": "margin"
            }]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn nested_blocks_comments_and_multiple_missing_props() {
        let pass = vec![
            "@font-face;",
            "@page;",
            "@font-face { font-display /* comment */ : auto; font-family: 'Arvo'; }",
            "@media screen { @font-face { font-display: swap; font-family: 'Arvo'; } }",
            "/* Привет — мир */ @font-face { font-display: swap; font-family: 'Привет'; }",
        ];

        let fail = vec![
            reject_warnings(
                "@font-face {}",
                None,
                vec![
                    (expected("@font-face", "font-display"), (1, 1, 1, 11)),
                    (expected("@font-face", "font-family"), (1, 1, 1, 11)),
                ],
            ),
            reject(
                "@page { @top-left { margin: 1cm; } }",
                None,
                expected("@page", "margin"),
                (1, 1, 1, 6),
            ),
            reject(
                "@media screen { @font-face { font-display: swap; } }",
                None,
                expected("@font-face", "font-family"),
                (1, 17, 1, 27),
            ),
        ];

        Tester::new(
            AtRulePropertyRequiredList::NAME,
            json!([{
                "font-face": ["font-display", "font-family"],
                "page": "margin"
            }]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn preprocessor_non_standard_at_rules_ignored() {
        let scss_pass = vec![
            "@#{$font-face} { font-display: auto; }",
            "@font-#{$face} { font-display: auto; }",
            "@font-face // comment with ; and { before block\n{ font-display: auto; font-family: 'Arvo'; }",
        ];

        Tester::new(
            AtRulePropertyRequiredList::NAME,
            json!([{
                "font-face": ["font-display", "font-family"]
            }]),
            &scss_pass,
            &[],
        )
        .with_custom_syntax("postcss-scss")
        .test();

        let less_pass = vec![
            "@font-face: { font-display: auto; };",
            "@font-face();",
            "@{font-face} { font-display: auto; }",
        ];

        Tester::new(
            AtRulePropertyRequiredList::NAME,
            json!([{
                "font-face": ["font-display", "font-family"]
            }]),
            &less_pass,
            &[],
        )
        .with_custom_syntax("postcss-less")
        .test();
    }
}
