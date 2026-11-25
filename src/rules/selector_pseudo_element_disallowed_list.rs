use oxc_css_parser::{Span, ast::ComplexSelector};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, options_matches, parse_primary_option_matchers, selector_pseudos},
};

fn reject_msg(value: &str) -> String {
    format!("Disallowed pseudo-element \"{value}\"")
}

pub struct SelectorPseudoElementDisallowedList {
    disallowed: Vec<OptionMatcher>,
}

impl SelectorPseudoElementDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            disallowed: parse_primary_option_matchers(primary),
        }
    }
}

impl Rule for SelectorPseudoElementDisallowedList {
    fn name(&self) -> &'static str {
        "selector-pseudo-element-disallowed-list"
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(sel.span);
        for (value_start, value_end, name_start, name_end, is_element) in selector_pseudos(raw) {
            if !is_element {
                continue;
            }
            let name = &raw[name_start..name_end];
            if !options_matches(&self.disallowed, name) {
                continue;
            }
            ctx.report(
                self.name(),
                Span {
                    start: sel.span.start + value_start as u32,
                    end: sel.span.start + value_end as u32,
                },
                reject_msg(&raw[value_start..value_end]),
            );
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
            "a::BEFORE {}",
            "a::after {}",
            "::first-line {}",
            "::-webkit-first-line {}",
            "a:not(::first-line) {}",
            "a::their-pseudo-element {}",
            "a::THEIR-other-pseudo-element {}",
        ];
        let fail = vec![
            reject("a::before {}", None, reject_msg("::before"), (1, 2, 1, 10)),
            reject(
                "a,\nb::before {}",
                None,
                reject_msg("::before"),
                (2, 2, 2, 10),
            ),
            reject(
                "::selection {}",
                None,
                reject_msg("::selection"),
                (1, 1, 1, 12),
            ),
            reject(
                "::-webkit-selection {}",
                None,
                reject_msg("::-webkit-selection"),
                (1, 1, 1, 20),
            ),
            reject(
                "a:not(::selection) {}",
                None,
                reject_msg("::selection"),
                (1, 7, 1, 18),
            ),
            reject(
                "a::my-pseudo-element {}",
                None,
                reject_msg("::my-pseudo-element"),
                (1, 2, 1, 21),
            ),
            reject(
                "a::MY-OTHER-pseudo-element {}",
                None,
                reject_msg("::MY-OTHER-pseudo-element"),
                (1, 2, 1, 27),
            ),
        ];
        Tester::new(
            "selector-pseudo-element-disallowed-list",
            json!(["before", "/^(-webkit-)?selection$/", "/^my/i"]),
            &pass,
            &fail,
        )
        .test();
    }
}
