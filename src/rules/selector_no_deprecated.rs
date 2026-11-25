use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        DEPRECATED_PSEUDO_CLASSES, DEPRECATED_PSEUDO_ELEMENTS, OptionMatcher, options_matches,
        parse_option_matchers, selector_pseudos_nested,
    },
};
use oxc_css_parser::ast::QualifiedRule;
fn class_remap(name: &str) -> Option<&'static str> {
    match name {
        "focus-ring" => Some("focus-visible"),
        "matches" | "-webkit-any" | "-moz-any" => Some("is"),
        "top-layer" | "popup-open" => Some("open"),
        "user-error" => Some("user-invalid"),
        _ => None,
    }
}
fn element_remap(name: &str) -> Option<&'static str> {
    match name {
        "content" => Some("slotted"),
        _ => None,
    }
}
pub struct SelectorNoDeprecated {
    ignore: Vec<OptionMatcher>,
}
impl SelectorNoDeprecated {
    pub fn new(s: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        Self {
            ignore: parse_option_matchers(s, "ignoreSelectors"),
        }
    }
}
impl Rule for SelectorNoDeprecated {
    fn name(&self) -> &'static str {
        "selector-no-deprecated"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(rule.selector.span);
        for (vs, ve, ns, ne, is_el) in selector_pseudos_nested(raw) {
            let value = &raw[vs..ve];
            let name = &raw[ns..ne];
            let lower = name.to_ascii_lowercase();
            if options_matches(&self.ignore, name) {
                continue;
            }
            let span = oxc_css_parser::Span {
                start: rule.selector.span.start + vs as u32,
                end: rule.selector.span.start + ve as u32,
            };
            if is_el {
                if !DEPRECATED_PSEUDO_ELEMENTS.contains(&lower.as_str()) {
                    continue;
                }
                if let Some(remap) = element_remap(&lower) {
                    let mapped = format!("::{remap}");
                    ctx.report_with_fix(
                        self.name(),
                        span,
                        format!("Expected \"{value}\" to be \"{mapped}\""),
                        Fix::single(span, mapped),
                    );
                } else {
                    ctx.report(
                        self.name(),
                        span,
                        format!("Deprecated selector \"{value}\""),
                    );
                }
            } else {
                if !DEPRECATED_PSEUDO_CLASSES.contains(&lower.as_str()) {
                    continue;
                }
                if let Some(remap) = class_remap(&lower) {
                    let mapped = format!(":{remap}");
                    ctx.report_with_fix(
                        self.name(),
                        span,
                        format!("Expected \"{value}\" to be \"{mapped}\""),
                        Fix::single(span, mapped),
                    );
                } else {
                    ctx.report(
                        self.name(),
                        span,
                        format!("Deprecated selector \"{value}\""),
                    );
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec![
            "abbr {}",
            "circle {}",
            "a:hover {}",
            "a::first-line {}",
            "::backdrop {}",
        ];
        let fail = vec![
            reject(
                "a:drop {}",
                None,
                "Deprecated selector \":drop\"",
                (1, 2, 1, 7),
            ),
            reject(
                "a:focus-ring {}",
                "a:focus-visible {}",
                "Expected \":focus-ring\" to be \":focus-visible\"",
                (1, 2, 1, 13),
            ),
            reject(
                "a:DROP {}",
                None,
                "Deprecated selector \":DROP\"",
                (1, 2, 1, 7),
            ),
            reject(
                "a::shadow {}",
                None,
                "Deprecated selector \"::shadow\"",
                (1, 2, 1, 10),
            ),
            reject(
                "a::content {}",
                "a::slotted {}",
                "Expected \"::content\" to be \"::slotted\"",
                (1, 2, 1, 11),
            ),
        ];
        Tester::new("selector-no-deprecated", json!(true), &pass, &fail)
            .with_fix(true)
            .test();
    }
}
