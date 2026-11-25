use super::selector_type_no_unknown::walk_selectors;
use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        DEPRECATED_HTML_TYPE_SELECTORS, DEPRECATED_PSEUDO_CLASSES, DEPRECATED_PSEUDO_ELEMENTS,
        DEPRECATED_SVG_TYPE_SELECTORS, OptionMatcher, in_set_ignore_case, options_matches,
        parse_option_matchers, selector_pseudos_nested,
    },
};
use oxc_css_parser::ast::{
    InterpolableIdent, QualifiedRule, SimpleSelector, TypeSelector, UnknownQualifiedRule,
};
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
        self.check_selector(rule.selector.span, ctx);
        walk_selectors(ctx.source, &rule.selector, |node, at| {
            let SimpleSelector::Type(TypeSelector::TagName(tag)) = node else {
                return;
            };
            let InterpolableIdent::Literal(ident) = &tag.name.name else {
                return;
            };
            let name = ident.name;
            if (in_set_ignore_case(DEPRECATED_HTML_TYPE_SELECTORS, name)
                || DEPRECATED_SVG_TYPE_SELECTORS.contains(&name))
                && !options_matches(&self.ignore, name)
            {
                ctx.report(
                    Rule::name(self),
                    at.span(ident.span.start, ident.span.end),
                    format!("Deprecated selector \"{}\"", ident.raw),
                );
            }
        });
    }

    fn check_unknown_qualified_rule<'a>(
        &self,
        rule: &'a UnknownQualifiedRule<'a>,
        ctx: &mut LintContext<'a>,
    ) {
        self.check_selector(rule.prelude.span, ctx);
    }
}

impl SelectorNoDeprecated {
    fn check_selector(&self, selector: oxc_css_parser::Span, ctx: &mut LintContext<'_>) {
        let raw = ctx.slice(selector);
        for (vs, ve, ns, ne, is_el) in selector_pseudos_nested(raw) {
            let value = &raw[vs..ve];
            let name = &raw[ns..ne];
            let lower = name.to_ascii_lowercase();
            if options_matches(&self.ignore, name) {
                continue;
            }
            let span = oxc_css_parser::Span {
                start: selector.start + vs as u32,
                end: selector.start + ve as u32,
            };
            if is_el {
                if !DEPRECATED_PSEUDO_ELEMENTS.contains(&lower.as_str()) {
                    continue;
                }
                if let Some(remap) = element_remap(&lower) {
                    let mapped = format!("::{remap}");
                    ctx.report_with_fix(
                        Rule::name(self),
                        span,
                        format!("Expected \"{value}\" to be \"{mapped}\""),
                        Fix::single(span, mapped),
                    );
                } else {
                    ctx.report(
                        Rule::name(self),
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
                        Rule::name(self),
                        span,
                        format!("Expected \"{value}\" to be \"{mapped}\""),
                        Fix::single(span, mapped),
                    );
                } else {
                    ctx.report(
                        Rule::name(self),
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
    use crate::tester::{Tester, reject};
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

    #[test]
    fn deprecated_type_selectors() {
        let pass = [
            "svg hatchpath {}",
            "::part(acronym) {}",
            ".popup, [popup] {}",
            "svg hkern {}",
        ];
        let fail = [
            reject(
                "abbr, POPUP {}",
                None,
                "Deprecated selector \"POPUP\"",
                (1, 7, 1, 12),
            ),
            reject(
                "svg hatchPath {}",
                None,
                "Deprecated selector \"hatchPath\"",
                (1, 5, 1, 14),
            ),
            reject(
                "a:is(acronym) {}",
                None,
                "Deprecated selector \"acronym\"",
                (1, 6, 1, 13),
            ),
        ];
        Tester::new(
            "selector-no-deprecated",
            json!([true, {"ignoreSelectors": ["/kern$/"]}]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn checks_rules_with_unparsed_selectors() {
        let fail = [reject(
            ":is(), :matches() {}",
            ":is(), :is() {}",
            "Expected \":matches\" to be \":is\"",
            (1, 8, 1, 16),
        )];
        Tester::new("selector-no-deprecated", json!(true), [], fail)
            .with_fix(true)
            .test();
    }
}
