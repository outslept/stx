use oxc_css_parser::ast::QualifiedRule;

use super::selector_no_qualifying_type::resolve_nested_selectors;
use crate::{context::LintContext, rules::Rule};
pub struct SelectorNoUnmatchable;
impl Rule for SelectorNoUnmatchable {
    fn name(&self) -> &'static str {
        "selector-no-unmatchable"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        let nested = ctx.ancestors.iter().any(|parent| {
            matches!(parent, crate::context::AstParent::QualifiedRule(_))
                || matches!(parent, crate::context::AstParent::AtRule(at_rule) if at_rule.name.name.eq_ignore_ascii_case("scope"))
        });
        for (sel, resolved) in rule
            .selector
            .selectors
            .iter()
            .zip(resolve_nested_selectors(rule, ctx))
        {
            let raw = ctx.slice(sel.span);
            for (fn_name, reason) in [
                ("is", "pseudo-elements cannot be represented by \":is()\""),
                (
                    "where",
                    "pseudo-elements cannot be represented by \":where()\"",
                ),
            ] {
                let needle = format!(":{fn_name}(");
                let mut search = raw;
                while let Some(pos) = search.find(&needle) {
                    let after = &search[pos + needle.len()..];
                    let Some(close) = after.find(')') else {
                        break;
                    };
                    if after[..close].contains("::") {
                        let selector = if nested {
                            format!("\"{}\" (\"{}\")", raw.trim(), resolved.trim())
                        } else {
                            format!("\"{}\"", raw.trim())
                        };
                        ctx.report(
                            self.name(),
                            sel.span,
                            format!("Unmatchable selector {selector}, {reason}"),
                        );
                        return;
                    }
                    search = &after[close..];
                }
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
        let pass = vec!["a {}", ":is(.foo) {}", ":not(::before) {}"];
        let fail = vec![
            reject_message(
                ":is(::before) {}",
                None,
                "Unmatchable selector \":is(::before)\", pseudo-elements cannot be represented by \":is()\"",
            ),
            reject_message(
                ":where(input::placeholder) {}",
                None,
                "Unmatchable selector \":where(input::placeholder)\", pseudo-elements cannot be represented by \":where()\"",
            ),
            reject_message(
                "article { & :is(::before) {} }",
                None,
                "Unmatchable selector \"& :is(::before)\" (\"article :is(::before)\"), pseudo-elements cannot be represented by \":is()\"",
            ),
        ];
        Tester::new("selector-no-unmatchable", json!(true), &pass, &fail).test();
    }
}
