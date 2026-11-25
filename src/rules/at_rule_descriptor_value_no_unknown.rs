//! Rule `at-rule-descriptor-value-no-unknown`: disallow unknown values for descriptors within at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-descriptor-value-no-unknown>

use crate::config::RuleSetting;
use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::{
    FONT_FEATURE_VALUE_TYPES, LENGTH_UNITS, NESTING_SUPPORTED_AT_KEYWORDS, PAGE_MARGIN_AT_KEYWORDS,
    at_rule_descriptors, has_interpolation, in_set_ignore_case,
};
use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;

/// At-rules whose declarations accept `var()` / `env()` substitutions even though
/// their properties are modeled as descriptors.
const VAR_FUNCTION_ACCEPTING_AT_KEYWORDS: &[&str] = &["function", "position-try"];

/// Page size keywords valid for the `size` descriptor inside `@page`.
const PAGE_SIZE_KEYWORDS: &[&str] = &[
    "a3", "a4", "a5", "b4", "b5", "jis-b4", "jis-b5", "ledger", "legal", "letter",
];

/// Keywords valid for the `font-display` descriptor inside `@font-face` and
/// `@font-feature-values`.
const FONT_DISPLAY_KEYWORDS: &[&str] = &["auto", "block", "fallback", "optional", "swap"];

/// Keywords valid for the `font-stretch` / `font-width` descriptors inside
/// `@font-face`.
const FONT_STRETCH_DESCRIPTOR_KEYWORDS: &[&str] = &[
    "auto",
    "condensed",
    "expanded",
    "extra-condensed",
    "extra-expanded",
    "normal",
    "semi-condensed",
    "semi-expanded",
    "ultra-condensed",
    "ultra-expanded",
];

/// Keywords valid for the `system` descriptor inside `@counter-style`.
const COUNTER_STYLE_SYSTEM_KEYWORDS: &[&str] = &[
    "additive",
    "alphabetic",
    "cyclic",
    "fixed",
    "numeric",
    "symbolic",
];

/// Keywords valid for the `speak-as` descriptor inside `@counter-style`.
const COUNTER_STYLE_SPEAK_AS_KEYWORDS: &[&str] =
    &["auto", "bullets", "numbers", "spell-out", "words"];

/// Enforces that known descriptors within at-rules (such as `@property`,
/// `@counter-style`, `@font-face`, `@page`, `@view-transition`, etc.) have valid
/// specification-conformant values.
///
/// # What it does
///
/// Checks standard-syntax declarations inside at-rules that do not support
/// general selector nesting. When a declaration's property is a known descriptor
/// for an enclosing at-rule, its value is validated against the CSS grammar for
/// that descriptor.
///
/// Unknown descriptors themselves are ignored by this rule (they are handled by
/// `at-rule-descriptor-no-unknown`). Additionally, `var()` and `env()` function
/// calls are disallowed in descriptors unless the enclosing at-rule explicitly
/// permits custom property substitution (such as `@function` or `@position-try`,
/// or page context properties inside `@page`).
///
/// # Why is this bad?
///
/// Invalid descriptor values inside at-rules like `@property` or `@font-face`
/// cause the descriptor (or the entire at-rule) to be dropped by the browser at
/// parse time.
///
/// # Options
///
/// ## Primary Option
///
/// - `true`: enable the rule.
///
/// ```json
/// {
///   "at-rule-descriptor-value-no-unknown": true
/// }
/// ```
///
/// # Examples
///
/// ## Incorrect
///
/// ```css
/// @counter-style foo { system: bar; }
/// @property --foo { syntax: bar; inherits: baz; }
/// @font-face { font-weight: var(--foo); }
/// @page { size: var(--foo); }
/// ```
///
/// ## Correct
///
/// ```css
/// @counter-style foo { system: cyclic; symbols: url(bar.svg); suffix: " "; }
/// @property --foo { syntax: "<integer>"; inherits: false; initial-value: 0; }
/// @page { margin: var(--foo); }
/// ```
///
/// # References
///
/// - [Stylelint: `at-rule-descriptor-value-no-unknown`](https://stylelint.io/user-guide/rules/at-rule-descriptor-value-no-unknown)
pub struct AtRuleDescriptorValueNoUnknown;

impl AtRuleDescriptorValueNoUnknown {
    /// Creates a new [`AtRuleDescriptorValueNoUnknown`] rule instance from the
    /// given [`RuleSetting`].
    pub fn from_setting(_setting: &RuleSetting) -> Option<Self> {
        Some(Self)
    }

