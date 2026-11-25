//! Rule `block-no-redundant-nested-style-rules`: disallow redundant nested style rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/block-no-redundant-nested-style-rules>

use oxc_css_parser::{Span, ast::Stylesheet};

use crate::context::LintContext;
use crate::rules::Rule;

crate::define_stx_lint! {
    /// Flag nested style rules whose selector is only `&`.
    ///
    /// Enable with `true`; there are no rule-specific secondary options. The
    /// check ignores whitespace and comments after `&`, but does not flag selectors
    /// that add a qualifier or combine `&` with another selector.
    ///
    /// ```css
    /// a { & { color: red; } }       /* reported */
    /// a { &:hover { color: red; } } /* accepted */
    /// ```
    "block-no-redundant-nested-style-rules", suspicious, recommended = true;
    #[derive(Default)]
    pub struct BlockNoRedundantNestedStyleRules;
}

impl BlockNoRedundantNestedStyleRules {
    /// Diagnostic message reported when a redundant nested `&` style rule is found.
    const REJECTED_MESSAGE: &'static str = "Redundant nested style rule";
}

impl Rule for BlockNoRedundantNestedStyleRules {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let len = u32::try_from(ctx.source.len()).unwrap_or(u32::MAX);
        let mut i: u32 = 0;
        let mut brace_depth: u32 = 0;
        let mut paren_depth: u32 = 0;
        let mut stmt_start: Option<u32> = None;
        while i < len {
            let b = byte_at(ctx, i);
            // Skip whitespace.
            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }
            // Skip block comments `/* ... */`.
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
                if stmt_start.is_none() {
                    stmt_start = Some(i);
                }
                i = skip_string(ctx, i, len);
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
                    && is_sole_nesting_selector(ctx, start, i)
                    && let Some(close_brace) = find_matching_brace(ctx, i, len)
                {
                    ctx.report(
                        self.name(),
                        Span {
                            start,
                            end: close_brace + 1,
                        },
                        Self::REJECTED_MESSAGE.to_string(),
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
fn is_sole_nesting_selector(ctx: &LintContext<'_>, start: u32, open_brace: u32) -> bool {
    if start >= open_brace || byte_at(ctx, start) != b'&' {
        return false;
    }
    let mut pos = start + 1;
    while pos < open_brace {
        let b = byte_at(ctx, pos);
        if b.is_ascii_whitespace() {
            pos += 1;
            continue;
        }
        if b == b'/' && pos + 1 < open_brace && byte_at(ctx, pos + 1) == b'*' {
            pos = skip_block_comment(ctx, pos, open_brace);
            continue;
        }
        if b == b'/' && pos + 1 < open_brace && byte_at(ctx, pos + 1) == b'/' {
            pos = skip_line_comment(ctx, pos, open_brace);
            continue;
        }
        return false;
    }
    true
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject, reject_warnings};

    const REJECTED: &str = "Redundant nested style rule";

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
            reject("a { & {} }", None, REJECTED, (1, 5, 1, 9)),
            reject("a { @media all { & {} } }", None, REJECTED, (1, 18, 1, 22)),
            reject("@scope (a) { & {} }", None, REJECTED, (1, 14, 1, 18)),
            reject("a { @media all {} & {} }", None, REJECTED, (1, 19, 1, 23)),
            reject_warnings(
                "a { & {} & {} }",
                None,
                vec![
                    (REJECTED.into(), (1, 5, 1, 9)),
                    (REJECTED.into(), (1, 10, 1, 14)),
                ],
            ),
            reject("a { & /* foo */ {} }", None, REJECTED, (1, 5, 1, 19)),
            reject("a { /* foo */ & {} }", None, REJECTED, (1, 15, 1, 19)),
        ];

        Tester::new(
            "block-no-redundant-nested-style-rules",
            json!([true]),
            pass,
            fail,
        )
        .test();
    }
}
