//! Rule `at-rule-empty-line-before`: require or disallow an empty line before at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-empty-line-before>

use crate::config::RuleSetting;
use crate::context::LintContext;
use crate::fix::Fix;
use crate::rules::Rule;
use crate::utils::{
    OptionMatcher, has_interpolation, options_matches, parse_option_matchers, secondary_has_flag,
};
use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;

/// Primary expectation mode for [`AtRuleEmptyLineBefore`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EmptyLineMode {
    /// Require an empty line before at-rules (`"always"`).
    Always,
    /// Disallow an empty line before at-rules (`"never"`).
    Never,
}
type Expectation = EmptyLineMode;

/// Requires or disallows an empty line before standard-syntax at-rules.
///
/// # What it does
///
/// Checks the whitespace immediately preceding every standard-syntax at-rule
/// (except the very first node at the root of the stylesheet) and enforces that
/// an empty line is either present (`"always"`) or absent (`"never"`), subject
/// to configured `except`, `ignore`, and `ignoreAtRules` secondary options.
///
/// # Why is this bad?
///
/// Inconsistent vertical spacing before at-rules (`@media`, `@keyframes`,
/// `@import`, `@supports`, etc.) makes stylesheets harder to scan and maintain.
///
/// # Options
///
/// ## Primary Option
///
/// - `"always"`: always require an empty line before at-rules.
/// - `"never"`: never allow an empty line before at-rules.
///
/// ## Secondary Options
///
/// - `except`: array of conditions under which the primary expectation is reversed:
///   - `"after-same-name"`: reverse when the at-rule follows another at-rule with the same name.
///   - `"inside-block"`: reverse when the at-rule is nested inside a block.
///   - `"blockless-after-same-name-blockless"`: reverse when a blockless at-rule follows another blockless at-rule with the same name.
///   - `"blockless-after-blockless"`: reverse when a blockless at-rule follows any other blockless at-rule.
///   - `"first-nested"`: reverse when the at-rule is the first nested statement inside a block.
/// - `ignore`: array of conditions under which the at-rule is ignored:
///   - `"after-comment"`: ignore at-rules immediately following a standalone comment.
///   - `"first-nested"`: ignore the first nested statement inside a block.
///   - `"inside-block"`: ignore at-rules nested inside a block.
///   - `"blockless-after-same-name-blockless"`: ignore blockless at-rules following another blockless at-rule with the same name.
///   - `"blockless-after-blockless"`: ignore blockless at-rules following another blockless at-rule.
/// - `ignoreAtRules`: `Array<string | RegExp> | string | RegExp` of at-rule names (without `@`) to ignore.
///
/// ```json
/// {
///   "at-rule-empty-line-before": [
///     "always",
///     {
///       "except": ["blockless-after-same-name-blockless", "first-nested"],
///       "ignore": ["after-comment"],
///       "ignoreAtRules": ["else"]
///     }
///   ]
/// }
/// ```
///
/// # Examples
///
/// ## Incorrect (with `"always"`)
///
/// ```css
/// a {} @media {}
/// a {}
/// @media {}
/// ```
///
/// ## Correct (with `"always"`)
///
/// ```css
/// a {}
///
/// @media {}
/// ```
///
/// # References
///
/// - [Stylelint: `at-rule-empty-line-before`](https://stylelint.io/user-guide/rules/at-rule-empty-line-before)
pub struct AtRuleEmptyLineBefore {
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

        Some(Self {
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
        })
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
        "at-rule-empty-line-before"
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
            self.name(),
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

    // Skip UTF-8 BOM at the start of the file.
    if pos == 0 && ctx.source.starts_with('\u{feff}') {
        pos = 3;
    }

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