    /// Formats the diagnostic message when `value` is invalid for `descriptor`.
    fn rejected_message(descriptor: &str, value: &str) -> String {
        format!("Unknown value \"{value}\" for descriptor \"{descriptor}\"")
    }
}

/// Represents an enclosing CSS block in the ancestry stack while scanning a
/// stylesheet.
#[derive(Clone, Copy)]
enum BlockKind<'a> {
    /// An at-rule block (e.g. `@font-face { ... }`) with its raw name without `@`.
    AtRule { name: &'a str },
    /// A qualified rule or non-at-rule block (e.g. `a { ... }`).
    QualifiedRule,
}

impl Rule for AtRuleDescriptorValueNoUnknown {
    fn name(&self) -> &'static str {
        "at-rule-descriptor-value-no-unknown"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let mut ancestors = Vec::new();
        self.scan_block(ctx, 0, len, &mut ancestors);
    }
}

impl AtRuleDescriptorValueNoUnknown {
    /// Recursively scans a block or top-level stylesheet range `[start, end)` for
    /// nested blocks and declarations.
    fn scan_block<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        start: u32,
        end: u32,
        ancestors: &mut Vec<BlockKind<'a>>,
    ) {
        let mut pos = start;

        while pos < end {
            pos = skip_ws_and_comments(ctx, pos, end);
            if pos >= end {
                break;
            }

            let stmt_start = pos;
            let (stmt_end, terminator, first_colon) = scan_statement(ctx, stmt_start, end);

            match terminator {
                StmtTerminator::OpenBrace(open_brace) => {
                    let close_brace = find_matching_brace(ctx, open_brace + 1, end);
                    let header = ctx.slice_range(stmt_start, open_brace).trim();

                    let block_kind = if let Some(at_name) = parse_standard_at_rule_name(header) {
                        BlockKind::AtRule { name: at_name }
                    } else {
                        BlockKind::QualifiedRule
                    };

                    ancestors.push(block_kind);
                    self.scan_block(ctx, open_brace + 1, close_brace, ancestors);
                    ancestors.pop();

                    pos = if close_brace < end {
                        close_brace + 1
                    } else {
                        end
                    };
                }
                StmtTerminator::Semicolon(semi_pos) => {
                    self.check_statement_decl(ctx, stmt_start, semi_pos, first_colon, ancestors);
                    pos = semi_pos + 1;
                }
                StmtTerminator::End => {
                    self.check_statement_decl(ctx, stmt_start, stmt_end, first_colon, ancestors);
                    pos = stmt_end;
                }
            }
        }
    }

    /// Validates the value of a standard-syntax declaration against all enclosing
    /// descriptor-bearing at-rules.
    fn check_statement_decl<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        stmt_start: u32,
        stmt_end: u32,
        first_colon: Option<u32>,
        ancestors: &[BlockKind<'a>],
    ) {
        if ancestors.is_empty() {
            return;
        }

        let Some(colon_pos) = first_colon else {
            return;
        };

        if byte_at(ctx, stmt_start) == Some(b'@') {
            return;
        }

        let raw_prop = ctx.slice_range(stmt_start, colon_pos);
        let prop = raw_prop.trim_end();
        if !is_standard_decl_prop(prop) {
            return;
        }

        let val_start = skip_ws_and_comments(ctx, colon_pos + 1, stmt_end);
        let raw_after_ws = ctx.slice_range(val_start, stmt_end);
        let value = raw_after_ws.trim_end();
        if value.is_empty() {
            return;
        }

        let val_len = u32::try_from(value.len()).unwrap_or(0);
        let val_span = Span {
            start: val_start,
            end: val_start + val_len,
        };

        let immediate_parent = ancestors.last().copied();

        for ancestor in ancestors {
            let BlockKind::AtRule { name: at_rule_name } = *ancestor else {
                continue;
            };

            if in_set_ignore_case(NESTING_SUPPORTED_AT_KEYWORDS, at_rule_name) {
                continue;
            }

            if let Some(BlockKind::AtRule { name: parent_name }) = immediate_parent
                && (in_set_ignore_case(PAGE_MARGIN_AT_KEYWORDS, parent_name)
                    || in_set_ignore_case(FONT_FEATURE_VALUE_TYPES, parent_name))
            {
                continue;
            }

            // Only validate descriptors that are actually known for this at-rule.
            let Some(descriptors) = at_rule_descriptors(at_rule_name) else {
                continue;
            };
            if !in_set_ignore_case(descriptors, prop) {
                continue;
            }

            // Check `var()` / `env()` function calls outside string literals.
            if contains_var_or_env_function(ctx, val_start, val_start + val_len) {
                if !in_set_ignore_case(VAR_FUNCTION_ACCEPTING_AT_KEYWORDS, at_rule_name) {
                    ctx.report(self.name(), val_span, Self::rejected_message(prop, value));
                }
                continue;
            }

            if !is_valid_descriptor_value(at_rule_name, prop, value) {
                ctx.report(self.name(), val_span, Self::rejected_message(prop, value));
            }
        }
    }
}

