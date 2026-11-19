use crate::{
    context::LintContext,
    rules::{
        Rule,
        value_utils::{PatternMatcher, ValueTokenKind, at_rule_prelude_span, tokenize_value},
    },
};
use oxc_css_parser::ast::AtRule;

#[derive(Default)]
pub struct CustomMediaPattern {
    pub matcher: PatternMatcher,
}

impl Rule for CustomMediaPattern {
    fn name(&self) -> &'static str {
        "custom-media-pattern"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if !name.eq_ignore_ascii_case("custom-media") {
            return;
        }
        let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let tokens = tokenize_value(ctx.source, p_span);
        if let Some(first) = tokens.first() {
            if first.kind == ValueTokenKind::Ident {
                if let Some(stripped) = first.raw.strip_prefix("--") {
                    if !self.matcher.is_match(stripped) {
                        ctx.report(
                            self.name(),
                            first.span,
                            format!(
                                "Expected custom media query name \"{}\" to match pattern \"{}\"",
                                first.raw, self.matcher.raw_pattern
                            ),
                        );
                    }
                }
            }
        }
    }
}
