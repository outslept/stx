use oxc_css_parser::{
    Span,
    ast::{AtRule, ComplexSelector},
};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, at_rule_prelude_span, options_matches, parse_primary_option_matchers,
        selector_pseudos,
    },
};

fn reject_msg(value: &str) -> String {
    format!("Disallowed pseudo-class \"{value}\"")
}

pub struct SelectorPseudoClassDisallowedList {
    disallowed: Vec<OptionMatcher>,
}

impl SelectorPseudoClassDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            disallowed: parse_primary_option_matchers(primary),
        }
    }

    fn check<'a>(&self, raw: &'a str, base: u32, ctx: &mut LintContext<'a>) {
        for (value_start, value_end, name_start, name_end, is_element) in selector_pseudos(raw) {
            if is_element {
                continue;
            }
            let name = &raw[name_start..name_end];
            if !options_matches(&self.disallowed, name) {
                continue;
            }
            ctx.report(
                self.name(),
                Span {
                    start: base + value_start as u32,
                    end: base + value_end as u32,
                },
                reject_msg(&raw[value_start..value_end]),
            );
        }
    }
}

impl Rule for SelectorPseudoClassDisallowedList {
    fn name(&self) -> &'static str {
        "selector-pseudo-class-disallowed-list"
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(sel.span);
        self.check(raw, sel.span.start, ctx);
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("page") {
            return;
        }
        if let Some(prelude) = at_rule_prelude_span(ctx.source, at_rule) {
            let raw = ctx.slice(prelude);
            self.check(raw, prelude.start, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "a {}",
            "a:hover {}",
            "a:nth-child(5) {}",
            "div:nth-LAST-child {}",
            "input:-Ms-INPUT-placeholder {}",
            ":root {}",
            "a:HOVER {}",
            "a:hover, a:nth-child(5) {}",
            "a::before {}",
            "a:nth-child(5)::before {}",
            "a:-moz-placeholder {}",
            "a:-MOZ-PLACEholder {}",
        ];
        let fail = vec![
            reject("a:focus {}", None, reject_msg(":focus"), (1, 2, 1, 8)),
            reject("a,\n:global {}", None, reject_msg(":global"), (2, 1, 2, 8)),
            reject(
                "input:-ms-input-placeholder {}",
                None,
                reject_msg(":-ms-input-placeholder"),
                (1, 6, 1, 28),
            ),
            reject(
                "a:not(::selection) {}",
                None,
                reject_msg(":not"),
                (1, 2, 1, 6),
            ),
            reject("a:has(> img) {}", None, reject_msg(":has"), (1, 2, 1, 6)),
        ];
        Tester::new(
            "selector-pseudo-class-disallowed-list",
            json!([
                "focus",
                "global",
                "/^(-ms-)?input-placeholder$/",
                "not",
                "nth-last-child",
                "has"
            ]),
            &pass,
            &fail,
        )
        .test();
    }
}
