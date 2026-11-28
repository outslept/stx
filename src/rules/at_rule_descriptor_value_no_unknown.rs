//! Disallows unknown values for descriptors within at-rules
//! (`at-rule-descriptor-value-no-unknown`).
//!
//! # Checked Constructs
//!
//! 1. Standard-syntax declarations inside any enclosing at-rule that does not support
//!    general selector nesting (`NESTING_SUPPORTED_AT_KEYWORDS`), provided the declaration's
//!    property is a known descriptor for that at-rule.
//! 2. Unknown descriptor names are ignored by this rule (they are reported by
//!    `at-rule-descriptor-no-unknown`).
//! 3. Declarations whose immediate parent is a `@page` margin at-rule
//!    (`PAGE_MARGIN_AT_KEYWORDS`) or a `@font-feature-values` sub-block
//!    (`FONT_FEATURE_VALUE_TYPES`) are ignored.
//! 4. CSS custom property and environment variable substitutions (`var()`, `env()`) are
//!    rejected inside descriptor values unless the enclosing at-rule is in
//!    `VAR_FUNCTION_ACCEPTING_AT_KEYWORDS` (`@function`, `@position-try`).
//! 5. Descriptor values in `@property`, `@counter-style`, `@view-transition`, `@font-face`,
//!    `@font-feature-values`, and `@page` are validated against their W3C CSS specification
//!    grammars.

use oxc_css_parser::{Span, ast::Stylesheet};

use crate::{
    config::RuleSetting,
    context::LintContext,
    rules::Rule,
    utils::{
        FONT_FEATURE_VALUE_TYPES, LENGTH_UNITS, NESTING_SUPPORTED_AT_KEYWORDS,
        PAGE_MARGIN_AT_KEYWORDS, at_rule_descriptors, has_interpolation, in_set_ignore_case,
    },
};

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

/// Formats the diagnostic message when `value` is invalid for `descriptor`.
#[inline]
#[must_use]
fn unknown_descriptor_value_diagnostic(descriptor: &str, value: &str) -> String {
    format!("Unknown value \"{value}\" for descriptor \"{descriptor}\"")
}

crate::define_stx_lint! {
    /// Disallows unknown values for descriptors within at-rules.
    ///
    /// Validates descriptor values inside `@property`, `@counter-style`, `@view-transition`,
    /// `@font-face`, `@font-feature-values`, and `@page` against their W3C grammars, and
    /// rejects `var()` / `env()` substitutions in descriptors outside `@function` and
    /// `@position-try`.
    "at-rule-descriptor-value-no-unknown", correctness, recommended = true,
    short_description = "Disallow unknown values for descriptors within at-rules.";

    #[derive(Debug, Default, Clone, Copy)]
    pub struct AtRuleDescriptorValueNoUnknown;
}

impl AtRuleDescriptorValueNoUnknown {
    /// Creates a new [`AtRuleDescriptorValueNoUnknown`] rule instance from the
    /// given [`RuleSetting`].
    #[must_use]
    pub const fn from_setting(_setting: &RuleSetting) -> Option<Self> {
        Some(Self)
    }
}

/// Represents an enclosing CSS block in the ancestry stack while scanning a
/// stylesheet.
#[derive(Debug, Clone, Copy)]
enum BlockKind<'a> {
    /// An at-rule block (e.g. `@font-face { ... }`) with its raw name without `@`.
    AtRule { name: &'a str },
    /// A qualified rule or non-at-rule block (e.g. `a { ... }`).
    QualifiedRule,
}

impl Rule for AtRuleDescriptorValueNoUnknown {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        if !ctx.source.contains('@') {
            return;
        }

        let mut ancestors = Vec::new();
        self.scan_block(ctx, 0, ctx.source.len(), &mut ancestors);
    }
}

