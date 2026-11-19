use crate::{
    context::LintContext,
    rules::{
        Rule,
        value_utils::{PatternMatcher, ValueTokenKind, at_rule_prelude_span, tokenize_value},
    },
};
use oxc_css_parser::ast::AtRule;

#[derive(Default)]
pub struct KeyframesNamePattern {
    pub matcher: PatternMatcher,
}

impl Rule for KeyframesNamePattern {
    fn name(&self) -> &'static str {
        "keyframes-name-pattern"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if !name.eq_ignore_ascii_case("keyframes")
            && !name.eq_ignore_ascii_case("-webkit-keyframes")
            && !name.eq_ignore_ascii_case("-moz-keyframes")
        {
            return;
        }
        let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let tokens = tokenize_value(ctx.source, p_span);
        if let Some(first) = tokens.first() {
            let kf_name = match first.kind {
                ValueTokenKind::Ident => first.raw,
                ValueTokenKind::String if first.raw.len() >= 2 => {
                    &first.raw[1..first.raw.len() - 1]
                }
                _ => return,
            };
            if !self.matcher.is_match(kf_name) {
                ctx.report(
                    self.name(),
                    first.span,
                    format!(
                        "Expected keyframe name \"{}\" to match pattern \"{}\"",
                        kf_name, self.matcher.raw_pattern
                    ),
                );
            }
        }
    }
}
