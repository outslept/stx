//! Rule: `block-no-empty`
//!
//! Disallow empty blocks.

use oxc_css_parser::ast::SimpleBlock;
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::{is_configuration_comment, secondary_has_flag};

crate::define_stx_lint! {
    /// Flag empty blocks in style rules, at-rules, and keyframes.
    ///
    /// Enable with `true`. Linter configuration comments never count as content.
    /// Set the secondary option `ignore: ["comments"]` to treat ordinary comments
    /// the same way. Other non-whitespace text, including a lone `;`, makes a
    /// block nonempty for this check.
    ///
    /// ```css
    /// a {}               /* reported */
    /// a { color: red; }  /* accepted */
    /// ```
    "block-no-empty", suspicious, recommended = true;
    #[derive(Default)]
    pub struct BlockNoEmpty {
        /// When `true`, blocks containing only non-configuration comments are also
        /// reported as empty (`"ignore": ["comments"]`).
        pub ignore_comments: bool,
    }
}

impl BlockNoEmpty {
    /// Constructs [`BlockNoEmpty`] from the rule's secondary options object.
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

        // Compute the byte range strictly inside `{` and `}`.
        let inner_start = block.span.start + 1;
        let inner_end = block.span.end.saturating_sub(1);

        let mut cursor = inner_start;

        // Inspect all comments located within the block boundaries.
        for comment in ctx.comments_in_range(inner_start, inner_end) {
            // If there is any non-whitespace character before the comment (e.g. a lone `;`),
            // the block is not considered empty.
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

        ctx.report(self.name(), block.span, "Empty block".to_string());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject};

    const NAME: &str = "block-no-empty";
    const REJECTED: &str = "Empty block";

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
            reject("a {}", None, REJECTED, (1, 3, 1, 5)),
            reject("a { }", None, REJECTED, (1, 3, 1, 6)),
            reject("a {\n}", None, REJECTED, (1, 3, 2, 2)),
            reject("a {\r\n}", None, REJECTED, (1, 3, 2, 2)),
            reject("a {\n\n}", None, REJECTED, (1, 3, 3, 2)),
            reject("a {\t}", None, REJECTED, (1, 3, 1, 6)),
            reject("a {\t\t}", None, REJECTED, (1, 3, 1, 7)),
            reject("@media print {}", None, REJECTED, (1, 14, 1, 16)),
            reject("@media print {\n}", None, REJECTED, (1, 14, 2, 2)),
            reject("@media print {\r\n}", None, REJECTED, (1, 14, 2, 2)),
            reject("@media print {\t}", None, REJECTED, (1, 14, 1, 17)),
            reject("@media print { a {} }", None, REJECTED, (1, 18, 1, 20)),
            reject(
                "@media print /*comment*/, screen {}",
                None,
                REJECTED,
                (1, 34, 1, 36),
            ),
            reject(
                "a { /* stylelint-disable foo */ }",
                None,
                REJECTED,
                (1, 3, 1, 34),
            ),
        ];

        Tester::new(NAME, json!([true]), pass, fail).test();
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
            reject("a { /* foo */ }", None, REJECTED, (1, 3, 1, 16)),
            reject("a {\n/* foo */\n}", None, REJECTED, (1, 3, 3, 2)),
            reject("a {\n/* foo */\n/* bar */\n}", None, REJECTED, (1, 3, 4, 2)),
            reject("a {\n/*\nfoo\nbar\n*/\n}", None, REJECTED, (1, 3, 6, 2)),
            reject(
                "@media print { a { /* foo */ } }",
                None,
                REJECTED,
                (1, 18, 1, 31),
            ),
            reject(
                "a { /* stylelint-disable foo */ }",
                None,
                REJECTED,
                (1, 3, 1, 34),
            ),
        ];

        Tester::new(NAME, json!([true, { "ignore": ["comments"] }]), pass, fail).test();
    }

    #[test]
    #[ignore = "sugarss is not supported"]
    fn sugarss_syntax() {
        let fail = &[
            reject(".a", None, REJECTED, (1, 2, 1, 4)),
            reject(".a\n  padding: 25px\n  .a\n", None, REJECTED, (3, 4, 3, 6)),
        ];

        Tester::new(NAME, json!([true]), [], fail)
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

        Tester::new(NAME, json!([true]), pass, [])
            .with_custom_syntax("postcss-scss")
            .test();
    }

    #[test]
    fn scss_ignore_comments() {
        let pass = &["a {\n// foo\ncolor: pink;\n}"];

        let fail = &[
            reject("a {\n// foo\n}", None, REJECTED, (1, 3, 3, 2)),
            reject("a {\n// foo\n// bar\n}", None, REJECTED, (1, 3, 4, 2)),
            reject(
                "@media print { a {\n// foo\n} }",
                None,
                REJECTED,
                (1, 18, 3, 2),
            ),
        ];

        Tester::new(NAME, json!([true, { "ignore": ["comments"] }]), pass, fail)
            .with_custom_syntax("postcss-scss")
            .test();
    }
}
