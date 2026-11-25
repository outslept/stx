use crate::{context::LintContext, rules::Rule, utils::*};
use oxc_css_parser::ast::QualifiedRule;

crate::define_stx_lint! {
    /// Limits the number of pseudo-classes in each selector.
    ///
    /// Set to a non-negative integer. Legacy single-colon pseudo-elements such as
    /// `:before` do not count. Does not fix violations.
    ///
    /// Example: `a:hover {}` exceeds a maximum of `0`.
    "selector-max-pseudo-class", restriction;
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
    match "none" {
        "count_selector_classes" => count_selector_classes(raw),
        "count_selector_universals" => count_selector_universals(raw),
        "count_selector_types" => count_selector_types(raw),
        _ => {
            if "pseudo-classes" == "combinators" {
                selector_combinators_nested(raw).len()
            } else if "pseudo-classes" == "compound selectors" {
                selector_combinators_nested(raw).len() + 1
            } else {
                selector_pseudos_nested(raw)
                    .iter()
                    .filter(|(_, _, ns, ne, e)| {
                        !*e && !matches!(
                            raw[*ns..*ne].to_ascii_lowercase().as_str(),
                            "before" | "after" | "first-line" | "first-letter"
                        )
                    })
                    .count()
            }
        }
    }
}
impl Rule for R {
    fn name(&self) -> &'static str {
        Self::NAME
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        for sel in &rule.selector.selectors {
            let raw = ctx.slice(sel.span);
            let s = raw.trim();
            let c = count(s);
            if c as u64 > self.max {
                ctx.report(
                    self.name(),
                    sel.span,
                    format!("Too many pseudo-classes in \"{}\", maximum {}", s, self.max),
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
        let pass = vec!["a {}", "a::before {}"];
        let fail = vec![reject_message(
            "a:hover {}",
            None,
            "Too many pseudo-classes in \"a:hover\", maximum 0",
        )];
        Tester::new("selector-max-pseudo-class", json!([0]), &pass, &fail).test();
    }
}
