use crate::{context::LintContext, fix::Fix, rules::Rule};
use oxc_css_parser::{Span, ast::AtRule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyframeSelectorMode {
    PercentageUnlessWithinKeywordOnlyBlock,
    Percentage,
    Keyword,
}

pub struct KeyframeSelectorNotation {
    pub mode: KeyframeSelectorMode,
}

impl Default for KeyframeSelectorNotation {
    fn default() -> Self {
        Self {
            mode: KeyframeSelectorMode::PercentageUnlessWithinKeywordOnlyBlock,
        }
    }
}

struct KeyframeSelItem<'a> {
    raw: &'a str,
    span: Span,
}

fn collect_keyframe_selectors<'a>(source: &'a str, block_span: Span) -> Vec<KeyframeSelItem<'a>> {
    let start = (block_span.start as usize + 1).min(source.len());
    let end = (block_span.end as usize)
        .saturating_sub(1)
        .min(source.len());
    if start >= end {
        return Vec::new();
    }

    let bytes = source.as_bytes();
    let mut i = start;
    let mut head_start = start;
    let mut depth = 0usize;
    let mut items = Vec::new();

    while i < end {
        let b = bytes[i];
        if b == b'/' && i + 1 < end && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < end && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(end);
            continue;
        }
        if b == b'{' {
            if depth == 0 {
                let header = &source[head_start..i];
                let mut offset = head_start;
                for part in header.split(',') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        if let Some(rel) = part.find(trimmed) {
                            let s = offset + rel;
                            let e = s + trimmed.len();
                            items.push(KeyframeSelItem {
                                raw: &source[s..e],
                                span: Span {
                                    start: s as u32,
                                    end: e as u32,
                                },
                            });
                        }
                    }
                    offset += part.len() + 1;
                }
            }
            depth += 1;
            i += 1;
            continue;
        }
        if b == b'}' {
            depth = depth.saturating_sub(1);
            i += 1;
            if depth == 0 {
                head_start = i;
            }
            continue;
        }
        i += 1;
    }

    items
}

impl Rule for KeyframeSelectorNotation {
    fn name(&self) -> &'static str {
        "keyframe-selector-notation"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if !name.eq_ignore_ascii_case("keyframes")
            && !name.eq_ignore_ascii_case("-webkit-keyframes")
            && !name.eq_ignore_ascii_case("-moz-keyframes")
        {
            return;
        }
        let Some(block) = &at_rule.block else {
            return;
        };

        let items = collect_keyframe_selectors(ctx.source, block.span);
        if items.is_empty() {
            return;
        }

        let all_keywords = items
            .iter()
            .all(|it| it.raw.eq_ignore_ascii_case("from") || it.raw.eq_ignore_ascii_case("to"));

        if self.mode == KeyframeSelectorMode::PercentageUnlessWithinKeywordOnlyBlock && all_keywords
        {
            return;
        }

        for item in items {
            let lower = item.raw.to_ascii_lowercase();
            match self.mode {
                KeyframeSelectorMode::Percentage
                | KeyframeSelectorMode::PercentageUnlessWithinKeywordOnlyBlock => {
                    let expected = match lower.as_str() {
                        "from" => Some("0%"),
                        "to" => Some("100%"),
                        _ => None,
                    };
                    if let Some(exp) = expected {
                        ctx.report_with_fix(
                            self.name(),
                            item.span,
                            format!("Expected \"{}\" to be \"{}\"", item.raw, exp),
                            Some(Fix::single(item.span, exp.to_string())),
                        );
                    }
                }
                KeyframeSelectorMode::Keyword => {
                    let expected = match lower.as_str() {
                        "0%" => Some("from"),
                        "100%" => Some("to"),
                        _ => None,
                    };
                    if let Some(exp) = expected {
                        ctx.report_with_fix(
                            self.name(),
                            item.span,
                            format!("Expected \"{}\" to be \"{}\"", item.raw, exp),
                            Some(Fix::single(item.span, exp.to_string())),
                        );
                    }
                }
            }
        }
    }
}
