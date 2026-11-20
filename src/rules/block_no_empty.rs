//! Rule: `block-no-empty`
//!
//! Disallow empty blocks.

use oxc_css_parser::ast::SimpleBlock;
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::{is_configuration_comment, secondary_has_flag};

/// ## `block-no-empty`
///
/// Disallow empty blocks in style rules, at-rules, and keyframe blocks.
///
/// ### What it does
///
/// Reports a diagnostic whenever a block `{ ... }` contains no statements,
/// declarations, or non-whitespace content.
///
/// ### Why is this bad?
///
/// Empty blocks are typically leftovers from refactoring or incomplete edits.
/// They add unnecessary bytes to stylesheets and clutter the codebase without
/// having any effect on rendering.
///
/// ### Options
///
/// #### Primary
///
/// - `true`: Enable the rule.
///
/// ```json
/// {
///   "block-no-empty": true
/// }
/// ```
///
/// #### Secondary (optional)
///
/// - `"ignore": ["comments"]`: Exclude regular comments from being treated as
///   content inside a block. Note that configuration comments (such as
///   `/* stylelint-disable */` or `/* stx-disable */`) are always ignored
///   regardless of this setting.
///
/// ```json
/// {
///   "block-no-empty": [true, { "ignore": ["comments"] }]
/// }
/// ```
///
/// ### Examples
///
/// #### Default (`true`)
///
/// **Incorrect:**
/// ```css
/// a {}
///
/// a { }
///
/// @media print {
///   a {}
/// }
/// ```
///
/// **Correct:**
/// ```css
/// a {
///   /* foo */
/// }
///
/// @media print {
///   a {
///     color: pink;
///   }
/// }
/// ```
///
/// #### With `{ "ignore": ["comments"] }`
///
/// **Incorrect:**
/// ```css
/// a {
///   /* foo */
/// }
///
/// @media print {
///   a {
///     /* foo */
///   }
/// }
/// ```
///
/// ### References
///
/// - [Stylelint Documentation: `block-no-empty`](https://stylelint.io/user-guide/rules/block-no-empty)
#[derive(Default)]
pub struct BlockNoEmpty {
    /// When `true`, blocks containing only non-configuration comments are also
    /// reported as empty (`"ignore": ["comments"]`).
    pub ignore_comments: bool,
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
        "block-no-empty"
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
