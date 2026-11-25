use super::keyframe_block_no_duplicate_selectors::is_keyframes;
use crate::{
    context::LintContext,
    rules::Rule,
    utils::{PatternMatcher, at_rule_prelude_span, ident_span, is_standard_syntax_keyframes_name},
};
use oxc_css_parser::ast::{AtRule, AtRulePrelude, InterpolableStr, KeyframesName};

#[derive(Default)]
pub struct KeyframesNamePattern {
    pub matcher: PatternMatcher,
}

impl Rule for KeyframesNamePattern {
    fn name(&self) -> &'static str {
        "keyframes-name-pattern"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !is_keyframes(at_rule) {
            return;
        }
        let span = match &at_rule.prelude {
            Some(AtRulePrelude::Keyframes(KeyframesName::Ident(name))) => ident_span(name),
            Some(AtRulePrelude::Keyframes(KeyframesName::Str(InterpolableStr::Literal(name)))) => {
                name.span
            }
            _ => {
                let Some(span) = at_rule_prelude_span(ctx.source, at_rule) else {
                    return;
                };
                span
            }
        };
        let name = ctx.slice(span);
        if is_standard_syntax_keyframes_name(name) && !self.matcher.is_match(name) {
            ctx.report(
                self.name(),
                span,
                format!(
                    "Expected \"{name}\" to match pattern \"{}\"",
                    self.matcher.raw_pattern
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    const RULE_NAME: &str = "keyframes-name-pattern";
    const PATTERN: &str = "/^foo-bar$/";

    fn expected(name: &str) -> String {
        format!("Expected \"{name}\" to match pattern \"{PATTERN}\"")
    }

    #[test]
    fn matches_the_original_name() {
        let pass = [" @KeyFrames   foo-bar   {}", "@-moz-keyframes foo-bar {}"];
        let fail = [
            reject(
                "@-webkit-keyframes FOO-bar {}",
                None,
                expected("FOO-bar"),
                (1, 20, 1, 27),
            ),
            reject(
                "@keyframes /* before */\n  \"foo-bar\" /* after */ {}",
                None,
                expected("\"foo-bar\""),
                (2, 3, 2, 12),
            ),
            reject(
                "@keyframes foo-bar extra {}",
                None,
                expected("foo-bar extra"),
                (1, 12, 1, 25),
            ),
        ];
        Tester::new(RULE_NAME, json!(PATTERN), pass, fail).test();
    }

    #[test]
    fn scss_interpolated_names() {
        let pass = ["@keyframes foo-#{$bar} {}"];
        let fail = [reject(
            "@keyframes Foo-bar {}",
            None,
            expected("Foo-bar"),
            (1, 12, 1, 19),
        )];
        Tester::new(RULE_NAME, json!(PATTERN), pass, fail)
            .with_custom_syntax("postcss-scss")
            .test();
    }
}
