//! Disallows redundant nested style rules (`block-no-redundant-nested-style-rules`).
//!
//! # Checked Constructs
//!
//! Style rules nested inside another block (such as a parent style rule,
//! `@media`, or `@scope`) are inspected:
//!
//! 1. **Sole `&` selector (`ruleNode.selector === '&'` in Stylelint)**:
//!    Any nested style rule whose selector consists solely of `&` (ignoring
//!    surrounding whitespace and comments before `{`, which PostCSS stores in
//!    `ruleNode.raws.between`) is reported as redundant.
//!
//! 2. **Root-level rules (`isRoot(parent)` in Stylelint)**:
//!    Top-level `& {}` rules at the stylesheet root are not nested and are
//!    ignored.
//!
//! 3. **Compound, complex, and interpolated selectors**:
//!    Nested selectors that qualify or combine `&` with other selector tokens
//!    (e.g. `&:hover`, `&.foo`, `&#foo`, `&[foo]`, `& .foo`, `& > .foo`,
//!    `&::before`, or SCSS `#{$foo} &`) are allowed.
//!
use oxc_css_parser::{Span, ast::Stylesheet};

use crate::context::LintContext;
use crate::rules::Rule;

crate::define_stx_lint! {
    /// Disallow redundant nested style rules.
    ///
    /// Flags nested style rules whose selector is solely `&` inside another
    /// style rule or at-rule block (such as `@media` or `@scope`).
    "block-no-redundant-nested-style-rules", suspicious, recommended = true,
    short_description = "Disallow redundant nested style rules.";
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct BlockNoRedundantNestedStyleRules;
}

/// Diagnostic message matching Stylelint's `messages.rejected`.
const REJECTED_MESSAGE: &str = "Redundant nested style rule";

impl Rule for BlockNoRedundantNestedStyleRules {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let bytes = ctx.source.as_bytes();
        let len = bytes.len();
        let mut i: usize = 0;
        let mut brace_depth: u32 = 0;
        let mut paren_depth: u32 = 0;
        let mut stmt_start: Option<usize> = None;

        while i < len {
            let b = bytes[i];

            // Skip whitespace.
            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            // Skip block comments `/* ... */`.
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
                if stmt_start.is_none() {
                    stmt_start = Some(i);
                }
                i = skip_string(bytes, i);
                continue;
            }

            // Skip SCSS `#{...}` interpolation without altering `brace_depth` or resetting `stmt_start`.
            if b == b'#' && i + 1 < len && bytes[i + 1] == b'{' {
                if stmt_start.is_none() {
                    stmt_start = Some(i);
                }
                i = find_matching_brace(bytes, i + 1).map_or(len, |close| close + 1);
                continue;
            }

            if b == b'(' {
                if stmt_start.is_none() {
                    stmt_start = Some(i);
                }
                paren_depth += 1;
                i += 1;
                continue;
            }

            if b == b')' {
                paren_depth = paren_depth.saturating_sub(1);
                i += 1;
                continue;
            }

            if b == b';' && paren_depth == 0 {
                stmt_start = None;
                i += 1;
                continue;
            }

            if b == b'}' && paren_depth == 0 {
                brace_depth = brace_depth.saturating_sub(1);
                stmt_start = None;
                i += 1;
                continue;
            }

            if b == b'{' && paren_depth == 0 {
                // Only check rules nested inside another block (`brace_depth >= 1`).
                if brace_depth >= 1
                    && let Some(start) = stmt_start
                    && is_sole_nesting_selector(bytes, start, i)
                    && let Some(close_brace) = find_matching_brace(bytes, i)
                    && let (Ok(span_start), Ok(span_end)) =
                        (u32::try_from(start), u32::try_from(close_brace + 1))
                {
                    ctx.report(
                        Self::NAME,
                        Span {
                            start: span_start,
                            end: span_end,
                        },
                        REJECTED_MESSAGE.to_string(),
                    );
                }

                brace_depth += 1;
                stmt_start = None;
                i += 1;
                continue;
            }

            if stmt_start.is_none() {
                stmt_start = Some(i);
            }
            i += 1;
        }
    }
}

