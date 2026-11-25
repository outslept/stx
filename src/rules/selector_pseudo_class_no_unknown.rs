use oxc_css_parser::{
    Span,
    ast::{AtRule, AtRulePrelude, InterpolableIdent, QualifiedRule, SimpleSelector},
};
use serde_json::{Map, Value};

use super::selector_type_no_unknown::{SelectorPosition, walk_selector_fragment, walk_selectors};
use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        AT_RULE_PAGE_PSEUDO_CLASSES, LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, OptionMatcher,
        PSEUDO_CLASSES, PSEUDO_ELEMENTS, WEBKIT_SCROLLBAR_PSEUDO_CLASSES,
        WEBKIT_SCROLLBAR_PSEUDO_ELEMENTS, in_set_ignore_case, is_standard_syntax_rule,
        options_matches, parse_option_matchers, vendor_prefix,
    },
};

const NAME: &str = SelectorPseudoClassNoUnknown::NAME;

fn has_scrollbar_ancestor(ctx: &LintContext<'_>) -> bool {
    ctx.ancestors.iter().rev().any(|ancestor| {
        let AstParent::QualifiedRule(rule) = ancestor else {
            return false;
        };
        if !ctx.slice(rule.selector.span).contains("::") {
            return false;
        }
        let mut found = false;
        walk_selectors(ctx.source, &rule.selector, |node, _| {
            if let SimpleSelector::PseudoElement(pseudo) = node
                && let InterpolableIdent::Literal(name) = &pseudo.name
                && in_set_ignore_case(WEBKIT_SCROLLBAR_PSEUDO_ELEMENTS, name.raw)
            {
                found = true;
            }
        });
        found
    })
}

crate::define_stx_lint! {
    /// Reports unknown pseudo-classes in selectors and `@page` preludes.
    ///
    /// Set to `true`. `ignorePseudoClasses` accepts names without colons or `/regex/` patterns.
    /// Vendor-prefixed names are exempt in style selectors, but not in `@page`.
    /// Does not fix violations.
    ///
    /// Example: `a:unknown {}` is reported unless `"unknown"` is ignored.
    "selector-pseudo-class-no-unknown", correctness, recommended = true;
    pub struct SelectorPseudoClassNoUnknown {
        ignore: Vec<OptionMatcher>,
    }
}

impl SelectorPseudoClassNoUnknown {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore: parse_option_matchers(secondary, "ignorePseudoClasses"),
        }
    }

    fn check_name(
        &self,
        name: &str,
        span: Span,
        position: Option<SelectorPosition<'_>>,
        ctx: &mut LintContext<'_>,
    ) {
        if name.starts_with("--")
            || name.starts_with("extend")
            || in_set_ignore_case(LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, name)
            || options_matches(&self.ignore, name)
        {
            return;
        }

        let known = if let Some(position) = position {
            vendor_prefix(name).is_some()
                || (!in_set_ignore_case(PSEUDO_ELEMENTS, name)
                    && (in_set_ignore_case(PSEUDO_CLASSES, name)
                        || (in_set_ignore_case(WEBKIT_SCROLLBAR_PSEUDO_CLASSES, name)
                            && match position.previous_element {
                                Some(element) => {
                                    in_set_ignore_case(WEBKIT_SCROLLBAR_PSEUDO_ELEMENTS, element)
                                }
                                None => position.has_nesting && has_scrollbar_ancestor(ctx),
                            })))
        } else {
            in_set_ignore_case(AT_RULE_PAGE_PSEUDO_CLASSES, name)
        };
        if !known {
            ctx.report(
                NAME,
                span,
                format!("Unknown pseudo-class selector \":{name}\""),
            );
        }
    }
}

impl Rule for SelectorPseudoClassNoUnknown {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if !ctx.slice(rule.selector.span).contains(':')
            || !is_standard_syntax_rule(ctx.source, rule)
        {
            return;
        }
        walk_selectors(ctx.source, &rule.selector, |node, position| {
            if let SimpleSelector::PseudoClass(pseudo) = node
                && let InterpolableIdent::Literal(name) = &pseudo.name
            {
                self.check_name(
                    name.raw,
                    position.span(pseudo.span.start, name.span.end),
                    Some(position),
                    ctx,
                );
            }
        });
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if at_rule.name.raw != "page" {
            return;
        }
        let Some(AtRulePrelude::Page(page)) = &at_rule.prelude else {
            return;
        };
        for selector in &page.selectors {
            for pseudo in &selector.pseudo {
                if let InterpolableIdent::Literal(name) = &pseudo.name
                    && name.span.start == pseudo.span.start + 1
                {
                    self.check_name(
                        name.raw,
                        Span {
                            start: pseudo.span.start,
                            end: name.span.end,
                        },
                        None,
                        ctx,
                    );
                }
                if let Some(tokens) = &pseudo.arg
                    && ctx.slice(tokens.span).contains(':')
                {
                    walk_selector_fragment(ctx.source, tokens, |node, position| {
                        if let SimpleSelector::PseudoClass(pseudo) = node
                            && let InterpolableIdent::Literal(name) = &pseudo.name
                        {
                            self.check_name(
                                name.raw,
                                position.span(pseudo.span.start, name.span.end),
                                None,
                                ctx,
                            );
                        }
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NAME;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn scrollbar_and_page_context() {
        Tester::new(
            NAME,
            json!(true),
            [
                "::-webkit-scrollbar-button:horizontal:decrement {}",
                "::-webkit-scrollbar .x:vertical {}",
                "::-webkit-scrollbar { @media all { &:horizontal {} } }",
                "a::selection:window-inactive, a:before, a:--custom, a:-moz-test, :heading(odd) {}",
                "@page foo:blank:left, :Nth(2n) {}",
            ],
            [
                reject(
                    ":first {}",
                    None,
                    "Unknown pseudo-class selector \":first\"",
                    (1, 1, 1, 7),
                ),
                reject(
                    "@page foo:blank:Hover {}",
                    None,
                    "Unknown pseudo-class selector \":Hover\"",
                    (1, 16, 1, 22),
                ),
                reject(
                    "@page :nth(:Mystery) {}",
                    None,
                    "Unknown pseudo-class selector \":Mystery\"",
                    (1, 12, 1, 20),
                ),
                reject(
                    "::-webkit-scrollbar { & :horizontal {} }",
                    None,
                    "Unknown pseudo-class selector \":horizontal\"",
                    (1, 25, 1, 36),
                ),
                reject(
                    "::-webkit-scrollbar { &::before:horizontal {} }",
                    None,
                    "Unknown pseudo-class selector \":horizontal\"",
                    (1, 32, 1, 43),
                ),
            ],
        )
        .test();
    }

    #[test]
    fn ignore_options_and_nested_pseudos() {
        Tester::new(
            NAME,
            json!([true, {"ignorePseudoClasses": ["unknown", "/^custom-/i"]}]),
            [
                "a:unknown, a:CUSTOM-x, a:contains(':not-a-pseudo') {}",
                "@page :unknown {}",
            ],
            [
                reject(
                    "a:is(.x:Nope) {}",
                    None,
                    "Unknown pseudo-class selector \":Nope\"",
                    (1, 8, 1, 13),
                ),
                reject(
                    "a:custom-fn(:Unknown) {}",
                    None,
                    "Unknown pseudo-class selector \":Unknown\"",
                    (1, 13, 1, 21),
                ),
                reject(
                    r"a:ho\76 er {}",
                    None,
                    r#"Unknown pseudo-class selector ":ho\76 er""#,
                    (1, 2, 1, 11),
                ),
            ],
        )
        .test();
    }
}
