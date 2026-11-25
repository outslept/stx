use crate::{context::LintContext, rules::Rule, utils::*};
use oxc_css_parser::ast::QualifiedRule;
pub struct R {
    max: u64,
}
impl R {
    pub fn from_config(p: &serde_json::Value) -> Option<Self> {
        Some(Self { max: p.as_u64()? })
    }
}
fn count(raw: &str) -> usize {
    selector_combinators_nested(raw).len() + 1
}
impl Rule for R {
    fn name(&self) -> &'static str {
        "selector-max-compound-selectors"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        for sel in &rule.selector.selectors {
            let raw = ctx.slice(sel.span);
            let s = raw.trim();
            if has_interpolation(s) {
                continue;
            }
            let c = count(s);
            if c as u64 > self.max {
                ctx.report(
                    self.name(),
                    sel.span,
                    format!(
                        "Too many compound selectors in \"{}\", maximum {}",
                        s, self.max
                    ),
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject_message};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec!["a b {}"];
        let fail = vec![reject_message(
            "a b c {}",
            None,
            "Too many compound selectors in \"a b c\", maximum 2",
        )];
        Tester::new("selector-max-compound-selectors", json!([2]), &pass, &fail).test();
    }
    #[test]
    fn ignores_interpolation_as_compounds() {
        Tester::new(
            "selector-max-compound-selectors",
            json!([0]),
            ["a#{$selector} b {}"],
            [],
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
