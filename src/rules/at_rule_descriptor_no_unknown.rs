//! Rule: `at-rule-descriptor-no-unknown`
//!
//! Disallow unknown descriptors for at-rules.

use oxc_css_parser::{Span, ast::Stylesheet};

use crate::{
    config::RuleSetting,
    context::LintContext,
    declare_stx_lint,
    rules::Rule,
    utils::{
        FONT_FEATURE_VALUE_TYPES, MARGIN_CONTEXT_PROPERTIES, NESTING_SUPPORTED_AT_KEYWORDS,
        PAGE_CONTEXT_PROPERTIES, PAGE_MARGIN_AT_KEYWORDS, at_rule_descriptors, has_interpolation,
        in_set, in_set_ignore_case,
    },
};

fn unknown_at_rule_descriptor_diagnostic(at_rule: &str, descriptor: &str) -> String {
    format!("Unknown descriptor \"{descriptor}\" for at-rule \"@{at_rule}\"")
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AtRuleDescriptorNoUnknown;

declare_stx_lint! {
    /// Disallow unknown descriptors within declaration-containing at-rules.
    ///
    /// ### What it does
    ///
    /// Checks standard-syntax declarations inside at-rules that do not support
    /// general selector nesting (such as `@font-face`, `@property`, `@counter-style`,
    /// `@page`, `@position-try`, etc.) and validates each descriptor name against the
    /// specification grammar for that at-rule. It also accounts for:
    /// - Page context properties directly inside `@page`.
    /// - Margin context properties inside page-margin at-rules (e.g. `@top-center`)
    ///   nested within `@page`.
    /// - Custom feature value names inside font-feature-value type blocks (e.g.
    ///   `@styleset`, `@swash`) nested within `@font-feature-values`.
    ///
    /// ### Why is this bad?
    ///
    /// Unknown descriptors inside at-rules such as `@font-face`, `@property`, or
    /// `@counter-style` are ignored by browsers and usually indicate typos or
    /// confusion between descriptors of different at-rules.
    ///
    /// ### Options
    ///
    /// #### Primary
    ///
    /// - `true`: Enable the rule.
    ///
    /// ```json
    /// {
    ///   "at-rule-descriptor-no-unknown": true
    /// }
    /// ```
    ///
    /// ### Examples
    ///
    /// **Incorrect:**
    /// ```css
    /// @property --foo { bar: 0; }
    /// @counter-style foo { bar: 0; }
    /// @page { bar: 0; }
    /// ```
    ///
    /// **Correct:**
    /// ```css
    /// @property --foo { syntax: "<integer>"; inherits: false; initial-value: 0; }
    /// @counter-style foo { system: cyclic; symbols: url(bar.svg); suffix: " "; }
    /// @page { size: landscape; }
    /// ```
    ///
    /// ### References
    ///
    /// - [Stylelint Documentation: `at-rule-descriptor-no-unknown`](https://stylelint.io/user-guide/rules/at-rule-descriptor-no-unknown)
    AtRuleDescriptorNoUnknown,
    "at-rule-descriptor-no-unknown",
    correctness,
    fix = none,
    recommended = true,
}

impl AtRuleDescriptorNoUnknown {
    /// Creates a new [`AtRuleDescriptorNoUnknown`] rule instance from the given
    /// [`RuleSetting`].
    pub fn from_setting(_setting: &RuleSetting) -> Option<Self> {
        Some(Self)
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
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        if !ctx.source.contains('@') {
            return;
        }
        let mut ancestors = Vec::new();
        scan_block(ctx, 0, ctx.source.len(), &mut ancestors);
    }
}

/// Recursively scans a block or top-level stylesheet range `[start, end)` for
/// nested blocks and declarations.
fn scan_block<'a>(
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
                let block_kind = match parse_standard_at_rule_name(header) {
                    Some(at_name) => BlockKind::AtRule { name: at_name },
                    None => BlockKind::QualifiedRule,
                };
                ancestors.push(block_kind);
                scan_block(ctx, open_brace + 1, close_brace, ancestors);
                ancestors.pop();
                pos = (close_brace + 1).min(end);
            }
            StmtTerminator::Semicolon(semi_pos) => {
                check_statement_decl(ctx, stmt_start, semi_pos, first_colon, ancestors);
                pos = semi_pos + 1;
            }
            StmtTerminator::End => {
                check_statement_decl(ctx, stmt_start, stmt_end, first_colon, ancestors);
                pos = stmt_end;
            }
        }
    }
}

/// Checks whether the statement spanning `[stmt_start, stmt_end)` is a
/// standard-syntax declaration and validates it against all enclosing
/// descriptor-bearing at-rules.
fn check_statement_decl<'a>(
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

    // Ignore statement-level at-rules (e.g. `@import url("foo:bar");`).
    if ctx.source.as_bytes().get(stmt_start) == Some(&b'@') {
        return;
    }

    let prop = ctx.source[stmt_start..colon_pos].trim_end();
    if !is_standard_decl_prop(prop) {
        return;
    }

    let value = ctx.source[colon_pos + 1..stmt_end].trim();
    if value.is_empty() {
        return;
    }

    let prop_span = Span {
        start: stmt_start as u32,
        end: (stmt_start + prop.len()) as u32,
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
            if is_margin_context_property_in_margin_at_rule(at_rule_name, prop, parent_name)
                || is_font_feature_value_type(at_rule_name, parent_name)
            {
                continue;
            }
        }

        if in_set_ignore_case(descriptors, prop) {
            continue;
        }

        ctx.report(
            AtRuleDescriptorNoUnknown::NAME,
            prop_span,
            unknown_at_rule_descriptor_diagnostic(at_rule_name, prop),
        );
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
    OpenBrace(usize),
    Semicolon(usize),
    End,
}

