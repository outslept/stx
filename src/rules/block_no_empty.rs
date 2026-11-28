//! Disallows empty blocks (`block-no-empty`).
//!
//! # Checked Constructs
//!
//! All `{ ... }` blocks in style rules, at-rules (`@media`, `@supports`,
//! `@keyframes`, etc.), and nested rules are inspected:
//!
//! 1. **AST statements (`hasNotableChild` in Stylelint)**:
//!    If the block contains any parsed child declaration, rule, or at-rule,
//!    it is considered non-empty.
//!
//! 2. **Comments**:
//!    - Linter configuration comments (e.g. `/* stylelint-disable */`) never
//!      count as notable content (`isConfigurationComment`).
//!    - Ordinary block comments (`/* ... */`) and preprocessor line comments
//!      (`// ...`) count as notable content by default, unless the secondary
//!      option `{ "ignore": ["comments"] }` is enabled.
//!
//! 3. **Raw non-whitespace characters (`hasNonWhitespace` in Stylelint)**:
//!    Any non-whitespace character outside comments inside `{ ... }` (such as
//!    a lone semicolon `a { ; }`) makes the block non-empty.

use oxc_css_parser::ast::SimpleBlock;
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::{is_configuration_comment, secondary_has_flag};

crate::define_stx_lint! {
    /// Disallow empty blocks.
    ///
    /// Flags empty `{ ... }` blocks in style rules, at-rules, and keyframes.
    /// Linter configuration comments never count as block content, and ordinary
    /// comments can also be ignored with `{ "ignore": ["comments"] }`.
    "block-no-empty", suspicious, recommended = true,
    short_description = "Disallow empty blocks.";
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct BlockNoEmpty {
        /// When `true`, blocks containing only non-configuration comments are also
        /// reported as empty (`"ignore": ["comments"]`).
        pub ignore_comments: bool,
    }
}

/// Diagnostic message matching Stylelint's `messages.rejected`.
const REJECTED_MESSAGE: &str = "Empty block";

impl BlockNoEmpty {
    /// Constructs a configured [`BlockNoEmpty`] rule instance from raw
    /// Stylelint secondary options.
    #[must_use]
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_comments: secondary_has_flag(secondary, "ignore", "comments"),
        }
    }
}

impl Rule for BlockNoEmpty {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        // Fast path: if the AST block has any parsed statements/declarations, it is not empty.
        if !block.statements.is_empty() {
            return;
        }

        // Compute the byte range strictly inside `{` and `}`, guarding against
        // unclosed blocks at EOF.
        let block_raw = ctx.slice_range(block.span.start, block.span.end);
        let inner_start = block.span.start.saturating_add(1);
        let inner_end = if block_raw.ends_with('}') {
            block.span.end.saturating_sub(1)
        } else {
            block.span.end
        };

        if inner_start > inner_end {
            return;
        }

        let mut cursor = inner_start;

        // Inspect all comments located within the block boundaries.
        for comment in ctx.comments_in_range(inner_start, inner_end) {
            // If there is any non-whitespace character before the comment (e.g. a lone `;`),
            // the block is not considered empty (`hasNonWhitespace` in Stylelint).
            if !ctx
                .slice_range(cursor, comment.span.start)
                .trim()
                .is_empty()
            {
                return;
            }

            // Regular comments count as notable content unless `ignore: ["comments"]` is enabled.
            // Configuration comments (e.g. `/* stylelint-disable */`) never count as content.
            if !self.ignore_comments && !is_configuration_comment(comment.content) {
                return;
            }

            cursor = comment.span.end;
        }

        // Check trailing text after the last comment (or the entire inner range if there are no comments)
        // for any non-whitespace characters (such as `{ ; }`).
        if !ctx.slice_range(cursor, inner_end).trim().is_empty() {
            return;
        }

        ctx.report(Self::NAME, block.span, REJECTED_MESSAGE.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = &[
            "",
            "@import \"foo.css\";",
            "a { color: pink; }",
            "a { /* foo */ }",
            "a {\n/* foo */\n}",
            "a {\n/* foo */\n/* bar */\n}",
            "a {\n/*\nfoo\nbar\n*/\n}",
            "a { ; }",
            "@media print { a { color: pink; } }",
            "@media print { a { /* foo */ } }",
            "@media print { a { ; } }",
            "@media print { ; }",
            "@import url(x.css)",
            "a { /* stylelint-disable */ }",
            "a { /* stylelint-disable block-no-empty */ }",
            "a { /* stylelint-disable-line block-no-empty */ }",
        ];

        let fail = &[
            reject("a {}", None, REJECTED_MESSAGE, (1, 3, 1, 5)),
            reject("a { }", None, REJECTED_MESSAGE, (1, 3, 1, 6)),
            reject("a {\n}", None, REJECTED_MESSAGE, (1, 3, 2, 2)),
            reject("a {\r\n}", None, REJECTED_MESSAGE, (1, 3, 2, 2)),
            reject("a {\n\n}", None, REJECTED_MESSAGE, (1, 3, 3, 2)),
            reject("a {\t}", None, REJECTED_MESSAGE, (1, 3, 1, 6)),
            reject("a {\t\t}", None, REJECTED_MESSAGE, (1, 3, 1, 7)),
            reject("@media print {}", None, REJECTED_MESSAGE, (1, 14, 1, 16)),
            reject("@media print {\n}", None, REJECTED_MESSAGE, (1, 14, 2, 2)),
            reject("@media print {\r\n}", None, REJECTED_MESSAGE, (1, 14, 2, 2)),
            reject("@media print {\t}", None, REJECTED_MESSAGE, (1, 14, 1, 17)),
            reject(
                "@media print { a {} }",
                None,
                REJECTED_MESSAGE,
                (1, 18, 1, 20),
            ),
            reject(
                "@media print /*comment*/, screen {}",
                None,
                REJECTED_MESSAGE,
                (1, 34, 1, 36),
            ),
            reject(
                "a { /* stylelint-disable foo */ }",
                None,
                REJECTED_MESSAGE,
                (1, 3, 1, 34),
            ),
        ];

        Tester::new(BlockNoEmpty::NAME, json!([true]), pass, fail).test();
    }