impl AtRuleDescriptorValueNoUnknown {
    /// Recursively scans a block or top-level stylesheet range `[start, end)` for
    /// nested blocks and declarations.
    fn scan_block<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        start: usize,
        end: usize,
        ancestors: &mut Vec<BlockKind<'a>>,
    ) {
        let bytes = ctx.source.as_bytes();
        let mut pos = start;

        while pos < end {
            pos = skip_ws_and_comments(bytes, pos, end);
            if pos >= end {
                break;
            }

            let stmt_start = pos;
            let (stmt_end, terminator, first_colon) = scan_statement(bytes, stmt_start, end);

            match terminator {
                StmtTerminator::OpenBrace(open_brace) => {
                    let close_brace = find_matching_brace(bytes, open_brace + 1, end);
                    let header = ctx.source[stmt_start..open_brace].trim();
                    let block_kind = if let Some(at_name) = parse_standard_at_rule_name(header) {
                        BlockKind::AtRule { name: at_name }
                    } else {
                        BlockKind::QualifiedRule
                    };

                    ancestors.push(block_kind);
                    self.scan_block(ctx, open_brace + 1, close_brace, ancestors);
                    ancestors.pop();

                    pos = (close_brace + 1).min(end);
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
        stmt_start: usize,
        stmt_end: usize,
        first_colon: Option<usize>,
        ancestors: &[BlockKind<'a>],
    ) {
        if ancestors.is_empty() {
            return;
        }

        let Some(colon_pos) = first_colon else {
            return;
        };

        let bytes = ctx.source.as_bytes();
        if bytes.get(stmt_start) == Some(&b'@') {
            return;
        }

        let prop = trim_trailing_ws_and_comments(&ctx.source[stmt_start..colon_pos]);
        if !is_standard_decl_prop(prop) {
            return;
        }

        let val_start = skip_ws_and_comments(bytes, colon_pos + 1, stmt_end);
        let value = ctx.source[val_start..stmt_end].trim_end();
        if value.is_empty() {
            return;
        }

        let val_end = val_start + value.len();
        let val_span = Span {
            start: u32::try_from(val_start).unwrap_or(u32::MAX),
            end: u32::try_from(val_end).unwrap_or(u32::MAX),
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
            if contains_var_or_env_function(&ctx.source[val_start..val_end]) {
                if !in_set_ignore_case(VAR_FUNCTION_ACCEPTING_AT_KEYWORDS, at_rule_name) {
                    ctx.report(
                        Self::NAME,
                        val_span,
                        unknown_descriptor_value_diagnostic(prop, value),
                    );
                }
                continue;
            }

            if !is_valid_descriptor_value(at_rule_name, prop, value) {
                ctx.report(
                    Self::NAME,
                    val_span,
                    unknown_descriptor_value_diagnostic(prop, value),
                );
            }
        }
    }
}

/// Checks whether `value` contains 1 or 2 whitespace-separated tokens that all
/// satisfy `pred`, without allocating a `Vec`.
#[inline]
#[must_use]
fn check_one_or_two_tokens(value: &str, mut pred: impl FnMut(&str) -> bool) -> bool {
    let mut iter = value.split_ascii_whitespace();
    let Some(first) = iter.next() else {
        return false;
    };
    if !pred(first) {
        return false;
    }
    match (iter.next(), iter.next()) {
        (None, _) => true,
        (Some(second), None) => pred(second),
        (Some(_), Some(_)) => false,
    }
}

/// Checks whether `value` matches the W3C specification grammar for `descriptor`
/// inside `at_rule_name`.
#[must_use]
fn is_valid_descriptor_value(at_rule_name: &str, descriptor: &str, value: &str) -> bool {
    if at_rule_name.eq_ignore_ascii_case("property") {
        if descriptor.eq_ignore_ascii_case("syntax") {
            return is_single_quoted_string(value);
        }
        if descriptor.eq_ignore_ascii_case("inherits") {
            return value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false");
        }
        if descriptor.eq_ignore_ascii_case("initial-value") {
            return !value.is_empty();
        }
    } else if at_rule_name.eq_ignore_ascii_case("counter-style") {
        if descriptor.eq_ignore_ascii_case("system") {
            return is_valid_counter_style_system(value);
        }
        if descriptor.eq_ignore_ascii_case("symbols") {
            return value
                .split_ascii_whitespace()
                .all(is_valid_counter_style_symbol);
        }
        if descriptor.eq_ignore_ascii_case("prefix") || descriptor.eq_ignore_ascii_case("suffix") {
            return is_valid_counter_style_symbol(value);
        }
        if descriptor.eq_ignore_ascii_case("negative") {
            return check_one_or_two_tokens(value, is_valid_counter_style_symbol);
        }
        if descriptor.eq_ignore_ascii_case("fallback") {
            return !value.eq_ignore_ascii_case("none") && is_custom_ident(value);
        }
        if descriptor.eq_ignore_ascii_case("speak-as") {
            return in_set_ignore_case(COUNTER_STYLE_SPEAK_AS_KEYWORDS, value)
                || is_custom_ident(value);
        }
    } else if at_rule_name.eq_ignore_ascii_case("view-transition") {
        if descriptor.eq_ignore_ascii_case("navigation") {
            return value.eq_ignore_ascii_case("auto") || value.eq_ignore_ascii_case("none");
        }
        if descriptor.eq_ignore_ascii_case("types") {
            return value.eq_ignore_ascii_case("none")
                || value.split_ascii_whitespace().all(is_custom_ident);
        }
    } else if at_rule_name.eq_ignore_ascii_case("font-face") {
        if descriptor.eq_ignore_ascii_case("font-display") {
            return in_set_ignore_case(FONT_DISPLAY_KEYWORDS, value);
        }
        if descriptor.eq_ignore_ascii_case("ascent-override")
            || descriptor.eq_ignore_ascii_case("descent-override")
            || descriptor.eq_ignore_ascii_case("line-gap-override")
        {
            return check_one_or_two_tokens(value, |p| {
                p.eq_ignore_ascii_case("normal") || is_non_negative_percentage(p)
            });
        }
        if descriptor.eq_ignore_ascii_case("size-adjust") {
            return is_non_negative_percentage(value);
        }
        if descriptor.eq_ignore_ascii_case("font-weight") {
            return is_valid_font_weight_descriptor(value);
        }
        if descriptor.eq_ignore_ascii_case("font-stretch")
            || descriptor.eq_ignore_ascii_case("font-width")
        {
            return check_one_or_two_tokens(value, |p| {
                in_set_ignore_case(FONT_STRETCH_DESCRIPTOR_KEYWORDS, p)
                    || is_non_negative_percentage(p)
            });
        }
        if descriptor.eq_ignore_ascii_case("font-style") {
            return value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("normal")
                || value.eq_ignore_ascii_case("italic")
                || value
                    .get(..7)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("oblique"));
        }
        if descriptor.eq_ignore_ascii_case("font-named-instance") {
            return value.eq_ignore_ascii_case("auto") || is_single_quoted_string(value);
        }
        if descriptor.eq_ignore_ascii_case("font-language-override") {
            return value.eq_ignore_ascii_case("normal") || is_single_quoted_string(value);
        }
        if descriptor.eq_ignore_ascii_case("font-family") {
            return is_single_quoted_string(value)
                || value.split_ascii_whitespace().all(is_custom_ident);
        }
    } else if at_rule_name.eq_ignore_ascii_case("font-feature-values") {
        if descriptor.eq_ignore_ascii_case("font-display") {
            return in_set_ignore_case(FONT_DISPLAY_KEYWORDS, value);
        }
    } else if at_rule_name.eq_ignore_ascii_case("page") {
        if descriptor.eq_ignore_ascii_case("size") {
            return is_valid_page_size(value);
        }
        if descriptor.eq_ignore_ascii_case("orientation")
            || descriptor.eq_ignore_ascii_case("page-orientation")
        {
            return in_set_ignore_case(
                &[
                    "upright",
                    "rotate-left",
                    "rotate-right",
                    "portrait",
                    "landscape",
                ],
                value,
            );
        }
        if descriptor.eq_ignore_ascii_case("marks") {
            return value.eq_ignore_ascii_case("none")
                || value
                    .split_ascii_whitespace()
                    .all(|p| p.eq_ignore_ascii_case("crop") || p.eq_ignore_ascii_case("cross"));
        }
        if descriptor.eq_ignore_ascii_case("bleed") {
            return value.eq_ignore_ascii_case("auto") || is_length(value);
        }
    }

    true
}

/// Validates the `system` descriptor value in `@counter-style`.
#[must_use]
fn is_valid_counter_style_system(value: &str) -> bool {
    let mut iter = value.split_ascii_whitespace();
    match (iter.next(), iter.next(), iter.next()) {
        (Some(single), None, _) => in_set_ignore_case(COUNTER_STYLE_SYSTEM_KEYWORDS, single),
        (Some(first), Some(second), None) if first.eq_ignore_ascii_case("fixed") => {
            second.parse::<i64>().is_ok()
        }
        (Some(first), Some(second), None) if first.eq_ignore_ascii_case("extends") => {
            is_custom_ident(second)
        }
        _ => false,
    }
}

/// Validates a `<symbol>` token (`<string> | <image> | <custom-ident>`) in
/// `@counter-style`.
#[must_use]
fn is_valid_counter_style_symbol(token: &str) -> bool {
    if is_single_quoted_string(token) {
        return true;
    }
    if token
        .get(..4)
        .is_some_and(|p| p.eq_ignore_ascii_case("url("))
        && token.ends_with(')')
    {
        return true;
    }
    is_custom_ident(token)
}

/// Validates the `font-weight` descriptor in `@font-face`.
#[must_use]
fn is_valid_font_weight_descriptor(value: &str) -> bool {
    if value.eq_ignore_ascii_case("auto") {
        return true;
    }
    check_one_or_two_tokens(value, |p| {
        p.eq_ignore_ascii_case("normal")
            || p.eq_ignore_ascii_case("bold")
            || p.parse::<f64>().is_ok_and(|n| (1.0..=1000.0).contains(&n))
    })
}

/// Validates the `size` descriptor in `@page`.
#[must_use]
fn is_valid_page_size(value: &str) -> bool {
    if value.eq_ignore_ascii_case("auto") {
        return true;
    }
    check_one_or_two_tokens(value, is_length)
        || check_one_or_two_tokens(value, |p| {
            p.eq_ignore_ascii_case("portrait")
                || p.eq_ignore_ascii_case("landscape")
                || in_set_ignore_case(PAGE_SIZE_KEYWORDS, p)
        })
}

/// Checks whether `s` is a single quoted CSS string literal (`"..."` or `'...'`).
#[must_use]
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
#[inline]
#[must_use]
fn is_non_negative_percentage(s: &str) -> bool {
    let Some(num_part) = s.strip_suffix('%') else {
        return false;
    };
    num_part.parse::<f64>().is_ok_and(|n| n >= 0.0)
}

/// Checks whether `s` is a `<length>` token or `0`.
#[must_use]
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
#[inline]
#[must_use]
fn is_custom_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Scans `value` and returns `true` if a `var(` or `env(` function call occurs
/// outside of string literals and comments.
#[must_use]
fn contains_var_or_env_function(value: &str) -> bool {
    let bytes = value.as_bytes();
    let end = bytes.len();
    let mut pos = 0;

    while pos < end {
        let b = bytes[pos];
        if b == b'/' && pos + 1 < end && bytes[pos + 1] == b'*' {
            pos = skip_block_comment(bytes, pos + 2, end);
            continue;
        }
        if b == b'"' || b == b'\'' {
            pos = skip_quoted_string(bytes, pos + 1, end, b);
            continue;
        }
        if b.is_ascii_alphabetic() || b == b'-' || b == b'_' {
            let ident_start = pos;
            pos += 1;
            while pos < end
                && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'-' || bytes[pos] == b'_')
            {
                pos += 1;
            }
            if pos < end && bytes[pos] == b'(' {
                let ident = &value[ident_start..pos];
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

/// Strips trailing ASCII whitespace and `/* ... */` comments between a
/// declaration's property name and `:`.
#[must_use]
fn trim_trailing_ws_and_comments(mut s: &str) -> &str {
    loop {
        s = s.trim_end_matches(|c: char| c.is_ascii_whitespace());
        if let Some(before_close) = s.strip_suffix("*/")
            && let Some(open_idx) = before_close.rfind("/*")
        {
            s = &before_close[..open_idx];
            continue;
        }
        return s;
    }
}

/// Extracts a standard-syntax at-rule name from a block header starting with `@`.
#[must_use]
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
    let after_name = rest[name_len..].trim_start();
    // Ignore Less detached ruleset definitions such as `@foo: { ... };`.
    if after_name.starts_with(':') {
        return None;
    }

    Some(name)
}

/// Checks whether `prop` is a standard-syntax CSS property/descriptor identifier.
#[inline]
#[must_use]
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
    OpenBrace(usize),
    Semicolon(usize),
    End,
}

/// Scans from `start` up to `end` to find the end of the current statement (`{`,
/// `;`, or `end`) and records the position of the first top-level `:`.
#[must_use]
fn scan_statement(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> (usize, StmtTerminator, Option<usize>) {
    let mut pos = start;
    let mut paren_depth: usize = 0;
    let mut first_colon: Option<usize> = None;

    while pos < end {
        match bytes[pos] {
            b'/' if pos + 1 < end && bytes[pos + 1] == b'*' => {
                pos = skip_block_comment(bytes, pos + 2, end);
            }
            b'/' if pos + 1 < end
                && bytes[pos + 1] == b'/'
                && (pos == 0 || bytes[pos - 1] != b':') =>
            {
                pos = skip_line_comment(bytes, pos + 2, end);
            }
            b'"' | b'\'' => {
                pos = skip_quoted_string(bytes, pos + 1, end, bytes[pos]);
            }
            b'#' if pos + 1 < end && bytes[pos + 1] == b'{' => {
                let close = find_matching_brace(bytes, pos + 2, end);
                pos = (close + 1).min(end);
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
#[must_use]
fn find_matching_brace(bytes: &[u8], start: usize, end: usize) -> usize {
    let mut pos = start;
    let mut depth: usize = 1;

    while pos < end {
        match bytes[pos] {
            b'/' if pos + 1 < end && bytes[pos + 1] == b'*' => {
                pos = skip_block_comment(bytes, pos + 2, end);
            }
            b'/' if pos + 1 < end
                && bytes[pos + 1] == b'/'
                && (pos == 0 || bytes[pos - 1] != b':') =>
            {
                pos = skip_line_comment(bytes, pos + 2, end);
            }
            b'"' | b'\'' => {
                pos = skip_quoted_string(bytes, pos + 1, end, bytes[pos]);
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

/// Skips whitespace, `/* ... */` block comments, and `// ...` line comments
/// starting from `pos` up to `end`.
#[must_use]
fn skip_ws_and_comments(bytes: &[u8], mut pos: usize, end: usize) -> usize {
    while pos < end {
        if bytes[pos].is_ascii_whitespace() {
            pos += 1;
        } else if bytes[pos] == b'/' && pos + 1 < end && bytes[pos + 1] == b'*' {
            pos = skip_block_comment(bytes, pos + 2, end);
        } else if bytes[pos] == b'/'
            && pos + 1 < end
            && bytes[pos + 1] == b'/'
            && (pos == 0 || bytes[pos - 1] != b':')
        {
            pos = skip_line_comment(bytes, pos + 2, end);
        } else {
            break;
        }
    }
    pos
}

/// Skips past the closing `*/` of a block comment.
#[inline]
#[must_use]
fn skip_block_comment(bytes: &[u8], mut pos: usize, end: usize) -> usize {
    while pos + 1 < end {
        if bytes[pos] == b'*' && bytes[pos + 1] == b'/' {
            return pos + 2;
        }
        pos += 1;
    }
    end
}

/// Skips past the newline terminating a `//` single-line comment.
#[inline]
#[must_use]
fn skip_line_comment(bytes: &[u8], mut pos: usize, end: usize) -> usize {
    while pos < end && bytes[pos] != b'\n' {
        pos += 1;
    }
    pos
}

/// Skips past the closing `quote` character of a string literal, respecting `\` escapes.
#[inline]
#[must_use]
fn skip_quoted_string(bytes: &[u8], mut pos: usize, end: usize, quote: u8) -> usize {
    while pos < end {
        match bytes[pos] {
            b'\\' => pos = (pos + 2).min(end),
            b if b == quote => return pos + 1,
            _ => pos += 1,
        }
    }
    end
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AtRuleDescriptorValueNoUnknown, unknown_descriptor_value_diagnostic as rejected};
    use crate::tester::{Tester, reject, reject_warnings};

    #[test]
    fn default_config() {
        let pass = &[
            "a { color: 0; }",
            "@counter-style foo { system: cyclic; symbols: url(bar.svg); suffix: \" \"; }",
            "@property --foo { syntax: \"<integer>\"; inherits: false; initial-value: 0; }",
            "@view-transition { navigation: auto; }",
            "@foo { bar: 0 }",
            "@media (width > 400px) { a { color: red; } }",
            "a { @media (width > 400px) { color: red; } }",
            "@property --foo { system: bar; }",
            "@page { margin: var(--foo); }",
            "@function --foo() { result: var(--bar); }",
            "@font-face { font-family: \"var(--foo)\"; }",
        ];

        let fail = &[
            reject(
                "@counter-style foo { system: bar; }",
                None,
                rejected("system", "bar"),
                (1, 30, 1, 33),
            ),
            reject(
                "@property --foo { syntax: bar; }",
                None,
                rejected("syntax", "bar"),
                (1, 27, 1, 30),
            ),
            reject_warnings(
                "@property --foo { syntax: bar; inherits: baz; }",
                None,
                vec![
                    (rejected("syntax", "bar"), (1, 27, 1, 30)),
                    (rejected("inherits", "baz"), (1, 42, 1, 45)),
                ],
            ),
            reject(
                "@property --foo { syntax: cyclic; }",
                None,
                rejected("syntax", "cyclic"),
                (1, 27, 1, 33),
            ),
            reject(
                "@font-face { font-weight: var(--foo); }",
                None,
                rejected("font-weight", "var(--foo)"),
                (1, 27, 1, 37),
            ),
            reject(
                "@page { size: var(--foo); }",
                None,
                rejected("size", "var(--foo)"),
                (1, 15, 1, 25),
            ),
            reject(
                "@font-face { @supports (foo: 0) { ascent-override: baz } }",
                None,
                rejected("ascent-override", "baz"),
                (1, 52, 1, 55),
            ),
            reject(
                "@mixin --qux-baz(@contents) {\n\t@supports (overflow-anchor: auto) {\n\t\t@contents;\n\t}\n}\n\nbody {\n\t@apply --qux-baz {\n\t\t@font-face {\n\t\t  ascent-override: foo;\n\t\t}\n\t}\n}",
                None,
                rejected("ascent-override", "foo"),
                (10, 22, 10, 25),
            ),
        ];

        Tester::new(
            AtRuleDescriptorValueNoUnknown::NAME,
            json!(true),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    #[ignore = "indirection via @mixin/@apply not supported"]
    fn with_indirection() {
        let fail = &[reject(
            "@mixin --qux-baz(@contents) {\n\t@supports (overflow-anchor: auto) {\n\t\t@font-face {\n\t\t\t@contents;\n\t\t}\n\t}\n}\n\nbody {\n\t@apply --qux-baz {\n\t\tascent-override: foo;\n\t}\n}",
            None,
            rejected("ascent-override", "foo"),
            (11, 20, 11, 23),
        )];

        Tester::new(AtRuleDescriptorValueNoUnknown::NAME, json!(true), [], fail).test();
    }

    #[test]
    fn edge_cases_and_utf8_safety() {
        let pass = &[
            "@counter-style foo { system: fixed 1; negative: \"(\" \")\"; fallback: decimal; speak-as: numbers; }",
            "@counter-style foo { system: extends decimal; }",
            "@font-face { font-display: swap; font-weight: 100 900; font-stretch: 50% 200%; font-style: oblique 10deg; size-adjust: 100%; }",
            "@page { size: a4 landscape; marks: crop cross; bleed: 6pt; page-orientation: rotate-left; }",
            "@position-try --foo { top: var(--offset); }",
            "@view-transition { navigation: none; types: slide fade; }",
        ];

        let fail = &[
            reject(
                "@counter-style foo { symbols: \"★\" \"☆\"; system: bar; }",
                None,
                rejected("system", "bar"),
                (1, 48, 1, 51),
            ),
            reject(
                "@font-face { font-display: fast; }",
                None,
                rejected("font-display", "fast"),
                (1, 28, 1, 32),
            ),
            reject(
                "@font-face { font-weight: 0; }",
                None,
                rejected("font-weight", "0"),
                (1, 27, 1, 28),
            ),
            reject(
                "@font-face { size-adjust: -10%; }",
                None,
                rejected("size-adjust", "-10%"),
                (1, 27, 1, 31),
            ),
            reject(
                "@counter-style foo { fallback: none; }",
                None,
                rejected("fallback", "none"),
                (1, 32, 1, 36),
            ),
            reject(
                "@view-transition { navigation: invalid; }",
                None,
                rejected("navigation", "invalid"),
                (1, 32, 1, 39),
            ),
            reject(
                "@page { marks: invalid; }",
                None,
                rejected("marks", "invalid"),
                (1, 16, 1, 23),
            ),
            reject(
                "@page { bleed: 10%; }",
                None,
                rejected("bleed", "10%"),
                (1, 16, 1, 19),
            ),
        ];

        Tester::new(
            AtRuleDescriptorValueNoUnknown::NAME,
            json!(true),
            pass,
            fail,
        )
        .test();
    }
}
