//! Rule `at-rule-descriptor-no-unknown`: disallow unknown descriptors for at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-descriptor-no-unknown>

use crate::config::RuleSetting;
use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::{
    FONT_FEATURE_VALUE_TYPES, MARGIN_CONTEXT_PROPERTIES, NESTING_SUPPORTED_AT_KEYWORDS,
    PAGE_CONTEXT_PROPERTIES, PAGE_MARGIN_AT_KEYWORDS, at_rule_descriptors, has_interpolation,
    in_set, in_set_ignore_case,
};
use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;

/// Enforces that descriptors within declaration-containing at-rules (such as
/// `@font-face`, `@property`, `@counter-style`, `@page`, `@position-try`, etc.)
/// are known standard descriptors defined in CSS specifications.
///
/// # What it does
///
/// Checks standard-syntax declarations inside at-rules that do not support
/// general selector nesting and validates each descriptor name against the
/// specification grammar for that at-rule. It also accounts for:
/// - Page context properties directly inside `@page`.
/// - Margin context properties inside page-margin at-rules (e.g. `@top-center`)
///   nested within `@page`.
/// - Custom feature value names inside font-feature-value type blocks (e.g.
///   `@styleset`, `@swash`) nested within `@font-feature-values`.
///
/// # Why is this bad?
///
/// Unknown descriptors inside at-rules such as `@font-face`, `@property`, or
/// `@counter-style` are ignored by browsers and usually indicate typos or
/// confusion between descriptors of different at-rules.
///
/// # Options
///
/// ## Primary Option
///
/// - `true`: enable the rule.
///
/// ```json
/// {
///   "at-rule-descriptor-no-unknown": true
/// }
/// ```
///
/// # Examples
///
/// ## Incorrect
///
/// ```css
/// @property --foo { bar: 0; }
/// @counter-style foo { bar: 0; }
/// @page { bar: 0; }
/// ```
///
/// ## Correct
///
/// ```css
/// @property --foo { syntax: "<integer>"; inherits: false; initial-value: 0; }
/// @counter-style foo { system: cyclic; symbols: url(bar.svg); suffix: " "; }
/// @page { size: landscape; }
/// ```
///
/// # References
///
/// - [Stylelint: `at-rule-descriptor-no-unknown`](https://stylelint.io/user-guide/rules/at-rule-descriptor-no-unknown)
pub struct AtRuleDescriptorNoUnknown;

impl AtRuleDescriptorNoUnknown {
    /// Creates a new [`AtRuleDescriptorNoUnknown`] rule instance from the given
    /// [`RuleSetting`].
    pub fn from_setting(_setting: &RuleSetting) -> Option<Self> {
        Some(Self)
    }

    /// Formats the diagnostic message when an unknown `descriptor` is found inside
    /// `at_rule`.
    fn rejected_message(at_rule: &str, descriptor: &str) -> String {
        format!("Unknown descriptor \"{descriptor}\" for at-rule \"@{at_rule}\"")
    }
}

/// Represents an enclosing CSS block in the ancestry stack while scanning a
/// stylesheet.
#[derive(Clone, Copy)]
enum BlockKind<'a> {
    /// An at-rule block (e.g. `@page { ... }`) with its raw name without `@`.
    AtRule { name: &'a str },
    /// A qualified rule or non-at-rule block (e.g. `a { ... }`).
    QualifiedRule,
}

impl Rule for AtRuleDescriptorNoUnknown {
    fn name(&self) -> &'static str {
        "at-rule-descriptor-no-unknown"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let mut ancestors = Vec::new();
        self.scan_block(ctx, 0, len, &mut ancestors);
    }
}

impl AtRuleDescriptorNoUnknown {
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

    /// Checks whether the statement spanning `[stmt_start, stmt_end)` is a
    /// standard-syntax declaration and validates it against all enclosing
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

        // Ignore statement-level at-rules (e.g. `@import url("foo:bar");`).
        if byte_at(ctx, stmt_start) == Some(b'@') {
            return;
        }

        let raw_prop = ctx.slice_range(stmt_start, colon_pos);
        let prop = raw_prop.trim_end();
        if !is_standard_decl_prop(prop) {
            return;
        }

        let value = ctx.slice_range(colon_pos + 1, stmt_end).trim();
        if value.is_empty() {
            return;
        }

        let prop_len = u32::try_from(prop.len()).unwrap_or(0);
        let prop_span = Span {
            start: stmt_start,
            end: stmt_start + prop_len,
        };

        let immediate_parent = ancestors.last().copied();

        for ancestor in ancestors {
            let BlockKind::AtRule { name: at_rule_name } = *ancestor else {
                continue;
            };

            if in_set_ignore_case(NESTING_SUPPORTED_AT_KEYWORDS, at_rule_name) {
                continue;
            }

            let Some(descriptors) = at_rule_descriptors(at_rule_name) else {
                continue;
            };

            if is_page_context_property_in_page_at_rule(at_rule_name, prop) {
                continue;
            }

            if let Some(BlockKind::AtRule { name: parent_name }) = immediate_parent {
                if is_margin_context_property_in_margin_at_rule(at_rule_name, prop, parent_name) {
                    continue;
                }
                if is_font_feature_value_type(at_rule_name, parent_name) {
                    continue;
                }
            }

            if in_set_ignore_case(descriptors, prop) {
                continue;
            }

            ctx.report(
                self.name(),
                prop_span,
                Self::rejected_message(at_rule_name, prop),
            );
        }
    }
}

/// Checks whether `prop` is a valid page context property inside `@page`.
fn is_page_context_property_in_page_at_rule(at_rule_name: &str, prop: &str) -> bool {
    at_rule_name.eq_ignore_ascii_case("page") && in_set(PAGE_CONTEXT_PROPERTIES, prop)
}

/// Checks whether `prop` is a valid margin context property inside a page-margin
/// at-rule nested within `@page`.
fn is_margin_context_property_in_margin_at_rule(
    at_rule_name: &str,
    prop: &str,
    parent_name: &str,
) -> bool {
    at_rule_name.eq_ignore_ascii_case("page")
        && in_set_ignore_case(PAGE_MARGIN_AT_KEYWORDS, parent_name)
        && in_set(MARGIN_CONTEXT_PROPERTIES, prop)
}

/// Checks whether `parent_name` is a font-feature-value type at-rule nested inside
/// `@font-feature-values`.
fn is_font_feature_value_type(at_rule_name: &str, parent_name: &str) -> bool {
    at_rule_name.eq_ignore_ascii_case("font-feature-values")
        && in_set_ignore_case(FONT_FEATURE_VALUE_TYPES, parent_name)
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

    // Ignore Less mixin definitions/calls such as `@mixin()` or `@foo: { ... }`.
    if after_name.starts_with('(') || after_name.starts_with(':') {
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