/// Scans from `start` up to `end` to find the end of the current statement (`{`,
/// `;`, or `end`) and records the position of the first top-level `:`.
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
            b'"' | b'\'' => {
                pos = skip_quoted_string(bytes, pos + 1, end, bytes[pos]);
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
fn find_matching_brace(bytes: &[u8], start: usize, end: usize) -> usize {
    let mut pos = start;
    let mut depth: usize = 1;

    while pos < end {
        match bytes[pos] {
            b'/' if pos + 1 < end && bytes[pos + 1] == b'*' => {
                pos = skip_block_comment(bytes, pos + 2, end);
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

/// Skips whitespace and `/* ... */` comments starting from `start` up to `end`.
fn skip_ws_and_comments(bytes: &[u8], mut pos: usize, end: usize) -> usize {
    while pos < end {
        if bytes[pos].is_ascii_whitespace() {
            pos += 1;
        } else if bytes[pos] == b'/' && pos + 1 < end && bytes[pos + 1] == b'*' {
            pos = skip_block_comment(bytes, pos + 2, end);
        } else {
            break;
        }
    }
    pos
}

/// Skips past the closing `*/` of a block comment.
fn skip_block_comment(bytes: &[u8], mut pos: usize, end: usize) -> usize {
    while pos + 1 < end {
        if bytes[pos] == b'*' && bytes[pos + 1] == b'/' {
            return pos + 2;
        }
        pos += 1;
    }
    end
}

/// Skips past the closing `quote` character of a string literal, respecting `\` escapes.
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

    use super::{AtRuleDescriptorNoUnknown, unknown_at_rule_descriptor_diagnostic};
    use crate::tester::{Tester, reject, reject_warnings};

    fn rejected(at_rule: &str, descriptor: &str) -> String {
        unknown_at_rule_descriptor_diagnostic(
            at_rule.strip_prefix('@').unwrap_or(at_rule),
            descriptor,
        )
    }

    #[test]
    fn default_config() {
        let pass = &[
            "a { unknown: 0; }",
            "@counter-style foo { system: cyclic; symbols: url(bar.svg); suffix: \" \"; }",
            "@property --foo { syntax: \"<integer>\"; inherits: false; initial-value: 0; }",
            "@view-transition { navigation: auto; }",
            "@foo { bar: 0 }",
            "@media (width > 400px) { a { color: red; } }",
            "a { @media (width > 400px) { color: red; } }",
            "@page { size: landscape; }",
            "@page { color: red; }",
            "@page { @top-center { content: \"foo\"; } }",
            "@font-feature-values foo { @styleset { bar: 1; } }",
        ];

        let fail = &[
            reject(
                "@counter-style foo { bar: 0; }",
                None,
                rejected("@counter-style", "bar"),
                (1, 22, 1, 25),
            ),
            reject(
                "@property --foo { bar: 0; }",
                None,
                rejected("@property", "bar"),
                (1, 19, 1, 22),
            ),
            reject_warnings(
                "@property --foo { bar: 0; baz: 0; }",
                None,
                vec![
                    (rejected("@property", "bar"), (1, 19, 1, 22)),
                    (rejected("@property", "baz"), (1, 27, 1, 30)),
                ],
            ),
            reject(
                "@property --foo { system: cyclic; }",
                None,
                rejected("@property", "system"),
                (1, 19, 1, 25),
            ),
            reject(
                "@position-try --foo { @supports (bar: 0) { bar: 0 } }",
                None,
                rejected("@position-try", "bar"),
                (1, 44, 1, 47),
            ),
            reject(
                "@page { bar: 0; }",
                None,
                rejected("@page", "bar"),
                (1, 9, 1, 12),
            ),
            reject(
                "@page { margin-block: 2px 1px; }",
                None,
                rejected("@page", "margin-block"),
                (1, 9, 1, 21),
            ),
            reject(
                "@page { content: \"foo\"; }",
                None,
                rejected("@page", "content"),
                (1, 9, 1, 16),
            ),
            reject(
                "@page { @top-center { bar: 0; } }",
                None,
                rejected("@page", "bar"),
                (1, 23, 1, 26),
            ),
            reject_warnings(
                "@page { foo: 0; @top-center { bar: 0; } }",
                None,
                vec![
                    (rejected("@page", "foo"), (1, 9, 1, 12)),
                    (rejected("@page", "bar"), (1, 31, 1, 34)),
                ],
            ),
            reject(
                "@mixin --qux-baz(@contents) {\n\t@supports (overflow-anchor: auto) {\n\t\t@contents;\n\t}\n}\n\nbody {\n\t@apply --qux-baz {\n\t\t@font-face {\n\t\t  foo: normal;\n\t\t}\n\t}\n}",
                None,
                rejected("@font-face", "foo"),
                (10, 5, 10, 8),
            ),
        ];

        Tester::new(AtRuleDescriptorNoUnknown::NAME, json!(true), pass, fail).test();
    }

    #[test]
    #[ignore = "indirection via @mixin/@apply not supported"]
    fn with_indirection() {
        let fail = &[reject(
            "@mixin --qux-baz(@contents) {\n\t@supports (overflow-anchor: auto) {\n\t\t@font-face {\n\t\t\t@contents;\n\t\t}\n\t}\n}\n\nbody {\n\t@apply --qux-baz {\n\t\tfoo: normal;\n\t}\n}",
            None,
            rejected("@font-face", "foo"),
            (11, 3, 11, 6),
        )];

        Tester::new(AtRuleDescriptorNoUnknown::NAME, json!(true), &[], fail).test();
    }
}
