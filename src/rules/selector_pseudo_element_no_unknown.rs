use oxc_css_parser::ast::{InterpolableIdent, QualifiedRule, SimpleSelector};
use serde_json::{Map, Value};

use super::selector_type_no_unknown::walk_selectors;
use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, PSEUDO_ELEMENTS, in_set_ignore_case, is_standard_syntax_rule,
        options_matches, parse_option_matchers, vendor_prefix,
    },
};

const NAME: &str = "selector-pseudo-element-no-unknown";

pub struct SelectorPseudoElementNoUnknown {
    ignore: Vec<OptionMatcher>,
}

impl SelectorPseudoElementNoUnknown {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore: parse_option_matchers(secondary, "ignorePseudoElements"),
        }
    }
}

impl Rule for SelectorPseudoElementNoUnknown {
    fn name(&self) -> &'static str {
        NAME
    }

    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if !ctx.slice(rule.selector.span).contains("::")
            || !is_standard_syntax_rule(ctx.source, rule)
        {
            return;
        }
        walk_selectors(ctx.source, &rule.selector, |node, position| {
            if let SimpleSelector::PseudoElement(pseudo) = node
                && let InterpolableIdent::Literal(name) = &pseudo.name
                && vendor_prefix(name.raw).is_none()
                && !in_set_ignore_case(PSEUDO_ELEMENTS, name.raw)
                && !options_matches(&self.ignore, name.raw)
            {
                ctx.report(
                    NAME,
                    position.span(pseudo.span.start, name.span.end),
                    format!("Unknown pseudo-element selector \"::{}\"", name.raw),
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::NAME;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn known_and_nested_elements() {
        Tester::new(
            NAME,
            json!(true),
            [
                "::picker(select), ::search-text, ::SCROLL-marker, a::shadow, a:before, a::-moz-test {}",
                r".foo\:\:fake, a:contains('::fake'), [title='::fake'] {}",
            ],
            [
                reject("a:not(::Mystery) {}", None, "Unknown pseudo-element selector \"::Mystery\"", (1, 7, 1, 16)),
                reject("a::slotted(.x::bad) {}", None, "Unknown pseudo-element selector \"::bad\"", (1, 14, 1, 19)),
                reject("a:custom(::bad)::before {}", None, "Unknown pseudo-element selector \"::bad\"", (1, 10, 1, 15)),
            ],
        ).test();
    }

    #[test]
    fn configured_names() {
        Tester::new(
            NAME,
            json!([true, {"ignorePseudoElements": ["pseudo", "/^my-/i"]}]),
            ["a::pseudo, a::MY-name {}"],
            [reject(
                "a::Pseudo() {}",
                None,
                "Unknown pseudo-element selector \"::Pseudo\"",
                (1, 2, 1, 10),
            )],
        )
        .test();
    }
}
