//! Rule `at-rule-empty-line-before`: require or disallow an empty line before at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-empty-line-before>

use std::ops::Deref;

use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;

use crate::config::RuleSetting;
use crate::context::LintContext;
use crate::declare_stx_lint;
use crate::fix::Fix;
use crate::rules::Rule;
use crate::utils::{
    OptionMatcher, has_interpolation, options_matches, parse_option_matchers, secondary_has_flag,
};

declare_stx_lint! {
    /// Require or disallow an empty line before at-rules.
    AtRuleEmptyLineBefore,
    "at-rule-empty-line-before",
    style,
    fix = fix,
    recommended = false,
}

/// Primary expectation mode for [`AtRuleEmptyLineBefore`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EmptyLineMode {
    /// Require an empty line before at-rules (`"always"`).
    #[default]
    Always,
    /// Disallow an empty line before at-rules (`"never"`).
    Never,
}

type Expectation = EmptyLineMode;

#[derive(Default)]
pub struct AtRuleEmptyLineBeforeConfig {
    expectation: Expectation,
    except_after_same_name: bool,
    except_inside_block: bool,
    except_first_nested: bool,
    except_blockless_after_blockless: bool,
    except_blockless_after_same_name_blockless: bool,
    ignore_after_comment: bool,
    ignore_first_nested: bool,
    ignore_inside_block: bool,
    ignore_blockless_after_blockless: bool,
    ignore_blockless_after_same_name_blockless: bool,
    ignore_at_rules: Vec<OptionMatcher>,
}

/// Requires or disallows an empty line before standard-syntax at-rules.
#[derive(Default)]
pub struct AtRuleEmptyLineBefore(AtRuleEmptyLineBeforeConfig);

impl Deref for AtRuleEmptyLineBefore {
    type Target = AtRuleEmptyLineBeforeConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AtRuleEmptyLineBefore {
    /// Diagnostic message when an empty line is required before an at-rule.
    const EXPECTED_MSG: &'static str = "Expected empty line before at-rule";
    /// Diagnostic message when an empty line is disallowed before an at-rule.
    const REJECTED_MSG: &'static str = "Expected no empty line before at-rule";

    /// Creates a new [`AtRuleEmptyLineBefore`] rule instance from the given
    /// [`RuleSetting`].
    pub fn from_setting(setting: &RuleSetting) -> Option<Self> {
        let expectation = match setting.primary.as_str()? {
            "always" => Expectation::Always,
            "never" => Expectation::Never,
            _ => return None,
        };
        let sec = setting.secondary.as_ref();
        Some(Self(AtRuleEmptyLineBeforeConfig {
            expectation,
            except_after_same_name: secondary_has_flag(sec, "except", "after-same-name"),
            except_inside_block: secondary_has_flag(sec, "except", "inside-block"),
            except_first_nested: secondary_has_flag(sec, "except", "first-nested"),
            except_blockless_after_blockless: secondary_has_flag(
                sec,
                "except",
                "blockless-after-blockless",
            ),
            except_blockless_after_same_name_blockless: secondary_has_flag(
                sec,
                "except",
                "blockless-after-same-name-blockless",
            ),
            ignore_after_comment: secondary_has_flag(sec, "ignore", "after-comment"),
            ignore_first_nested: secondary_has_flag(sec, "ignore", "first-nested"),
            ignore_inside_block: secondary_has_flag(sec, "ignore", "inside-block"),
            ignore_blockless_after_blockless: secondary_has_flag(
                sec,
                "ignore",
                "blockless-after-blockless",
            ),
            ignore_blockless_after_same_name_blockless: secondary_has_flag(
                sec,
                "ignore",
                "blockless-after-same-name-blockless",
            ),
            ignore_at_rules: parse_option_matchers(sec, "ignoreAtRules"),
        }))
    }
}

/// Classification of a child node within a root stylesheet or `{ ... }` block.
enum ChildNodeKind<'a> {
    /// A block or line comment.
    Comment,
    /// An at-rule statement (`@name ...;` or `@name ... { ... }`).
    AtRule {
        /// At-rule name without the leading `@`.
        name: &'a str,
        /// `true` if the at-rule has no `{ ... }` block.
        is_blockless: bool,
        /// `true` if the at-rule uses standard CSS/SCSS syntax (not a Less mixin/variable).
        is_standard: bool,
        /// Byte range `(inner_start, inner_end)` of the `{ ... }` body if not blockless.
        body_range: Option<(u32, u32)>,
    },
    /// A qualified rule or declaration, optionally containing a `{ ... }` body.
    Other {
        /// Byte range `(inner_start, inner_end)` of the `{ ... }` body if present.
        body_range: Option<(u32, u32)>,
    },
}

/// A parsed child node inside a container block or root stylesheet.
struct ChildNode<'a> {
    /// Start offset of the whitespace (`raws.before`) preceding this node.
    before_start: u32,
    /// Start offset of the node itself (e.g. `@` for an at-rule).
    node_start: u32,
    /// End offset of the node.
    node_end: u32,
    /// Kind of child node.
    kind: ChildNodeKind<'a>,
}

impl Rule for AtRuleEmptyLineBefore {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let newline = detect_newline(ctx.source);
        self.check_container(ctx, 0, len, false, newline);
    }
}