/// Checks whether `value` matches the W3C specification grammar for `descriptor`
/// inside `at_rule_name`.
fn is_valid_descriptor_value(at_rule_name: &str, descriptor: &str, value: &str) -> bool {
    let at_lower = at_rule_name.to_ascii_lowercase();
    let desc_lower = descriptor.to_ascii_lowercase();

    match (at_lower.as_str(), desc_lower.as_str()) {
        // @property
        ("property", "syntax") => is_single_quoted_string(value),
        ("property", "inherits") => {
            value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false")
        }
        ("property", "initial-value") => !value.is_empty(),

        // @counter-style
        ("counter-style", "system") => is_valid_counter_style_system(value),
        ("counter-style", "symbols") => value.split_whitespace().all(is_valid_counter_style_symbol),
        ("counter-style", "prefix" | "suffix") => is_valid_counter_style_symbol(value),
        ("counter-style", "negative") => {
            let parts: Vec<&str> = value.split_whitespace().collect();
            (1..=2).contains(&parts.len()) && parts.into_iter().all(is_valid_counter_style_symbol)
        }
        ("counter-style", "fallback") => {
            !value.eq_ignore_ascii_case("none") && is_custom_ident(value)
        }
        ("counter-style", "speak-as") => {
            in_set_ignore_case(COUNTER_STYLE_SPEAK_AS_KEYWORDS, value) || is_custom_ident(value)
        }

        // @view-transition
        ("view-transition", "navigation") => {
            value.eq_ignore_ascii_case("auto") || value.eq_ignore_ascii_case("none")
        }
        ("view-transition", "types") => {
            value.eq_ignore_ascii_case("none") || value.split_whitespace().all(is_custom_ident)
        }

        // @font-face & @font-feature-values
        ("font-face" | "font-feature-values", "font-display") => {
            in_set_ignore_case(FONT_DISPLAY_KEYWORDS, value)
        }
        ("font-face", "ascent-override" | "descent-override" | "line-gap-override") => {
            let parts: Vec<&str> = value.split_whitespace().collect();
            (1..=2).contains(&parts.len())
                && parts
                    .into_iter()
                    .all(|p| p.eq_ignore_ascii_case("normal") || is_non_negative_percentage(p))
        }
        ("font-face", "size-adjust") => is_non_negative_percentage(value),
        ("font-face", "font-weight") => is_valid_font_weight_descriptor(value),
        ("font-face", "font-stretch" | "font-width") => {
            let parts: Vec<&str> = value.split_whitespace().collect();
            (1..=2).contains(&parts.len())
                && parts.into_iter().all(|p| {
                    in_set_ignore_case(FONT_STRETCH_DESCRIPTOR_KEYWORDS, p)
                        || is_non_negative_percentage(p)
                })
        }
        ("font-face", "font-style") => {
            value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("normal")
                || value.eq_ignore_ascii_case("italic")
                || value.to_ascii_lowercase().starts_with("oblique")
        }
        ("font-face", "font-named-instance") => {
            value.eq_ignore_ascii_case("auto") || is_single_quoted_string(value)
        }
        ("font-face", "font-language-override") => {
            value.eq_ignore_ascii_case("normal") || is_single_quoted_string(value)
        }
        ("font-face", "font-family") => {
            is_single_quoted_string(value) || value.split_whitespace().all(is_custom_ident)
        }

        // @page
        ("page", "size") => is_valid_page_size(value),
        ("page", "orientation" | "page-orientation") => matches!(
            value.to_ascii_lowercase().as_str(),
            "upright" | "rotate-left" | "rotate-right" | "portrait" | "landscape"
        ),
        ("page", "marks") => {
            value.eq_ignore_ascii_case("none")
                || value
                    .split_whitespace()
                    .all(|p| p.eq_ignore_ascii_case("crop") || p.eq_ignore_ascii_case("cross"))
        }
        ("page", "bleed") => value.eq_ignore_ascii_case("auto") || is_length(value),

        _ => true,
    }
}

