use crate::{context::LintContext, rules::Rule, utils::*};
use oxc_css_parser::ast::QualifiedRule;

crate::define_stx_lint! {
    /// Limits the number of combinators in each selector.
    ///
    /// Set to a non-negative integer. Descendant whitespace counts as a combinator.
    /// Selectors with interpolation are skipped. Does not fix violations.
    ///
    /// Example: `a b {}` exceeds a maximum of `0`.
    "selector-max-combinators", restriction;
    pub struct R {
        max: u64,
    }
}

impl R {
    pub fn from_config(p: &serde_json::Value) -> Option<Self> {
        Some(Self { max: p.as_u64()? })
    }
}
fn count(raw: &str) -> usize {
    selector_combinators_nested(raw).len()
}
impl Rule for R {
    fn name(&self) -> &'static str {
        Self::NAME
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
                    format!("Too many combinators in \"{}\", maximum {}", s, self.max),
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
            "a b {}",
            None,
            "Too many combinators in \"a b\", maximum 0",
        )];
        Tester::new("selector-max-combinators", json!([0]), &pass, &fail).test();
    }
    #[test]
    fn ignores_interpolation_in_functional_pseudo_classes() {
        Tester::new(
            "selector-max-combinators",
            json!([0]),
            ["a:nth-child(#{ $index > b }) {}"],
            [],
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
