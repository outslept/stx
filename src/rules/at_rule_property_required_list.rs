//! Rule `at-rule-property-required-list`: specify a list of required properties
//! or descriptors for an at-rule.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-property-required-list>

use std::ops::Deref;

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::options::parse_string_set_map;

/// Configuration for [`AtRulePropertyRequiredList`].
#[derive(Default)]
pub struct AtRulePropertyRequiredListConfig {
    /// Required lowercased property/descriptor names keyed by lowercased at-rule name.
    required_by_at_rule: Vec<(String, Vec<String>)>,
}

crate::define_stx_lint! {
    /// Require specific declarations directly in configured at-rule blocks.
    ///
    /// In config files, put the map in the primary option: keys are at-rule
    /// names without `@`, and values are a declaration name or a list of names.
    /// Matching is ASCII case-insensitive. Blockless at-rules are ignored,
    /// and declarations in child blocks do not count.
    ///
    /// The constructor can read this map from the secondary object when the
    /// primary is not an object, but config validation rejects at-rule names
    /// as secondary keys.
    ///
    /// With `{"font-face": ["font-display", "font-family"]}`:
    ///
    /// ```css
    /// @font-face { font-family: Body; }                     /* reported */
    /// @font-face { font-family: Body; font-display: swap; } /* accepted */
    /// ```
    "at-rule-property-required-list", restriction;
    #[derive(Default)]
    pub struct AtRulePropertyRequiredList(AtRulePropertyRequiredListConfig);
}

