use crate::{
    context::LintContext,
    rules::{
        Rule,
        value_utils::{PatternMatcher, at_rule_prelude_span},
    },
};
use oxc_css_parser::{Span, ast::AtRule};

#[derive(Default)]
pub struct LayerNamePattern {
    pub matcher: PatternMatcher,
}

impl Rule for LayerNamePattern {
    fn name(&self) -> &'static str {
        "layer-name-pattern"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if !name.eq_ignore_ascii_case("layer") {
            return;
        }
        let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let raw_prelude = ctx.slice(p_span);
        let mut offset = p_span.start as usize;

        for part in raw_prelude.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                if !trimmed.split('.').all(|seg| self.matcher.is_match(seg)) {
                    let rel = part.find(trimmed).unwrap_or(0);
                    let s = offset + rel;
                    let sp = Span {
                        start: s as u32,
                        end: (s + trimmed.len()) as u32,
                    };
                    ctx.report(
                        self.name(),
                        sp,
                        format!(
                            "Expected layer name \"{}\" to match pattern \"{}\"",
                            trimmed, self.matcher.raw_pattern
                        ),
                    );
                }
            }
            offset += part.len() + 1;
        }
    }
}
