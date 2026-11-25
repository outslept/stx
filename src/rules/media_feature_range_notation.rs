use crate::{context::LintContext, fix::Fix, rules::Rule, utils::at_rule_prelude_span};
use oxc_css_parser::{Span, ast::AtRule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaRangeMode {
    Context,
    Prefix,
}

pub struct MediaFeatureRangeNotation {
    pub mode: MediaRangeMode,
}

impl Default for MediaFeatureRangeNotation {
    fn default() -> Self {
        Self {
            mode: MediaRangeMode::Context,
        }
    }
}

const RANGE_FEATURES: &[&str] = &[
    "width",
    "height",
    "device-width",
    "device-height",
    "aspect-ratio",
    "device-aspect-ratio",
    "color",
    "color-index",
    "monochrome",
    "resolution",
    "inline-size",
    "block-size",
];

impl MediaFeatureRangeNotation {
    fn check_paren_group(&self, open_idx: usize, close_idx: usize, ctx: &mut LintContext<'_>) {
        let inner = &ctx.source[open_idx + 1..close_idx];
        if inner.contains('(') || inner.contains(')') {
            return;
        }
        let full_span = Span {
            start: open_idx as u32,
            end: (close_idx + 1) as u32,
        };

        match self.mode {
            MediaRangeMode::Context => {
                let Some(colon_pos) = inner.find(':') else {
                    return;
                };
                let feat = inner[..colon_pos].trim();
                let val = inner[colon_pos + 1..].trim();
                if val.is_empty() {
                    return;
                }
                let lower = feat.to_ascii_lowercase();
                let (op, base) = if let Some(b) = lower.strip_prefix("min-") {
                    (">=", b)
                } else if let Some(b) = lower.strip_prefix("max-") {
                    ("<=", b)
                } else {
                    return;
                };

                if RANGE_FEATURES.contains(&base) {
                    let orig_base = &feat[4..];
                    let expected = format!("({} {} {})", orig_base, op, val);
                    ctx.report_with_fix(
                        self.name(),
                        full_span,
                        "Expected \"context\" media feature range notation".to_string(),
                        Some(Fix::single(full_span, expected)),
                    );
                }
            }
            MediaRangeMode::Prefix => {
                let (op, prefix) = if inner.contains(">=") {
                    (">=", "min-")
                } else if inner.contains("<=") {
                    ("<=", "max-")
                } else {
                    return;
                };
                let parts: Vec<&str> = inner.split(op).collect();
                if parts.len() != 2 {
                    return;
                }
                let feat = parts[0].trim();
                let val = parts[1].trim();
                if RANGE_FEATURES.iter().any(|f| f.eq_ignore_ascii_case(feat)) {
                    let expected = format!("({}{}: {})", prefix, feat, val);
                    ctx.report_with_fix(
                        self.name(),
                        full_span,
                        "Expected \"prefix\" media feature range notation".to_string(),
                        Some(Fix::single(full_span, expected)),
                    );
                }
            }
        }
    }
}

impl Rule for MediaFeatureRangeNotation {
    fn name(&self) -> &'static str {
        "media-feature-range-notation"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if !name.eq_ignore_ascii_case("media") && !name.eq_ignore_ascii_case("container") {
            return;
        }
        let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };

        let start = p_span.start as usize;
        let end = (p_span.end as usize).min(ctx.source.len());
        let bytes = ctx.source.as_bytes();
        let mut stack = Vec::new();
        let mut i = start;

        while i < end {
            if bytes[i] == b'(' {
                stack.push(i);
            } else if bytes[i] == b')' {
                if let Some(open_idx) = stack.pop() {
                    self.check_paren_group(open_idx, i, ctx);
                }
            }
            i += 1;
        }
    }
}
