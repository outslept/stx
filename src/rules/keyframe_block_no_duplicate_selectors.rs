use std::collections::HashSet;

use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::AtRule};

pub struct KeyframeBlockNoDuplicateSelectors;

impl Rule for KeyframeBlockNoDuplicateSelectors {
    fn name(&self) -> &'static str {
        "keyframe-block-no-duplicate-selectors"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        let is_keyframes = name.eq_ignore_ascii_case("keyframes")
            || name.eq_ignore_ascii_case("-webkit-keyframes")
            || name.eq_ignore_ascii_case("-moz-keyframes")
            || name.eq_ignore_ascii_case("-o-keyframes");

        if !is_keyframes {
            return;
        }

        let Some(block) = &at_rule.block else {
            return;
        };

        let src = ctx.source;
        let start = block.span.start as usize + 1;
        let end = (block.span.end as usize).saturating_sub(1);
        if start >= end || end > src.len() {
            return;
        }

        let bytes = src.as_bytes();
        let mut i = start;
        let mut depth = 0usize;
        let mut header_start = start;
        let mut seen = HashSet::new();

        while i < end {
            if i + 1 < end && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < end && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
                continue;
            }

            if i + 1 < end && bytes[i] == b'/' && bytes[i + 1] == b'/' {
                i += 2;
                while i < end && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }

            match bytes[i] {
                b'{' => {
                    if depth == 0 {
                        check_keyframe_header(src, header_start, i, &mut seen, self.name(), ctx);
                    }
                    depth += 1;
                }
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        header_start = i + 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

fn check_keyframe_header(
    src: &str,
    start: usize,
    end: usize,
    seen: &mut HashSet<String>,
    rule_name: &'static str,
    ctx: &mut LintContext<'_>,
) {
    let slice = &src[start..end];
    let mut part_offset = 0usize;

    for part in slice.split(',') {
        let part_start = start + part_offset;
        let part_len = part.len();
        part_offset += part_len + 1;

        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        let leading_ws = part.len() - part.trim_start().len();
        let span_start = (part_start + leading_ws) as u32;
        let span_end = span_start + trimmed.len() as u32;

        let normalized = trimmed.to_ascii_lowercase();
        if !seen.insert(normalized) {
            ctx.report(
                rule_name,
                Span {
                    start: span_start,
                    end: span_end,
                },
                format!("Unexpected duplicate \"{trimmed}\""),
            );
        }
    }
}
