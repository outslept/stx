use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::{ComplexSelector, ComplexSelectorChild, SimpleSelector};

pub struct SelectorMaxId {
    pub max: usize,
}

impl Rule for SelectorMaxId {
    fn name(&self) -> &'static str {
        "selector-max-id"
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        let mut count = 0;

        for child in &sel.children {
            if let ComplexSelectorChild::CompoundSelector(compound) = child {
                for simple in &compound.children {
                    if matches!(simple, SimpleSelector::Id(_)) {
                        count += 1;
                    }
                }
            }
        }

        if count > self.max {
            let selector_str = ctx.slice(sel.span).trim();
            let noun = if self.max == 1 {
                "selector"
            } else {
                "selectors"
            };
            ctx.report(
                self.name(),
                sel.span,
                format!(
                    "Expected \"{}\" to have no more than {} ID {}",
                    selector_str, self.max, noun
                ),
            );
        }
    }
}