impl AtRuleEmptyLineBefore {
    /// Scans all immediate child nodes in `[start, end)` and checks at-rules,
    /// then recurses into any child blocks.
    fn check_container<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        start: u32,
        end: u32,
        is_nested: bool,
        newline: &str,
    ) {
        let nodes = parse_container_nodes(ctx, start, end);
        for (idx, node) in nodes.iter().enumerate() {
            if let ChildNodeKind::AtRule {
                name,
                is_blockless,
                is_standard,
                ..
            } = node.kind
            {
                self.check_at_rule_node(
                    ctx,
                    &nodes,
                    idx,
                    name,
                    is_blockless,
                    is_standard,
                    is_nested,
                    newline,
                );
            }
            // Recurse into nested block body if present.
            let body_range = match node.kind {
                ChildNodeKind::AtRule { body_range, .. } | ChildNodeKind::Other { body_range } => {
                    body_range
                }
                ChildNodeKind::Comment => None,
            };
            if let Some((body_start, body_end)) = body_range {
                self.check_container(ctx, body_start, body_end, true, newline);
            }
        }
    }

    /// Evaluates a single at-rule child node at `idx` inside `nodes`.
    #[allow(clippy::too_many_arguments)]
    fn check_at_rule_node<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        nodes: &[ChildNode<'a>],
        idx: usize,
        name: &'a str,
        is_blockless: bool,
        is_standard: bool,
        is_nested: bool,
        newline: &str,
    ) {
        // Ignore the very first node of the root stylesheet.
        if !is_nested && idx == 0 {
            return;
        }
        if !is_standard {
            return;
        }
        if options_matches(&self.ignore_at_rules, name) {
            return;
        }
        let prev_non_shared = get_previous_non_shared_line_comment_node(ctx, nodes, idx);
        let is_after_blockless = matches!(
            prev_non_shared,
            Some(ChildNode {
                kind: ChildNodeKind::AtRule {
                    is_blockless: true,
                    ..
                },
                ..
            })
        );
        let is_blockless_after_blockless = is_blockless && is_after_blockless;
        let is_after_same_name = matches!(
            prev_non_shared,
            Some(ChildNode {
                kind: ChildNodeKind::AtRule {
                    name: prev_name,
                    ..
                },
                ..
            }) if *prev_name == name
        );
        let is_blockless_after_same_name_blockless =
            is_blockless_after_blockless && is_after_same_name;
        let first_nested = is_nested && is_first_nested(ctx, nodes, idx);
        let after_comment = matches!(
            prev_non_shared,
            Some(ChildNode {
                kind: ChildNodeKind::Comment,
                ..
            })
        );
        // Apply `ignore` conditions.
        if self.ignore_blockless_after_blockless && is_blockless_after_blockless {
            return;
        }
        if self.ignore_first_nested && first_nested {
            return;
        }
        if self.ignore_blockless_after_same_name_blockless && is_blockless_after_same_name_blockless
        {
            return;
        }
        if self.ignore_inside_block && is_nested {
            return;
        }
        if self.ignore_after_comment && after_comment {
            return;
        }
        let node = &nodes[idx];
        let raws_before = ctx.slice_range(node.before_start, node.node_start);
        let has_empty_line_before = has_empty_line(raws_before);
        let mut expect_empty_line_before = self.expectation == Expectation::Always;
        // Apply `except` conditions.
        if (self.except_after_same_name && is_after_same_name)
            || (self.except_inside_block && is_nested)
            || (self.except_first_nested && first_nested)
            || (self.except_blockless_after_blockless && is_blockless_after_blockless)
            || (self.except_blockless_after_same_name_blockless
                && is_blockless_after_same_name_blockless)
        {
            expect_empty_line_before = !expect_empty_line_before;
        }
        if expect_empty_line_before == has_empty_line_before {
            return;
        }
        let message = if expect_empty_line_before {
            Self::EXPECTED_MSG
        } else {
            Self::REJECTED_MSG
        };
        let fixed_before = if expect_empty_line_before {
            add_empty_line(raws_before, newline)
        } else {
            remove_empty_lines(raws_before)
        };
        let fix = Fix::single(
            Span {
                start: node.before_start,
                end: node.node_start,
            },
            fixed_before,
        );
        ctx.report_with_fix(
            Self::NAME,
            Span {
                start: node.node_start,
                end: node.node_end,
            },
            message.to_string(),
            Some(fix),
        );
    }
}