/// Validates the `system` descriptor value in `@counter-style`.
fn is_valid_counter_style_system(value: &str) -> bool {
    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.as_slice() {
        [single] => in_set_ignore_case(COUNTER_STYLE_SYSTEM_KEYWORDS, single),
        [first, second] if first.eq_ignore_ascii_case("fixed") => second.parse::<i64>().is_ok(),
        [first, second] if first.eq_ignore_ascii_case("extends") => is_custom_ident(second),
        _ => false,
    }
}

/// Validates a `<symbol>` token (`<string> | <image> | <custom-ident>`) in
/// `@counter-style`.
fn is_valid_counter_style_symbol(token: &str) -> bool {
    if is_single_quoted_string(token) {
        return true;
    }
    let lower = token.to_ascii_lowercase();
    if lower.starts_with("url(") && lower.ends_with(')') {
        return true;
    }
    is_custom_ident(token)
}

/// Validates the `font-weight` descriptor in `@font-face`.
fn is_valid_font_weight_descriptor(value: &str) -> bool {
    if value.eq_ignore_ascii_case("auto") {
        return true;
    }
    let parts: Vec<&str> = value.split_whitespace().collect();
    (1..=2).contains(&parts.len())
        && parts.into_iter().all(|p| {
            p.eq_ignore_ascii_case("normal")
                || p.eq_ignore_ascii_case("bold")
                || p.parse::<f64>().is_ok_and(|n| (1.0..=1000.0).contains(&n))
        })
}

/// Validates the `size` descriptor in `@page`.
fn is_valid_page_size(value: &str) -> bool {
    if value.eq_ignore_ascii_case("auto") {
        return true;
    }
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.is_empty() || parts.len() > 2 {
        return false;
    }
    if parts.iter().all(|p| is_length(p)) {
        return true;
    }
    parts.iter().all(|p| {
        p.eq_ignore_ascii_case("portrait")
            || p.eq_ignore_ascii_case("landscape")
            || in_set_ignore_case(PAGE_SIZE_KEYWORDS, p)
    })
}

/// Checks whether `s` is a single quoted CSS string literal (`"..."` or `'...'`).
fn is_single_quoted_string(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    let first = bytes[0];
    if (first != b'"' && first != b'\'') || bytes[bytes.len() - 1] != first {
        return false;
    }
    let mut i = 1;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
        } else if bytes[i] == first {
            return false;
        } else {
            i += 1;
        }
    }
    true
}

/// Checks whether `s` is a non-negative `<percentage>` token (e.g. `0%`, `100%`).
fn is_non_negative_percentage(s: &str) -> bool {
    let Some(num_part) = s.strip_suffix('%') else {
        return false;
    };
    num_part.parse::<f64>().is_ok_and(|n| n >= 0.0)
}

/// Checks whether `s` is a `<length>` token or `0`.
fn is_length(s: &str) -> bool {
    if s == "0" {
        return true;
    }
    let unit_start = s
        .bytes()
        .position(|b| b.is_ascii_alphabetic())
        .unwrap_or(s.len());
    if unit_start == 0 || unit_start == s.len() {
        return false;
    }
    let (num_part, unit_part) = s.split_at(unit_start);
    num_part.parse::<f64>().is_ok() && in_set_ignore_case(LENGTH_UNITS, unit_part)
}

/// Checks whether `s` is a valid CSS `<custom-ident>`.
fn is_custom_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Scans `[start, end)` in `ctx.source` and returns `true` if a `var(` or `env(`
/// function call occurs outside of string literals and comments.
fn contains_var_or_env_function(ctx: &LintContext<'_>, start: u32, end: u32) -> bool {
    let mut pos = start;
    while pos < end {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };

        if b == b'/' && byte_at(ctx, pos + 1) == Some(b'*') {
            pos = skip_block_comment(ctx, pos + 2, end);
            continue;
        }

        if b == b'"' || b == b'\'' {
            pos = skip_quoted_string(ctx, pos + 1, end, b);
            continue;
        }

        if b.is_ascii_alphabetic() || b == b'-' || b == b'_' {
            let ident_start = pos;
            pos += 1;
            while pos < end {
                let Some(ch) = byte_at(ctx, pos) else {
                    break;
                };
                if ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_' {
                    pos += 1;
                } else {
                    break;
                }
            }
            if byte_at(ctx, pos) == Some(b'(') {
                let ident = ctx.slice_range(ident_start, pos);
                if ident.eq_ignore_ascii_case("var") || ident.eq_ignore_ascii_case("env") {
                    return true;
                }
            }
            continue;
        }

        pos += 1;
    }
    false
}

