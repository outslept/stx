use oxc_css_parser::ast::SimpleBlock;
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;

#[derive(Default)]
pub struct BlockNoEmpty {
    pub ignore_comments: bool,
}

impl BlockNoEmpty {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        let mut ignore_comments = false;
        if let Some(sec) = secondary {
            if let Some(arr) = sec.get("ignore").and_then(|v| v.as_array()) {
                for item in arr {
                    if item.as_str() == Some("comments") {
                        ignore_comments = true;
                    }
                }
            }
        }
        Self { ignore_comments }
    }
}

fn is_configuration_comment(content: &str) -> bool {
    let trimmed = content.trim();
    let base = match trimmed.split_once("--") {
        Some((before, _)) => before.trim(),
        None => trimmed,
    };
    for prefix in [
        "stylelint-disable-next-line",
        "stylelint-disable-line",
        "stylelint-disable",
        "stylelint-enable",
        "stylelint-configure",
    ] {
        if let Some(rest) = base.strip_prefix(prefix) {
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                return true;
            }
        }
    }
    false
}

impl Rule for BlockNoEmpty {
    fn name(&self) -> &'static str {
        "block-no-empty"
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        if !block.statements.is_empty() {
            return;
        }

        let inner_start = (block.span.start + 1) as usize;
        let inner_end = block.span.end.saturating_sub(1) as usize;
        if inner_start > inner_end || inner_end > ctx.source.len() {
            return;
        }

        let mut cursor = inner_start;
        for comment in ctx.comments {
            let c_start = comment.span.start as usize;
            let c_end = comment.span.end as usize;
            if c_end <= inner_start {
                continue;
            }
            if c_start >= inner_end {
                break;
            }

            if c_start > cursor {
                let gap = &ctx.source[cursor..c_start];
                if gap.chars().any(|ch| !ch.is_whitespace()) {
                    return;
                }
            }

            if !is_configuration_comment(comment.content) && !self.ignore_comments {
                return;
            }

            cursor = c_end;
        }

        if cursor < inner_end {
            let tail = &ctx.source[cursor..inner_end];
            if tail.chars().any(|ch| !ch.is_whitespace()) {
                return;
            }
        }

        ctx.report(self.name(), block.span, "Empty block".to_string());
    }
}