/// Parses `[start, end)` into immediate child nodes (`Comment`, `AtRule`, `Other`).
fn parse_container_nodes<'a>(ctx: &LintContext<'a>, start: u32, end: u32) -> Vec<ChildNode<'a>> {
    let mut nodes = Vec::new();
    let mut pos = start;
    while pos < end {
        let before_start = pos;
        pos = skip_whitespace(ctx, pos, end);
        if pos >= end {
            break;
        }
        let node_start = pos;
        let Some(b) = byte_at(ctx, node_start) else {
            break;
        };
        // Block comment `/* ... */`.
        if b == b'/' && byte_at(ctx, node_start + 1) == Some(b'*') {
            let comment_end = skip_block_comment(ctx, node_start + 2, end);
            nodes.push(ChildNode {
                before_start,
                node_start,
                node_end: comment_end,
                kind: ChildNodeKind::Comment,
            });
            pos = comment_end;
            continue;
        }
        // Single-line comment `// ...`.
        if b == b'/' && byte_at(ctx, node_start + 1) == Some(b'/') {
            let comment_end = skip_line_comment(ctx, node_start + 2, end);
            nodes.push(ChildNode {
                before_start,
                node_start,
                node_end: comment_end,
                kind: ChildNodeKind::Comment,
            });
            pos = comment_end;
            continue;
        }
        // Scan statement up to `;`, `{`, or `end`.
        let terminator = scan_statement_terminator(ctx, node_start, end);
        let (node_end, is_blockless, body_range) = match terminator {
            StmtTerminator::OpenBrace(open_brace) => {
                let close_brace = find_matching_brace(ctx, open_brace + 1, end);
                let mut stmt_end = if close_brace < end {
                    close_brace + 1
                } else {
                    end
                };
                // Consume a trailing `;` immediately after `}` (e.g. `@media {};`).
                if byte_at(ctx, stmt_end) == Some(b';') {
                    stmt_end += 1;
                }
                (stmt_end, false, Some((open_brace + 1, close_brace)))
            }
            StmtTerminator::Semicolon(semi_pos) => (semi_pos + 1, true, None),
            StmtTerminator::End(end_pos) => (end_pos, true, None),
        };
        if b == b'@' {
            let name_start = node_start + 1;
            let name_end = scan_ident_end(ctx, name_start, node_end);
            if name_end > name_start {
                let name = ctx.slice_range(name_start, name_end);
                let prelude_end = body_range.map_or(node_end, |(bs, _)| bs.saturating_sub(1));
                let is_standard = is_standard_at_rule(ctx, name_end, prelude_end, is_blockless);
                nodes.push(ChildNode {
                    before_start,
                    node_start,
                    node_end,
                    kind: ChildNodeKind::AtRule {
                        name,
                        is_blockless,
                        is_standard,
                        body_range,
                    },
                });
                pos = node_end;
                continue;
            }
        }
        nodes.push(ChildNode {
            before_start,
            node_start,
            node_end,
            kind: ChildNodeKind::Other { body_range },
        });
        pos = node_end;
    }
    nodes
}

/// Checks whether an at-rule is standard syntax (ignoring Less variables and
/// Less mixin calls such as `@mixin()`).
fn is_standard_at_rule(
    ctx: &LintContext<'_>,
    name_end: u32,
    prelude_end: u32,
    is_blockless: bool,
) -> bool {
    let after_name = byte_at(ctx, name_end);
    // Less variable `@var: value;`.
    if after_name == Some(b':') {
        return false;
    }
    // Less blockless mixin call `@mixin();` (`!node.nodes && node.raws.afterName === '' && node.params[0] === '('`).
    if is_blockless && after_name == Some(b'(') {
        return false;
    }
    let prelude = ctx.slice_range(name_end, prelude_end).trim();
    if is_blockless && prelude.trim_end_matches(';').trim().is_empty() {
        return false;
    }
    if prelude.starts_with(':') || has_interpolation(prelude) {
        return false;
    }
    true
}

/// Returns the previous sibling node, skipping any comment that shares a line
/// with the node preceding it.
fn get_previous_non_shared_line_comment_node<'a, 'b>(
    ctx: &LintContext<'a>,
    nodes: &'b [ChildNode<'a>],
    idx: usize,
) -> Option<&'b ChildNode<'a>> {
    let mut cur = idx;
    while cur > 0 {
        cur -= 1;
        let candidate = &nodes[cur];
        if matches!(candidate.kind, ChildNodeKind::Comment) && cur > 0 {
            let comment_before = ctx.slice_range(candidate.before_start, candidate.node_start);
            if !comment_before.contains('\n') {
                continue;
            }
        }
        return Some(candidate);
    }
    None
}

/// Returns `true` if `nodes[idx]` is the first nested node in a block, or is
/// only preceded by a comment on the same line as the block's opening `{`.
fn is_first_nested(ctx: &LintContext<'_>, nodes: &[ChildNode<'_>], idx: usize) -> bool {
    if idx == 0 {
        return true;
    }
    if idx == 1 && matches!(nodes[0].kind, ChildNodeKind::Comment) {
        let comment_before = ctx.slice_range(nodes[0].before_start, nodes[0].node_start);
        return !comment_before.contains('\n');
    }
    false
}

/// Checks whether `raws_before` contains at least one empty line (two or more `\n`s).
fn has_empty_line(raws_before: &str) -> bool {
    raws_before.bytes().filter(|&b| b == b'\n').count() >= 2
}

/// Detects the newline sequence (`"\r\n"` or `"\n"`) used in `source`.
fn detect_newline(source: &str) -> &'static str {
    let bytes = source.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            if i > 0 && bytes[i - 1] == b'\r' {
                return "\r\n";
            }
            return "\n";
        }
    }
    "\n"
}

/// Inserts an empty line into `raws_before` using `newline`.
fn add_empty_line(raws_before: &str, newline: &str) -> String {
    if let Some(last_nl) = raws_before.rfind('\n') {
        format!(
            "{}{newline}{}",
            &raws_before[..=last_nl],
            &raws_before[last_nl + 1..]
        )
    } else {
        format!("{newline}{newline}{raws_before}")
    }
}

/// Removes empty lines from `raws_before`, preserving a single line break and
/// trailing indentation.
fn remove_empty_lines(raws_before: &str) -> String {
    let Some(first_nl) = raws_before.find('\n') else {
        return raws_before.to_string();
    };
    let last_nl = raws_before.rfind('\n').unwrap_or(first_nl);
    format!(
        "{}{}",
        &raws_before[..=first_nl],
        &raws_before[last_nl + 1..]
    )
}

/// How a statement inside a container was terminated.
enum StmtTerminator {
    OpenBrace(u32),
    Semicolon(u32),
    End(u32),
}

