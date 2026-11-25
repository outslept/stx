use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{RANGE_TYPE_MEDIA_FEATURE_NAMES, at_rule_prelude_span},
};
use oxc_css_parser::{Span, ast::AtRule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaRangeMode {
    Context,
    Prefix,
}

crate::define_stx_lint! {
    /// Require consistent range notation for recognized features in `@media`.
    ///
    /// Set the primary option to `"context"` for comparison operators or
    /// `"prefix"` for colon notation with `min-` and `max-` bounds. Include
    /// `"exact-value"` in the secondary `except` option (a string or array) to
    /// reverse the choice for exact matches such as `(width: 50em)` and
    /// `(width = 50em)`. Without that exception, an exact-only query may be
    /// skipped by the initial range check.
    /// Edits are only emitted when converting colon notation to comparisons.
    ///
    /// With `"context"`, `@media (min-width: 50em) {}` is fixed to
    /// `@media (width >= 50em) {}`.
    "media-feature-range-notation", style, fix = conditional;
    pub struct MediaFeatureRangeNotation {
        pub mode: MediaRangeMode,
        pub except_exact_value: bool,
    }
}

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
        let range_op = inner.find(['<', '>', '=']);
        let (name, plain) = match (range_op, inner.find(':')) {
            (None, Some(colon)) => (inner[..colon].trim(), true),
            (Some(_), None) => {
                // The feature name is the identifier side of the range.
                let Some(name) = inner
                    .split(['<', '>', '='])
                    .map(str::trim)
                    .find(|part| part.starts_with(|c: char| c.is_ascii_alphabetic()))
                else {
                    return;
                };
                (name, false)
            }
            _ => return,
        };
        let lower = name.to_ascii_lowercase();
        let unprefixed = lower
            .strip_prefix("min-")
            .or_else(|| lower.strip_prefix("max-"))
            .unwrap_or(&lower);
        if !RANGE_TYPE_MEDIA_FEATURE_NAMES.contains(&unprefixed) {
            return;
        }
        let mut expected = self.mode;
        if self.except_exact_value {
            let exact_operator = !plain && {
                let ops: String = inner.chars().filter(|c| "<>=".contains(*c)).collect();
                ops == "="
            };
            let plain_unprefixed = plain && lower.len() == unprefixed.len();
            if exact_operator || plain_unprefixed {
                expected = match self.mode {
                    MediaRangeMode::Prefix => MediaRangeMode::Context,
                    MediaRangeMode::Context => MediaRangeMode::Prefix,
                };
            }
        }
        let fix = match (expected, plain) {
            (MediaRangeMode::Prefix, true) | (MediaRangeMode::Context, false) => return,
            (MediaRangeMode::Context, true) => {
                let colon = inner.find(':').unwrap_or(0);
                let op = match &lower[..lower.len() - unprefixed.len()] {
                    "min-" => ">=",
                    "max-" => "<=",
                    _ => "=",
                };
                let name_part = &inner[..colon];
                let before = &name_part[..name_part.len() - name_part.trim_start().len()];
                let after = match &name_part[name_part.trim_end().len()..] {
                    "" => " ",
                    ws => ws,
                };
                let base = &name.trim()[name.len() - unprefixed.len()..];
                let rest = &inner[colon + 1..];
                Some(Fix::single(
                    full_span,
                    format!("({before}{base}{after}{op}{rest})"),
                ))
            }
            (MediaRangeMode::Prefix, false) => None,
        };
        let expected = match expected {
            MediaRangeMode::Prefix => "prefix",
            MediaRangeMode::Context => "context",
        };
        ctx.report_with_fix(
            self.name(),
            full_span,
            format!("Expected \"{expected}\" media feature range notation"),
            fix,
        );
    }
}

impl Rule for MediaFeatureRangeNotation {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("media") {
            return;
        }
        let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let params = ctx.slice(p_span);
        // Upstream prefilter, skipped with `except: ["exact-value"]`.
        if !self.except_exact_value {
            let lower = params.to_ascii_lowercase();
            let relevant = match self.mode {
                MediaRangeMode::Prefix => params.contains(['<', '>']),
                MediaRangeMode::Context => lower.contains("min-") || lower.contains("max-"),
            };
            if !relevant {
                return;
            }
        }

        let start = p_span.start as usize;
        let end = (p_span.end as usize).min(ctx.source.len());
        let mut stack = Vec::new();
        for (i, &b) in ctx.source.as_bytes()[..end].iter().enumerate().skip(start) {
            if b == b'(' {
                stack.push(i);
            } else if b == b')'
                && let Some(open_idx) = stack.pop()
            {
                self.check_paren_group(open_idx, i, ctx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_message, reject_messages};
    use serde_json::json;

    #[test]
    fn context_fix_keeps_whitespace() {
        let msg = "Expected \"context\" media feature range notation";
        let fail = [
            reject_message(
                "@media not print, ( min-width  : 1px ) {}",
                "@media not print, ( width  >= 1px ) {}",
                msg,
            ),
            reject_message("@media (max-width:1px) {}", "@media (width <=1px) {}", msg),
        ];
        Tester::new("media-feature-range-notation", json!("context"), [], fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn exact_values() {
        let msg = || "Expected \"context\" media feature range notation".to_string();
        let fail = [reject_messages(
            "@media (min-width: 1px)\n  and (width: 3px) {}",
            "@media (width >= 1px)\n  and (width = 3px) {}",
            vec![msg(), msg()],
        )];
        Tester::new(
            "media-feature-range-notation",
            json!("context"),
            ["@media (width: 3px) {}"],
            fail,
        )
        .with_fix(true)
        .test();
        let fail = [reject_message(
            "@media (width: 1px) {}",
            "@media (width = 1px) {}",
            "Expected \"context\" media feature range notation",
        )];
        Tester::new(
            "media-feature-range-notation",
            json!(["prefix", {"except": ["exact-value"]}]),
            ["@media (min-width: 1px) {}"],
            fail,
        )
        .with_fix(true)
        .test();
    }
}
