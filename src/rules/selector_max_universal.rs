use super::selector_max_compound_selectors::{SelectorNode, walk_selector};
use crate::{context::LintContext, rules::Rule, utils::*};
use oxc_css_parser::ast::{ComplexSelector, QualifiedRule, SimpleSelector, TypeSelector};
use serde_json::{Map, Value};

pub struct R {
    max: u64,
    ignore_after: Vec<OptionMatcher>,
}
impl R {
    pub fn from_config(p: &Value, secondary: Option<&Map<String, Value>>) -> Option<Self> {
        Some(Self {
            max: p.as_u64()?,
            ignore_after: parse_option_matchers(secondary, "ignoreAfterCombinators"),
        })
    }

    fn count(&self, selector: &ComplexSelector<'_>, ctx: &LintContext<'_>) -> usize {
        if self.ignore_after.is_empty() {
            return count_selector_universals(ctx.slice(selector.span));
        }
        let mut count = 0;
        let mut after_ignored = false;
        walk_selector(selector, &mut |node| {
            if matches!(
                node,
                SelectorNode::Simple(SimpleSelector::Type(TypeSelector::Universal(_)))
            ) && !after_ignored
            {
                count += 1;
            }
            after_ignored = matches!(node, SelectorNode::Combinator(_))
                && options_matches(&self.ignore_after, node.value(ctx));
        });
        count
    }
}
impl Rule for R {
    fn name(&self) -> &'static str {
        "selector-max-universal"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        for sel in &rule.selector.selectors {
            let raw = ctx.slice(sel.span);
            let s = raw.trim();
            let c = self.count(sel, ctx);
            if c as u64 > self.max {
                ctx.report(
                    self.name(),
                    sel.span,
                    format!(
                        "Too many universal selectors in \"{}\", maximum {}",
                        s, self.max
                    ),
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_message};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec!["a {}"];
        let fail = vec![reject_message(
            "* {}",
            None,
            "Too many universal selectors in \"*\", maximum 0",
        )];
        Tester::new("selector-max-universal", json!([0]), &pass, &fail).test();
    }

    #[test]
    fn ignores_only_universals_immediately_after_configured_combinators() {
        Tester::new(
            "selector-max-universal",
            json!([0, {"ignoreAfterCombinators": [" ", "~", "+", ">"]}]),
            [
                ".foo\n  * {}",
                ".foo ~ * {}",
                ".foo + * {}",
                ".foo > * {}",
                ":has(> *) {}",
            ],
            [reject_message(
                "* {}",
                None,
                "Too many universal selectors in \"*\", maximum 0",
            )],
        )
        .test();
        Tester::new(
            "selector-max-universal",
            json!([1, {"ignoreAfterCombinators": ">"}]),
            ["*.foo > * {}"],
            [
                reject_message(
                    "*.foo * {}",
                    None,
                    "Too many universal selectors in \"*.foo *\", maximum 1",
                ),
                reject_message(
                    "* > :is(.foo > *, *) {}",
                    None,
                    "Too many universal selectors in \"* > :is(.foo > *, *)\", maximum 1",
                ),
            ],
        )
        .test();
    }
}