/// Scans from `start` up to `end` to find the top-level `{`, `;`, or `end`
/// terminating the statement.
fn scan_statement_terminator(ctx: &LintContext<'_>, start: u32, end: u32) -> StmtTerminator {
    let mut pos = start;
    let mut paren_depth: u32 = 0;
    while pos < end {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };
        match b {
            b'/' if byte_at(ctx, pos + 1) == Some(b'*') => {
                pos = skip_block_comment(ctx, pos + 2, end);
            }
            b'/' if byte_at(ctx, pos + 1) == Some(b'/') => {
                pos = skip_line_comment(ctx, pos + 2, end);
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
            b'{' if paren_depth == 0 => {
                return StmtTerminator::OpenBrace(pos);
            }
            b';' if paren_depth == 0 => {
                return StmtTerminator::Semicolon(pos);
            }
            _ => {
                pos += 1;
            }
        }
    }
    StmtTerminator::End(end)
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
            b'/' if byte_at(ctx, pos + 1) == Some(b'/') => {
                pos = skip_line_comment(ctx, pos + 2, end);
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

/// Scans an ASCII identifier (`[a-zA-Z0-9_-]+`) starting at `start`.
fn scan_ident_end(ctx: &LintContext<'_>, start: u32, end: u32) -> u32 {
    let mut pos = start;
    while pos < end {
        let Some(b) = byte_at(ctx, pos) else {
            break;
        };
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
            pos += 1;
        } else {
            break;
        }
    }
    pos
}

/// Skips ASCII whitespace starting from `start` up to `end`.
fn skip_whitespace(ctx: &LintContext<'_>, start: u32, end: u32) -> u32 {
    let mut pos = start;
    while pos < end {
        if byte_at(ctx, pos).is_some_and(|b| b.is_ascii_whitespace()) {
            pos += 1;
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

/// Skips past the end of a single-line `// ...` comment.
fn skip_line_comment(ctx: &LintContext<'_>, start: u32, end: u32) -> u32 {
    let mut pos = start;
    while pos < end {
        if byte_at(ctx, pos) == Some(b'\n') {
            return pos + 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_edit};
    use serde_json::{Value, json};

    const EXPECTED: &str = "Expected empty line before at-rule";
    const REJECTED: &str = "Expected no empty line before at-rule";

    #[allow(clippy::type_complexity)]
    fn assert_fix_and_edits(
        config: Value,
        pass: &[&str],
        cases: &[(
            &'static str,
            &'static str,
            &'static str,
            (usize, usize, usize, usize),
            (u32, u32, &'static str),
        )],
    ) {
        let fail: Vec<_> = cases
            .iter()
            .map(|&(code, fixed, msg, span, _)| reject(code, fixed, msg, span))
            .collect();
        Tester::new(AtRuleEmptyLineBefore::NAME, config.clone(), pass, &fail)
            .with_fix(true)
            .test();

        let fail_edits: Vec<_> = cases
            .iter()
            .map(|&(code, _, msg, span, edit)| reject_edit(code, msg, span, edit))
            .collect();
        Tester::new(AtRuleEmptyLineBefore::NAME, config, [], &fail_edits)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn always_default() {
        let pass = vec![
            "a {} b {}",
            "@font-face {}",
            "a {}\n\n@media {}",
            "a {}\n\n@mEdIa {}",
            "a {}\n\n@MEDIA {}",
            "@keyframes foo {}\n\n@media {}",
            "@kEyFrAmEs foo {}\n\n@MeDia {}",
            "@KEYFRAMES foo {}\n\n@MEDIA {}",
            "@-webkit-keyframes foo {}\n\n@media {}",
            "@-webkit-keyframes foo {}\n\n@-webkit-keyframes bar {}",
            "a {}\r\n\r\n@media {}",
            "a {}\n\r\n@media {}",
            "a {\n\n  @mixin foo;\n}",
        ];

        let cases = vec![
            (
                "a {} @media {}",
                "a {}\n\n @media {}",
                EXPECTED,
                (1, 6, 1, 15),
                (3, 4, "}\n\n"),
            ),
            (
                "a {} @mEdIa {}",
                "a {}\n\n @mEdIa {}",
                EXPECTED,
                (1, 6, 1, 15),
                (3, 4, "}\n\n"),
            ),
            (
                "a {} @MEDIA {}",
                "a {}\n\n @MEDIA {}",
                EXPECTED,
                (1, 6, 1, 15),
                (3, 4, "}\n\n"),
            ),
            (
                "@keyframes foo {} @media {}",
                "@keyframes foo {}\n\n @media {}",
                EXPECTED,
                (1, 19, 1, 28),
                (16, 17, "}\n\n"),
            ),
            (
                "@-webkit-keyframes foo {} @media {}",
                "@-webkit-keyframes foo {}\n\n @media {}",
                EXPECTED,
                (1, 27, 1, 36),
                (24, 25, "}\n\n"),
            ),
            (
                "@-webkit-keyframes foo {} @-webkit-keyframes bar {}",
                "@-webkit-keyframes foo {}\n\n @-webkit-keyframes bar {}",
                EXPECTED,
                (1, 27, 1, 52),
                (24, 25, "}\n\n"),
            ),
            (
                "a {}\n@media {}",
                "a {}\n\n@media {}",
                EXPECTED,
                (2, 1, 2, 10),
                (4, 5, "\n\n"),
            ),
            (
                "a {}\r\n@media {}",
                "a {}\r\n\r\n@media {}",
                EXPECTED,
                (2, 1, 2, 10),
                (5, 6, "\n\r\n"),
            ),
            (
                "a {}\n\n/* comment */\n@media {}",
                "a {}\n\n/* comment */\n\n@media {}",
                EXPECTED,
                (4, 1, 4, 10),
                (19, 20, "\n\n"),
            ),
            (
                "a {}\r\n\r\n/* comment */\r\n@media {}",
                "a {}\r\n\r\n/* comment */\r\n\r\n@media {}",
                EXPECTED,
                (4, 1, 4, 10),
                (22, 23, "\n\r\n"),
            ),
        ];

        assert_fix_and_edits(json!("always"), &pass, &cases);
    }

    #[test]
    fn always_except_blockless_after_blockless_1() {
        let pass = vec![
            "a {} b {}",
            "@font-face {}",
            "a {}\n\n@media {}",
            "a {\n\n  @mixin foo;\n}",
            "@keyframes foo {}\n\n@import 'x.css'",
            "@import 'x.css';\n@import 'y.css'",
            "@import 'x.css';",
        ];

        let cases = vec![
            (
                "@keyframes foo {}\n@import 'x.css'",
                "@keyframes foo {}\n\n@import 'x.css'",
                EXPECTED,
                (2, 1, 2, 16),
                (17, 18, "\n\n"),
            ),
            (
                "@import 'x.css';\n\n@import 'y.css'",
                "@import 'x.css';\n@import 'y.css'",
                REJECTED,
                (3, 1, 3, 16),
                (17, 18, ""),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "except": ["blockless-after-blockless"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_except_blockless_after_blockless_2() {
        let pass = vec![
            "@import 'y.css'; @import 'x.css';",
            "@charset \"UTF-8\";\n@import url(x.css); /* comment */\n@import url(y.css);",
        ];

        let cases = vec![
            (
                "@keyframes foo {}\n@import 'x.css'",
                "@keyframes foo {}\n\n@import 'x.css'",
                EXPECTED,
                (2, 1, 2, 16),
                (17, 18, "\n\n"),
            ),
            (
                "@import 'x.css';\n\n@import 'y.css'",
                "@import 'x.css';\n@import 'y.css'",
                REJECTED,
                (3, 1, 3, 16),
                (17, 18, ""),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "except": ["blockless-after-blockless"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_ignore_blockless_after_blockless() {
        let pass = vec!["@charset 'UTF-8'; @import 'x.css';"];

        let cases = vec![
            (
                "@import 'x.css'; @media {};",
                "@import 'x.css';\n\n @media {};",
                EXPECTED,
                (1, 18, 1, 28),
                (15, 16, ";\n\n"),
            ),
            (
                "@import 'test'; @include mixin(1) { @content; };",
                "@import 'test';\n\n @include mixin(1) { @content; };",
                EXPECTED,
                (1, 17, 1, 49),
                (14, 15, ";\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "ignore": ["blockless-after-blockless"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_ignore_after_comment() {
        let pass = vec![
            "/* foo */\n@media {}",
            "/* foo */\n\n@media{}",
            "/* foo */\r\n\r\n@media {}",
        ];

        let cases = vec![
            (
                "a {} @media {}",
                "a {}\n\n @media {}",
                EXPECTED,
                (1, 6, 1, 15),
                (3, 4, "}\n\n"),
            ),
            (
                "bar {} /* foo */\n@media {}",
                "bar {} /* foo */\n\n@media {}",
                EXPECTED,
                (2, 1, 2, 10),
                (16, 17, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "ignore": ["after-comment"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_ignore_first_nested() {
        let pass = vec![
            "@supports {\n  @media {}\n}",
            "@supports {\n\n  @media {}\n}",
        ];

        let cases = vec![(
            "@supports {\n  @media {}\n  @media{}\n}",
            "@supports {\n  @media {}\n\n  @media{}\n}",
            EXPECTED,
            (3, 3, 3, 11),
            (23, 24, "\n\n"),
        )];

        assert_fix_and_edits(
            json!(["always", { "ignore": ["first-nested"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_except_inside_block() {
        let pass = vec![
            "a {}\n\n@media {}",
            "a {\n  @mixin foo;\n  color: pink;\n}",
            "a {\n  color: pink;\n  @mixin foo;\n}",
        ];

        let cases = vec![
            (
                "a {\n\n  @mixin foo;\n  color: pink;\n}",
                "a {\n  @mixin foo;\n  color: pink;\n}",
                REJECTED,
                (3, 3, 3, 14),
                (4, 5, ""),
            ),
            (
                "a {\n\n  color: pink;\n\n  @mixin foo;\n}",
                "a {\n\n  color: pink;\n  @mixin foo;\n}",
                REJECTED,
                (5, 3, 5, 14),
                (20, 21, ""),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "except": ["inside-block"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_except_first_nested() {
        let pass = vec![
            "a {}\n\n@media {}",
            "a {\n  @mixin foo;\n  color: pink;\n}",
            "a { /* comment */\n  @mixin foo;\n  color: pink;\n}",
        ];

        let cases = vec![
            (
                "a {\n  color: pink;\n  @mixin foo;\n}",
                "a {\n  color: pink;\n\n  @mixin foo;\n}",
                EXPECTED,
                (3, 3, 3, 14),
                (18, 19, "\n\n"),
            ),
            (
                "a {\n\n  @mixin foo;\n  color: pink;\n}",
                "a {\n  @mixin foo;\n  color: pink;\n}",
                REJECTED,
                (3, 3, 3, 14),
                (4, 5, ""),
            ),
            (
                "a {/* comment */\n\n  @mixin foo;\n  color: pink;\n}",
                "a {/* comment */\n  @mixin foo;\n  color: pink;\n}",
                REJECTED,
                (3, 3, 3, 14),
                (17, 18, ""),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "except": ["first-nested"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_ignore_inside_block() {
        let pass = vec![
            "a {}\n\n@media {}",
            "a {\n  @mixin foo;\n  color: pink;\n}",
            "a {\n  color: pink;\n  @mixin foo;\n}",
            "a {\n\n  @mixin foo;\n  color: pink;\n}",
            "a {\n\n  color: pink;\n\n  @mixin foo;\n}",
        ];

        assert_fix_and_edits(
            json!(["always", { "ignore": ["inside-block"] }]),
            &pass,
            &[],
        );
    }

    #[test]
    fn always_except_blockless_and_inside_block() {
        let pass = vec![
            "a {\n  @mixin foo;\n  color: pink;\n}",
            "a {\n  color: pink;\n  @mixin foo;\n}",
            "@keyframes foo {}\n\n@import 'x.css'",
            "@import 'x.css';\n@import 'y.css'",
            "@import 'x.css';",
            "a {\n  @mixin foo;\n  @mixin bar;\n}",
        ];

        let cases = vec![
            (
                "@keyframes foo {}\n@import 'x.css'",
                "@keyframes foo {}\n\n@import 'x.css'",
                EXPECTED,
                (2, 1, 2, 16),
                (17, 18, "\n\n"),
            ),
            (
                "@import 'x.css';\n\n@import 'y.css'",
                "@import 'x.css';\n@import 'y.css'",
                REJECTED,
                (3, 1, 3, 16),
                (17, 18, ""),
            ),
            (
                "a {\n\n  @mixin foo;\n  color: pink;\n}",
                "a {\n  @mixin foo;\n  color: pink;\n}",
                REJECTED,
                (3, 3, 3, 14),
                (4, 5, ""),
            ),
            (
                "a {\n\n  color: pink;\n\n  @mixin foo;\n}",
                "a {\n\n  color: pink;\n  @mixin foo;\n}",
                REJECTED,
                (5, 3, 5, 14),
                (20, 21, ""),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "except": ["blockless-after-blockless", "inside-block"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_default() {
        let pass = vec![
            "a {}\n\nb {}",
            "\n\n@font-face {}",
            "a {}\n@media {}",
            "a {} @media {}",
            "@keyframes foo {}\n@media {}",
            "@keyframes foo {} @media {}",
            "a {\n  @mixin foo;\n}",
        ];

        let cases = vec![
            (
                "a {}\n\n@media {}",
                "a {}\n@media {}",
                REJECTED,
                (3, 1, 3, 10),
                (5, 6, ""),
            ),
            (
                "a {}\r\n\r\n@media {}",
                "a {}\r\n@media {}",
                REJECTED,
                (3, 1, 3, 10),
                (6, 8, ""),
            ),
            (
                "@keyframes foo {}\n/* comment */\n\n@media {}",
                "@keyframes foo {}\n/* comment */\n@media {}",
                REJECTED,
                (4, 1, 4, 10),
                (32, 33, ""),
            ),
            (
                "@keyframes foo {}\r\n/* comment */\r\n\r\n@media {}",
                "@keyframes foo {}\r\n/* comment */\r\n@media {}",
                REJECTED,
                (4, 1, 4, 10),
                (34, 36, ""),
            ),
        ];

        assert_fix_and_edits(json!("never"), &pass, &cases);
    }

    #[test]
    fn never_except_blockless_after_blockless() {
        let pass = vec![
            "a {\n  @mixin foo;\n}",
            "@keyframes foo {}\n@import 'x.css'",
            "@import 'x.css';\n\n@import 'y.css'",
            "@import 'x.css';",
        ];

        let cases = vec![
            (
                "@keyframes foo {}\n\n@import 'x.css'",
                "@keyframes foo {}\n@import 'x.css'",
                REJECTED,
                (3, 1, 3, 16),
                (18, 19, ""),
            ),
            (
                "@import 'x.css';\n@import 'y.css'",
                "@import 'x.css';\n\n@import 'y.css'",
                EXPECTED,
                (2, 1, 2, 16),
                (16, 17, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["never", { "except": ["blockless-after-blockless"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_except_inside_block() {
        let pass = vec![
            "a {\n\n  @mixin foo;\n  color: pink;\n}",
            "a {\n  color: pink;\n\n  @mixin foo;\n}",
        ];

        let cases = vec![
            (
                "a {\n  @mixin foo;\n  color: pink;\n}",
                "a {\n\n  @mixin foo;\n  color: pink;\n}",
                EXPECTED,
                (2, 3, 2, 14),
                (3, 4, "\n\n"),
            ),
            (
                "a {\n\n  color: pink;\n  @mixin foo;\n}",
                "a {\n\n  color: pink;\n\n  @mixin foo;\n}",
                EXPECTED,
                (4, 3, 4, 14),
                (19, 20, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["never", { "except": ["inside-block"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_except_first_nested() {
        let pass = vec!["a {\n\n  @mixin foo;\n  color: pink;\n}"];

        let cases = vec![
            (
                "a {\n  color: pink;\n\n  @mixin foo;\n}",
                "a {\n  color: pink;\n  @mixin foo;\n}",
                REJECTED,
                (4, 3, 4, 14),
                (19, 20, ""),
            ),
            (
                "a {\n  @mixin foo;\n  color: pink;\n}",
                "a {\n\n  @mixin foo;\n  color: pink;\n}",
                EXPECTED,
                (2, 3, 2, 14),
                (3, 4, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["never", { "except": ["first-nested"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_ignore_blockless_after_blockless() {
        let pass = vec![
            "\n      @media {};\n      @import 'x.css';\n    ",
            "\n      @import 'x.css';\n      @import 'y.css';\n    ",
            "\n      @import 'x.css';\n\n      @import 'y.css';\n    ",
        ];

        let cases = vec![(
            "\n      @import 'x.css';\n\n      @media {};\n    ",
            "\n      @import 'x.css';\n      @media {};\n    ",
            REJECTED,
            (4, 7, 4, 17),
            (24, 25, ""),
        )];

        assert_fix_and_edits(
            json!(["never", { "ignore": ["blockless-after-blockless"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_ignore_after_comment() {
        let pass = vec![
            "/* foo */\n@media {}",
            "/* foo */\r\n\r\n@media {}",
            "/* foo */\n\n@media {}",
        ];

        let cases = vec![
            (
                "b {}\n\n@media {}",
                "b {}\n@media {}",
                REJECTED,
                (3, 1, 3, 10),
                (5, 6, ""),
            ),
            (
                "b {}\r\n\r\n@media {}",
                "b {}\r\n@media {}",
                REJECTED,
                (3, 1, 3, 10),
                (6, 8, ""),
            ),
            (
                "b {} /* comment */\n\n@media {}",
                "b {} /* comment */\n@media {}",
                REJECTED,
                (3, 1, 3, 10),
                (19, 20, ""),
            ),
        ];

        assert_fix_and_edits(
            json!(["never", { "ignore": ["after-comment"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_ignore_inside_block() {
        let pass = vec![
            "a {\n  @mixin foo;\n  color: pink;\n}",
            "a {\n  color: pink;\n  @mixin foo;\n}",
            "a {\n\n  @mixin foo;\n  color: pink;\n}",
            "a {\n\n  color: pink;\n\n  @mixin foo;\n}",
        ];

        assert_fix_and_edits(json!(["never", { "ignore": ["inside-block"] }]), &pass, &[]);
    }

    #[test]
    fn always_ignore_at_rules_else() {
        let pass = vec![
            "\n      @if(true) {\n      } @else {\n      }\n    ",
            "\n      @if(true) {\n      }\n      @else {\n      }\n    ",
            "\n      @if(true) {}\n\n      @else if(true) {\n      }  @else {\n      }\n    ",
        ];

        let cases = vec![
            (
                "\n      @if(true) {\n      } @else-mixin {\n      }\n    ",
                "\n      @if(true) {\n      }\n\n @else-mixin {\n      }\n    ",
                EXPECTED,
                (3, 9, 4, 8),
                (25, 26, "}\n\n"),
            ),
            (
                "\n      @if (true) {}\n      @if (false) {\n      }\n    ",
                "\n      @if (true) {}\n\n      @if (false) {\n      }\n    ",
                EXPECTED,
                (3, 7, 4, 8),
                (20, 21, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "ignoreAtRules": ["else"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_ignore_at_rules_regex_prefix() {
        let pass = vec!["\n\t\t\ta {}\n\t\t\t@--my-at-rule {}\n\t\t"];

        Tester::new(
            AtRuleEmptyLineBefore::NAME,
            json!(["always", { "ignoreAtRules": "/^--my-/" }]),
            &pass,
            &[],
        )
        .test();
    }

    #[test]
    fn always_ignore_at_rules_regex_el() {
        let pass = vec![
            "@keyframes {}; @an-element-mixin();",
            "\n      @if true {}\n      @else {\n      }\n    ",
        ];

        let cases = vec![(
            "\n      @else {\n        color: pink;\n      }\n      @if true {}\n    ",
            "\n      @else {\n        color: pink;\n      }\n\n      @if true {}\n    ",
            EXPECTED,
            (5, 7, 5, 18),
            (43, 44, "\n\n"),
        )];

        assert_fix_and_edits(
            json!(["always", { "ignoreAtRules": "/el/" }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_ignore_at_rules_else() {
        let pass = vec![
            "\n      @if(true) {\n      } @else {\n      }\n    ",
            "\n      @if(true) {\n      }\n\n      @else {\n      }\n    ",
            "\n      @if(true) {}\n\n      @else if(true) {}\n\n      @else {}\n    ",
        ];

        let cases = vec![
            (
                "\n      @if(true) {\n      }\n\n      @else-mixin {}\n    ",
                "\n      @if(true) {\n      }\n      @else-mixin {}\n    ",
                REJECTED,
                (5, 7, 5, 21),
                (27, 28, ""),
            ),
            (
                "\n      @if (true)\n      {}\n\n      @if (false) {\n      }\n    ",
                "\n      @if (true)\n      {}\n      @if (false) {\n      }\n    ",
                REJECTED,
                (5, 7, 6, 8),
                (27, 28, ""),
            ),
        ];

        assert_fix_and_edits(
            json!(["never", { "ignoreAtRules": ["else"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_ignore_at_rules_regex_el() {
        let pass = vec![
            "\n      @keyframes {}\n\n      @someelement-mixin();\n    ",
            "\n      @if true {}\n\n      @else {\n      }\n    ",
        ];

        let cases = vec![(
            "\n      @else {\n        color: pink;\n      }\n\n      @if true {}\n    ",
            "\n      @else {\n        color: pink;\n      }\n      @if true {}\n    ",
            REJECTED,
            (6, 7, 6, 18),
            (44, 45, ""),
        )];

        assert_fix_and_edits(json!(["never", { "ignoreAtRules": "/el/" }]), &pass, &cases);
    }

    #[test]
    fn always_ignore_blockless_after_same_name_blockless() {
        let pass = vec![
            "@charset \"UTF-8\";\n\n@import url(x.css);\n@import url(y.css);",
            "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include loop;\n  @include doo;\n}",
            "@charset \"UTF-8\";\n\n@import url(x.css); /* comment */\n@import url(y.css);",
        ];

        let cases = vec![
            (
                "@charset \"UTF-8\";\n@import url(x.css);\n@import url(y.css);",
                "@charset \"UTF-8\";\n\n@import url(x.css);\n@import url(y.css);",
                EXPECTED,
                (2, 1, 2, 20),
                (17, 18, "\n\n"),
            ),
            (
                "a {\n\n  @extends .foo;\n  @extends .bar;\n  @include loop;\n  @include doo;\n}",
                "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include loop;\n  @include doo;\n}",
                EXPECTED,
                (5, 3, 5, 17),
                (38, 39, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "ignore": ["blockless-after-same-name-blockless"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_except_blockless_after_same_name_blockless() {
        let pass = vec![
            "@charset \"UTF-8\";\n\n@import url(x.css);\n@import url(y.css);",
            "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include loop;\n  @include doo;\n}",
            "@charset \"UTF-8\";\n\n@import url(x.css); /* comment */\n@import url(y.css);",
        ];

        let cases = vec![
            (
                "@charset \"UTF-8\";\n@import url(x.css);\n@import url(y.css);",
                "@charset \"UTF-8\";\n\n@import url(x.css);\n@import url(y.css);",
                EXPECTED,
                (2, 1, 2, 20),
                (17, 18, "\n\n"),
            ),
            (
                "a {\n\n  @extends .foo;\n  @extends .bar;\n  @include loop;\n  @include doo;\n}",
                "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include loop;\n  @include doo;\n}",
                EXPECTED,
                (5, 3, 5, 17),
                (38, 39, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "except": ["blockless-after-same-name-blockless"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_except_blockless_after_same_name_blockless() {
        let pass = vec![
            "@charset \"UTF-8\";\n@import url(x.css);\n\n@import url(y.css);",
            "a {\n  @extends .foo;\n\n  @extends .bar;\n  @include loop;\n\n  @include doo;\n}",
        ];

        let cases = vec![
            (
                "@charset \"UTF-8\";\n@import url(x.css);\n@import url(y.css);",
                "@charset \"UTF-8\";\n@import url(x.css);\n\n@import url(y.css);",
                EXPECTED,
                (3, 1, 3, 20),
                (37, 38, "\n\n"),
            ),
            (
                "a {\n  @extends .bar;\n  @include loop;\n  @include doo;\n}",
                "a {\n  @extends .bar;\n  @include loop;\n\n  @include doo;\n}",
                EXPECTED,
                (4, 3, 4, 16),
                (37, 38, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["never", { "except": ["blockless-after-same-name-blockless"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn always_except_after_same_name() {
        let pass = vec![
            "@charset \"UTF-8\";\n\n@include x;\n@include y {}",
            "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include y {}\n  @include x;\n}",
        ];

        let cases = vec![
            (
                "@charset \"UTF-8\";\n@include x;\n@include y {}",
                "@charset \"UTF-8\";\n\n@include x;\n@include y {}",
                EXPECTED,
                (2, 1, 2, 12),
                (17, 18, "\n\n"),
            ),
            (
                "a {\n\n  @extends .foo;\n  @extends .bar;\n  @include y {}\n  @include x;\n}",
                "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include y {}\n  @include x;\n}",
                EXPECTED,
                (5, 3, 5, 16),
                (38, 39, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["always", { "except": ["after-same-name"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn never_except_after_same_name() {
        let pass = vec![
            "@charset \"UTF-8\";\n@include x;\n\n@include y {}",
            "a {\n  @extends .foo;\n\n  @extends .bar;\n  @include y {}\n\n  @include x;\n}",
        ];

        let cases = vec![
            (
                "@charset \"UTF-8\";\n@include x;\n@include y {}",
                "@charset \"UTF-8\";\n@include x;\n\n@include y {}",
                EXPECTED,
                (3, 1, 3, 14),
                (29, 30, "\n\n"),
            ),
            (
                "a {\n  @extends .bar;\n  @include x;\n  @include y {}\n}",
                "a {\n  @extends .bar;\n  @include x;\n\n  @include y {}\n}",
                EXPECTED,
                (4, 3, 4, 16),
                (34, 35, "\n\n"),
            ),
        ];

        assert_fix_and_edits(
            json!(["never", { "except": ["after-same-name"] }]),
            &pass,
            &cases,
        );
    }

    #[test]
    fn less_syntax() {
        let pass = vec![
            "\n        .someMixin() { margin: 0; }\n        span { .someMixin(); }\n      ",
            "\n        @myVariable: #f7f8f9;\n        span { background-color: @myVariable; }\n      ",
        ];

        Tester::new(AtRuleEmptyLineBefore::NAME, json!("always"), &pass, &[])
            .with_custom_syntax("postcss-less")
            .with_compute_edit_info(true)
            .test();
    }
}