impl Deref for AtRulePropertyRequiredList {
    type Target = AtRulePropertyRequiredListConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AtRulePropertyRequiredList {
    /// Creates a new `at-rule-property-required-list` rule instance from configuration options.
    pub fn new(primary: &Value, secondary: Option<&Map<String, Value>>) -> Self {
        Self(AtRulePropertyRequiredListConfig {
            required_by_at_rule: parse_string_set_map(primary, secondary),
        })
    }

    /// Formats the diagnostic message when a required property or descriptor is missing.
    #[inline]
    fn expected_message(at_rule: &str, property: &str) -> String {
        format!("Expected property or descriptor \"{property}\" for at-rule \"{at_rule}\"")
    }

    /// Looks up the list of required properties/descriptors for `at_rule_lower`.
    fn required_props_for(&self, at_rule_lower: &str) -> Option<&[String]> {
        self.required_by_at_rule
            .iter()
            .find(|(name, _)| name == at_rule_lower)
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

        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let mut i: u32 = 0;
        // Track whether the scanner is positioned where a statement / at-rule can start.
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
                i = skip_block_comment(ctx, i, len);
                continue;
            }

            // Skip single-line comments `// ...` when not part of a URL scheme (`://`).
            if b == b'/'
                && i + 1 < len
                && byte_at(ctx, i + 1) == b'/'
                && (i == 0 || byte_at(ctx, i - 1) != b':')
            {
                i = skip_line_comment(ctx, i, len);
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

                    // Ignore Less variable declarations (`@var: ...`) and Less mixin calls (`@mixin(...)`).
                    let next_non_ws = skip_spaces_and_tabs(ctx, name_end, len);
                    let is_less_construct = next_non_ws < len
                        && (byte_at(ctx, next_non_ws) == b':'
                            || (next_non_ws == name_end && byte_at(ctx, next_non_ws) == b'('));

                    let header_end = find_at_rule_header_end(ctx, name_end, len);
                    if !is_less_construct
                        && header_end < len
                        && byte_at(ctx, header_end) == b'{'
                        && let Some(close_brace) = find_matching_brace(ctx, header_end, len)
                    {
                        let name = ctx.slice_range(name_start, name_end);
                        let name_lower = name.to_ascii_lowercase();

                        if let Some(required_props) = self.required_props_for(&name_lower) {
                            let present_props =
                                collect_direct_declaration_props(ctx, header_end + 1, close_brace);
                            let at_name = ctx.slice_range(at_start, name_end);
                            let span = Span {
                                start: at_start,
                                end: name_end,
                            };

                            for req in required_props {
                                let req_lower = req.to_ascii_lowercase();
                                if !present_props.iter().any(|p| p == &req_lower) {
                                    ctx.report(
                                        Self::NAME,
                                        span,
                                        Self::expected_message(at_name, req),
                                    );
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
fn collect_direct_declaration_props(
    ctx: &LintContext<'_>,
    block_start: u32,
    block_end: u32,
) -> Vec<String> {
    let mut props = Vec::new();
    let mut pos = block_start;
    let mut stmt_start = block_start;
    let mut paren_depth: u32 = 0;
    let mut colon_pos: Option<u32> = None;

    while pos < block_end {
        let b = byte_at(ctx, pos);

        // Skip block comments.
        if b == b'/' && pos + 1 < block_end && byte_at(ctx, pos + 1) == b'*' {
            pos = skip_block_comment(ctx, pos, block_end);
            continue;
        }

        // Skip single-line comments.
        if b == b'/'
            && pos + 1 < block_end
            && byte_at(ctx, pos + 1) == b'/'
            && (pos == 0 || byte_at(ctx, pos - 1) != b':')
        {
            pos = skip_line_comment(ctx, pos, block_end);
            continue;
        }

        // Skip strings.
        if b == b'"' || b == b'\'' {
            pos = skip_string(ctx, pos, block_end);
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
            if let Some(nested_close) = find_matching_brace(ctx, pos, block_end) {
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
                && let Some(prop) = extract_prop_name(ctx, stmt_start, colon)
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
        && let Some(prop) = extract_prop_name(ctx, stmt_start, colon)
    {
        props.push(prop);
    }

    props
}

/// Extracts and lowercases the property identifier from `stmt_start..colon_pos`.
fn extract_prop_name(ctx: &LintContext<'_>, mut start: u32, mut end: u32) -> Option<String> {
    // Advance past leading whitespace and comments.
    while start < end {
        let b = byte_at(ctx, start);
        if b.is_ascii_whitespace() {
            start += 1;
            continue;
        }
        if b == b'/' && start + 1 < end && byte_at(ctx, start + 1) == b'*' {
            start = skip_block_comment(ctx, start, end);
            continue;
        }
        if b == b'/' && start + 1 < end && byte_at(ctx, start + 1) == b'/' {
            start = skip_line_comment(ctx, start, end);
            continue;
        }
        break;
    }

    while end > start && byte_at(ctx, end - 1).is_ascii_whitespace() {
        end -= 1;
    }

    if start >= end || byte_at(ctx, start) == b'@' {
        return None;
    }

    let raw = ctx.slice_range(start, end).trim();
    if raw.is_empty() {
        None
    } else {
        Some(raw.to_ascii_lowercase())
    }
}

/// Finds the end of an at-rule header (the position of `{`, `;`, `}`, or EOF).
fn find_at_rule_header_end(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    let mut paren_depth: u32 = 0;
    while pos < len {
        let b = byte_at(ctx, pos);
        if b == b'/' && pos + 1 < len && byte_at(ctx, pos + 1) == b'*' {
            pos = skip_block_comment(ctx, pos, len);
            continue;
        }
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

/// Finds the matching `}` for an opening `{` at `open_brace_pos`.
fn find_matching_brace(ctx: &LintContext<'_>, open_brace_pos: u32, len: u32) -> Option<u32> {
    let mut depth: u32 = 0;
    let mut pos = open_brace_pos;
    while pos < len {
        let b = byte_at(ctx, pos);
        if b == b'/' && pos + 1 < len && byte_at(ctx, pos + 1) == b'*' {
            pos = skip_block_comment(ctx, pos, len);
            continue;
        }
        if b == b'/'
            && pos + 1 < len
            && byte_at(ctx, pos + 1) == b'/'
            && (pos == 0 || byte_at(ctx, pos - 1) != b':')
        {
            pos = skip_line_comment(ctx, pos, len);
            continue;
        }
        if b == b'"' || b == b'\'' {
            pos = skip_string(ctx, pos, len);
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
fn skip_block_comment(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    pos += 2;
    while pos + 1 < len && !(byte_at(ctx, pos) == b'*' && byte_at(ctx, pos + 1) == b'/') {
        pos += 1;
    }
    if pos + 1 < len { pos + 2 } else { len }
}

/// Skips a `// ...` line comment starting at `pos`.
fn skip_line_comment(ctx: &LintContext<'_>, mut pos: u32, len: u32) -> u32 {
    pos += 2;
    while pos < len && byte_at(ctx, pos) != b'\n' {
        pos += 1;
    }
    pos
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
                AtRulePropertyRequiredList::expected_message("@font-face", "font-family"),
                (1, 1, 1, 11),
            ),
            reject(
                "@FONT-FACE { FONT-DISPLAY: AUTO; }",
                None,
                AtRulePropertyRequiredList::expected_message("@FONT-FACE", "font-family"),
                (1, 1, 1, 11),
            ),
            reject(
                "@font-face { font-family: 'Arvo'; font-weight: normal }",
                None,
                AtRulePropertyRequiredList::expected_message("@font-face", "font-display"),
                (1, 1, 1, 11),
            ),
            reject(
                "@page { padding: 0.5cm }",
                None,
                AtRulePropertyRequiredList::expected_message("@page", "margin"),
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
}