/// Extracts a standard-syntax at-rule name from a block header starting with `@`.
fn parse_standard_at_rule_name(header: &str) -> Option<&str> {
    let rest = header.strip_prefix('@')?;
    if has_interpolation(header) {
        return None;
    }

    let name_len = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
        .count();
    if name_len == 0 {
        return None;
    }

    let name = &rest[..name_len];
    let after_name = &rest[name_len..];

    // Ignore Less mixin definitions/calls unless it is CSS `@function --foo()`.
    if !name.eq_ignore_ascii_case("function")
        && (after_name.starts_with('(') || after_name.starts_with(':'))
    {
        return None;
    }

    Some(name)
}

/// Checks whether `prop` is a standard-syntax CSS property/descriptor identifier.
fn is_standard_decl_prop(prop: &str) -> bool {
    if prop.is_empty() || prop.starts_with('$') || prop.starts_with('@') || has_interpolation(prop)
    {
        return false;
    }
    prop.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// How a statement inside a block was terminated.
enum StmtTerminator {
    OpenBrace(u32),
    Semicolon(u32),
    End,
}

/// Scans from `start` up to `end` to find the end of the current statement (`{`,
/// `;`, or `end`) and records the position of the first top-level `:`.
fn scan_statement(
    ctx: &LintContext<'_>,
    start: u32,
    end: u32,
) -> (u32, StmtTerminator, Option<u32>) {
    let mut pos = start;
    let mut paren_depth: u32 = 0;
    let mut first_colon: Option<u32> = None;

    while pos < end {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };

        match b {
            b'/' if byte_at(ctx, pos + 1) == Some(b'*') => {
                pos = skip_block_comment(ctx, pos + 2, end);
            }
            b'"' | b'\'' => {
                pos = skip_quoted_string(ctx, pos + 1, end, b);
            }
            b'(' => {
                paren_depth += 1;
                pos += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                pos += 1;
            }
            b':' if paren_depth == 0 && first_colon.is_none() => {
                first_colon = Some(pos);
                pos += 1;
            }
            b'{' if paren_depth == 0 => {
                return (pos, StmtTerminator::OpenBrace(pos), first_colon);
            }
            b';' if paren_depth == 0 => {
                return (pos, StmtTerminator::Semicolon(pos), first_colon);
            }
            _ => {
                pos += 1;
            }
        }
    }

    (end, StmtTerminator::End, first_colon)
}

/// Finds the matching `}` for a block body starting at `start` (immediately after `{`).
fn find_matching_brace(ctx: &LintContext<'_>, start: u32, end: u32) -> u32 {
    let mut pos = start;
    let mut depth: u32 = 1;

    while pos < end {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };

        match b {
            b'/' if byte_at(ctx, pos + 1) == Some(b'*') => {
                pos = skip_block_comment(ctx, pos + 2, end);
            }
            b'"' | b'\'' => {
                pos = skip_quoted_string(ctx, pos + 1, end, b);
            }
            b'{' => {
                depth += 1;
                pos += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return pos;
                }
                pos += 1;
            }
            _ => {
                pos += 1;
            }
        }
    }

    end
}

/// Skips whitespace and `/* ... */` comments starting from `start` up to `end`.
fn skip_ws_and_comments(ctx: &LintContext<'_>, start: u32, end: u32) -> u32 {
    let mut pos = start;
    while pos < end {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };
        if b.is_ascii_whitespace() {
            pos += 1;
        } else if b == b'/' && byte_at(ctx, pos + 1) == Some(b'*') {
            pos = skip_block_comment(ctx, pos + 2, end);
        } else {
            break;
        }
    }
    pos
}

/// Skips past the closing `*/` of a block comment.
fn skip_block_comment(ctx: &LintContext<'_>, start: u32, end: u32) -> u32 {
    let mut pos = start;
    while pos < end {
        if byte_at(ctx, pos) == Some(b'*') && byte_at(ctx, pos + 1) == Some(b'/') {
            return pos + 2;
        }
        pos += 1;
    }
    end
}

/// Skips past the closing `quote` character of a string literal, respecting `\` escapes.
fn skip_quoted_string(ctx: &LintContext<'_>, start: u32, end: u32, quote: u8) -> u32 {
    let mut pos = start;
    while pos < end {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };
        if b == b'\\' {
            pos = (pos + 2).min(end);
        } else if b == quote {
            return pos + 1;
        } else {
            pos += 1;
        }
    }
    end
}

/// Returns the byte at `offset` in `ctx.source` using `u32` span indexing.
#[inline]
fn byte_at(ctx: &LintContext<'_>, offset: u32) -> Option<u8> {
    ctx.slice_range(offset, offset.saturating_add(1))
        .as_bytes()
        .first()
        .copied()
}