/// Returns `true` if the selector in `start..open_brace` consists solely of `&`
/// followed only by optional whitespace and comments (`raws.between` in PostCSS).
#[must_use]
fn is_sole_nesting_selector(bytes: &[u8], start: usize, open_brace: usize) -> bool {
    if start >= open_brace || bytes[start] != b'&' {
        return false;
    }

    let mut pos = start + 1;
    while pos < open_brace {
        let b = bytes[pos];
        if b.is_ascii_whitespace() {
            pos += 1;
            continue;
        }
        if b == b'/' && pos + 1 < open_brace && bytes[pos + 1] == b'*' {
            pos = skip_block_comment_bounded(bytes, pos, open_brace);
            continue;
        }
        if b == b'/' && pos + 1 < open_brace && bytes[pos + 1] == b'/' {
            pos = skip_line_comment_bounded(bytes, pos, open_brace);
            continue;
        }
        return false;
    }

    true
}

/// Finds the matching `}` for an opening `{` at `open_brace_pos`.
#[must_use]
fn find_matching_brace(bytes: &[u8], open_brace_pos: usize) -> Option<usize> {
    let len = bytes.len();
    let mut depth: u32 = 0;
    let mut pos = open_brace_pos;

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
#[must_use]
fn skip_string(bytes: &[u8], mut pos: usize) -> usize {
    let len = bytes.len();
    let quote = bytes[pos];
    pos += 1;
    while pos < len {
        let cur = bytes[pos];
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn default_options() {
        let pass = &[
            "",
            "a {}",
            "& {}",
            "a { &:hover {} }",
            "a { &.foo {} }",
            "a { &#foo {} }",
            "a { &[foo] {} }",
            "a { & .foo {} }",
            "a { & > .foo {} }",
            "a { &::before { content: \"\"; } }",
            "a & {}",
            "@media all { a {} }",
            "a { @media all { color: red; } }",
            "a { /* comment */ &.foo {} }",
            "a { & /* comment */ .foo {} }",
        ];

        let fail = &[
            reject("a { & {} }", None, REJECTED_MESSAGE, (1, 5, 1, 9)),
            reject(
                "a { @media all { & {} } }",
                None,
                REJECTED_MESSAGE,
                (1, 18, 1, 22),
            ),
            reject(
                "@scope (a) { & {} }",
                None,
                REJECTED_MESSAGE,
                (1, 14, 1, 18),
            ),
            reject(
                "a { @media all {} & {} }",
                None,
                REJECTED_MESSAGE,
                (1, 19, 1, 23),
            ),
            reject_warnings(
                "a { & {} & {} }",
                None,
                vec![
                    (REJECTED_MESSAGE.into(), (1, 5, 1, 9)),
                    (REJECTED_MESSAGE.into(), (1, 10, 1, 14)),
                ],
            ),
            reject(
                "a { & /* foo */ {} }",
                None,
                REJECTED_MESSAGE,
                (1, 5, 1, 19),
            ),
            reject(
                "a { /* foo */ & {} }",
                None,
                REJECTED_MESSAGE,
                (1, 15, 1, 19),
            ),
        ];

        Tester::new(
            BlockNoRedundantNestedStyleRules::NAME,
            json!([true]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn edge_cases_and_regressions() {
        let pass = &[
            "/* Привет — мир */ a { content: \"Привет\"; &.foo {} }",
            "a { &, .foo {} }",
            "a { .foo, & {} }",
        ];

        let fail = &[
            reject_warnings(
                "a { & { & {} } }",
                None,
                vec![
                    (REJECTED_MESSAGE.into(), (1, 5, 1, 15)),
                    (REJECTED_MESSAGE.into(), (1, 9, 1, 13)),
                ],
            ),
            reject(
                "/* Привет */ a { & { color: red; } }",
                None,
                REJECTED_MESSAGE,
                (1, 18, 1, 35),
            ),
        ];

        Tester::new(
            BlockNoRedundantNestedStyleRules::NAME,
            json!([true]),
            pass,
            fail,
        )
        .test();

        let scss_pass = &[
            "a { #{$foo} & {} }",
            "a { & #{$foo} {} }",
            "a { &-#{$foo} {} }",
        ];

        let scss_fail = &[
            reject(
                "a {\n  & // line comment\n  {\n    color: red;\n  }\n}",
                None,
                REJECTED_MESSAGE,
                (2, 3, 5, 4),
            ),
            reject(
                "#{$selector} { & {} }",
                None,
                REJECTED_MESSAGE,
                (1, 16, 1, 20),
            ),
        ];

        Tester::new(
            BlockNoRedundantNestedStyleRules::NAME,
            json!([true]),
            scss_pass,
            scss_fail,
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