    #[test]
    fn ignore_comments() {
        let pass = &[
            "a { /* foo */ color: pink; }",
            "a {\n/* foo */\ncolor: pink;\n}",
            "@import \"foo.css\";",
            "a { /* stylelint-disable */ }",
        ];

        let fail = &[
            reject("a { /* foo */ }", None, REJECTED_MESSAGE, (1, 3, 1, 16)),
            reject("a {\n/* foo */\n}", None, REJECTED_MESSAGE, (1, 3, 3, 2)),
            reject(
                "a {\n/* foo */\n/* bar */\n}",
                None,
                REJECTED_MESSAGE,
                (1, 3, 4, 2),
            ),
            reject(
                "a {\n/*\nfoo\nbar\n*/\n}",
                None,
                REJECTED_MESSAGE,
                (1, 3, 6, 2),
            ),
            reject(
                "@media print { a { /* foo */ } }",
                None,
                REJECTED_MESSAGE,
                (1, 18, 1, 31),
            ),
            reject(
                "a { /* stylelint-disable foo */ }",
                None,
                REJECTED_MESSAGE,
                (1, 3, 1, 34),
            ),
        ];

        Tester::new(
            BlockNoEmpty::NAME,
            json!([true, { "ignore": ["comments"] }]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    #[ignore = "sugarss is not supported"]
    fn sugarss_syntax() {
        let fail = &[
            reject(".a", None, REJECTED_MESSAGE, (1, 2, 1, 4)),
            reject(
                ".a\n  padding: 25px\n  .a\n",
                None,
                REJECTED_MESSAGE,
                (3, 4, 3, 6),
            ),
        ];

        Tester::new(BlockNoEmpty::NAME, json!([true]), [], fail)
            .with_custom_syntax("sugarss")
            .test();
    }

    #[test]
    fn scss_default() {
        let pass = &[
            "a {\n// foo\n}",
            "a {\n// foo\n// bar\n}",
            "@media print { a {\n// foo\n} }",
        ];

        Tester::new(BlockNoEmpty::NAME, json!([true]), pass, [])
            .with_custom_syntax("postcss-scss")
            .test();
    }

    #[test]
    fn scss_ignore_comments() {
        let pass = &["a {\n// foo\ncolor: pink;\n}"];

        let fail = &[
            reject("a {\n// foo\n}", None, REJECTED_MESSAGE, (1, 3, 3, 2)),
            reject(
                "a {\n// foo\n// bar\n}",
                None,
                REJECTED_MESSAGE,
                (1, 3, 4, 2),
            ),
            reject(
                "@media print { a {\n// foo\n} }",
                None,
                REJECTED_MESSAGE,
                (1, 18, 3, 2),
            ),
        ];

        Tester::new(
            BlockNoEmpty::NAME,
            json!([true, { "ignore": ["comments"] }]),
            pass,
            fail,
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }

    #[test]
    fn edge_cases_and_regressions() {
        let pass = &[
            "@keyframes fade { from { opacity: 0; } to { opacity: 1; } }",
            "a { b { c { color: red; } } }",
            "/* stylelint-disable-next-line block-no-empty */\na {}",
        ];

        let fail = &[
            reject("@keyframes fade {}", None, REJECTED_MESSAGE, (1, 17, 1, 19)),
            reject(
                "@keyframes fade { from {} }",
                None,
                REJECTED_MESSAGE,
                (1, 24, 1, 26),
            ),
            reject("a { b { c {} } }", None, REJECTED_MESSAGE, (1, 11, 1, 13)),
            reject_warnings(
                "a {} b {}",
                None,
                vec![
                    (REJECTED_MESSAGE.to_string(), (1, 3, 1, 5)),
                    (REJECTED_MESSAGE.to_string(), (1, 8, 1, 10)),
                ],
            ),
        ];

        Tester::new(BlockNoEmpty::NAME, json!([true]), pass, fail).test();

        let less_pass = &["a {\n// less comment\n}"];
        let less_fail = &[reject(
            "a {\n// less comment\n}",
            None,
            REJECTED_MESSAGE,
            (1, 3, 3, 2),
        )];

        Tester::new(BlockNoEmpty::NAME, json!([true]), less_pass, [])
            .with_custom_syntax("postcss-less")
            .test();

        Tester::new(
            BlockNoEmpty::NAME,
            json!([true, { "ignore": ["comments"] }]),
            [],
            less_fail,
        )
        .with_custom_syntax("postcss-less")
        .test();
    }
}
