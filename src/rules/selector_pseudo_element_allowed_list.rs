use oxc_css_parser::{Span, ast::ComplexSelector};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, options_matches, parse_primary_option_matchers, selector_pseudos},
};

fn reject_msg(value: &str) -> String {
    format!("Disallowed pseudo-element \"{value}\"")
}

crate::define_stx_lint! {
    /// Allows only listed pseudo-elements in selectors.
    ///
    /// Set to a name or a list of names and `/regex/` patterns, without leading
    /// colons or arguments. Checks double-colon notation only, with case-sensitive
    /// literal name matching. Does not fix violations.
    ///
    /// Example: `a::after {}` is disallowed if only `"before"` is listed.
    "selector-pseudo-element-allowed-list", restriction;
    pub struct SelectorPseudoElementAllowedList {
        allowed: Vec<OptionMatcher>,
    }
}

impl SelectorPseudoElementAllowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            allowed: parse_primary_option_matchers(primary),
        }
    }
}

impl Rule for SelectorPseudoElementAllowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(sel.span);
        for (value_start, value_end, name_start, name_end, is_element) in selector_pseudos(raw) {
            if !is_element {
                continue;
            }
            let name = &raw[name_start..name_end];
            if options_matches(&self.allowed, name) {
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
            "a::before {}",
            "::selection {}",
            "::-webkit-selection {}",
            "a:not(::selection) {}",
            "a::my-pseudo-element {}",
            "a::MY-other-pseudo-element {}",
        ];
        let fail = vec![
            reject("a::BEFORE {}", None, reject_msg("::BEFORE"), (1, 2, 1, 10)),
            reject("a::after {}", None, reject_msg("::after"), (1, 2, 1, 9)),
            reject("a::AFTER {}", None, reject_msg("::AFTER"), (1, 2, 1, 9)),
            reject("a,\nb::after {}", None, reject_msg("::after"), (2, 2, 2, 9)),
            reject(
                "a::not-my-pseudo-element {}",
                None,
                reject_msg("::not-my-pseudo-element"),
                (1, 2, 1, 25),
            ),
        ];
        Tester::new(
            "selector-pseudo-element-allowed-list",
            json!(["before", "/^(-webkit-)?selection$/", "/^my/i"]),
            &pass,
            &fail,
        )
        .test();
    }
}
