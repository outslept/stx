use oxc_css_parser::ast::{
    CombinatorKind, ComplexSelector, ComplexSelectorChild, CompoundSelector,
    PseudoClassSelectorArgKind as Arg, PseudoElementSelectorArgKind as ElementArg, QualifiedRule,
    SimpleSelector, TypeSelector,
};

use super::selector_max_specificity::is_nested;
use crate::{context::LintContext, rules::Rule, utils::*};

pub struct R {
    max: u64,
    ignore_types: Vec<OptionMatcher>,
    ignore_descendant: bool,
    ignore_child: bool,
    ignore_compounded: bool,
    ignore_next_sibling: bool,
    ignore_custom_elements: bool,
}
impl R {
    pub fn from_config(
        p: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        Some(Self {
            max: p.as_u64()?,
            ignore_types: parse_option_matchers(secondary, "ignoreTypes"),
            ignore_descendant: secondary_has_flag(secondary, "ignore", "descendant"),
            ignore_child: secondary_has_flag(secondary, "ignore", "child"),
            ignore_compounded: secondary_has_flag(secondary, "ignore", "compounded"),
            ignore_next_sibling: secondary_has_flag(secondary, "ignore", "next-sibling"),
            ignore_custom_elements: secondary_has_flag(secondary, "ignore", "custom-elements"),
        })
    }

    fn count_types(
        &self,
        selector: &ComplexSelector<'_>,
        leading: Option<&CombinatorKind>,
        implicit_descendant: bool,
        has_nesting: &mut bool,
    ) -> usize {
        let mut direct = 0;
        let mut in_pseudos = 0;
        let mut previous = leading;
        let mut after_descendant = leading == Some(&CombinatorKind::Descendant);
        let mut after_child = leading == Some(&CombinatorKind::Child);
        for child in &selector.children {
            match child {
                ComplexSelectorChild::Combinator(combinator) => {
                    previous = Some(&combinator.kind);
                    after_descendant |= combinator.kind == CombinatorKind::Descendant;
                    after_child |= combinator.kind == CombinatorKind::Child;
                }
                ComplexSelectorChild::CompoundSelector(compound) => {
                    let ignore_tag = (self.ignore_descendant && after_descendant)
                        || (self.ignore_child && after_child)
                        || (self.ignore_next_sibling
                            && previous == Some(&CombinatorKind::NextSibling));
                    direct += self.count_tags(compound, ignore_tag);
                    in_pseudos += self.count_arguments(compound, has_nesting);
                    previous = None;
                }
            }
        }
        in_pseudos
            + if implicit_descendant && !*has_nesting {
                0
            } else {
                direct
            }
    }

    fn count_tags(&self, compound: &CompoundSelector<'_>, ignore_tag: bool) -> usize {
        if ignore_tag || (self.ignore_compounded && compound.children.len() > 1) {
            return 0;
        }
        compound
            .children
            .iter()
            .filter(|node| {
                let SimpleSelector::Type(TypeSelector::TagName(tag)) = node else {
                    return false;
                };
                ident_name(&tag.name.name).is_some_and(|name| {
                    !options_matches(&self.ignore_types, name)
                        && !(self.ignore_custom_elements && is_custom_element(name))
                })
            })
            .count()
    }

    fn count_arguments(&self, compound: &CompoundSelector<'_>, has_nesting: &mut bool) -> usize {
        compound
            .children
            .iter()
            .map(|node| match node {
                SimpleSelector::Nesting(_) => {
                    *has_nesting = true;
                    0
                }
                SimpleSelector::PseudoClass(pseudo) => {
                    match pseudo.arg.as_ref().map(|arg| &arg.kind) {
                        Some(Arg::SelectorList(list)) => list
                            .selectors
                            .iter()
                            .map(|sel| self.count_types(sel, None, false, has_nesting))
                            .sum(),
                        Some(Arg::RelativeSelectorList(list)) => list
                            .selectors
                            .iter()
                            .map(|rel| {
                                self.count_types(
                                    &rel.complex_selector,
                                    rel.combinator.as_ref().map(|c| &c.kind),
                                    false,
                                    has_nesting,
                                )
                            })
                            .sum(),
                        Some(Arg::CompoundSelectorList(list)) => list
                            .selectors
                            .iter()
                            .map(|sel| {
                                self.count_tags(sel, false) + self.count_arguments(sel, has_nesting)
                            })
                            .sum(),
                        Some(Arg::Nth(nth)) => nth
                            .matcher
                            .as_ref()
                            .and_then(|m| m.selector.as_ref())
                            .map_or(0, |list| {
                                list.selectors
                                    .iter()
                                    .map(|sel| self.count_types(sel, None, false, has_nesting))
                                    .sum()
                            }),
                        _ => 0,
                    }
                }
                SimpleSelector::PseudoElement(pseudo) => {
                    match pseudo.arg.as_ref().map(|arg| &arg.kind) {
                        Some(ElementArg::CompoundSelector(sel)) => {
                            self.count_tags(sel, false) + self.count_arguments(sel, has_nesting)
                        }
                        Some(ElementArg::CompoundSelectorList(list)) => list
                            .selectors
                            .iter()
                            .map(|sel| {
                                self.count_tags(sel, false) + self.count_arguments(sel, has_nesting)
                            })
                            .sum(),
                        _ => 0,
                    }
                }
                _ => 0,
            })
            .sum()
    }
}
impl Rule for R {
    fn name(&self) -> &'static str {
        "selector-max-type"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if !is_standard_syntax_rule(ctx.source, rule) || ctx.in_keyframes() {
            return;
        }
        let implicit_descendant = self.ignore_descendant && is_nested(ctx);
        for sel in &rule.selector.selectors {
            if self.count_types(sel, None, implicit_descendant, &mut false) as u64 > self.max {
                let s = ctx.slice(sel.span).trim();
                ctx.report(
                    self.name(),
                    sel.span,
                    format!("Too many type selectors in \"{}\", maximum {}", s, self.max),
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
    fn ignores_anb_and_non_selector_arguments() {
        Tester::new(
            "selector-max-type",
            json!([0, {"ignore": ["custom-elements"]}]),
            [
                ":nth-child(odd) {}",
                ":nth-child(2n + 1) {}",
                "custom-element::part(foo) {}",
            ],
            [reject_message(
                ":nth-child(2n of div) {}",
                None,
                "Too many type selectors in \":nth-child(2n of div)\", maximum 0",
            )],
        )
        .test();
    }

    #[test]
    fn ignores_explicit_and_implicit_descendants() {
        Tester::new(
            "selector-max-type",
            json!([0, {"ignore": ["descendant"]}]),
            [
                ".a div {}",
                ".a { div, a {} }",
                "@scope (.a) { div {} }",
                ".a .b > div {}",
            ],
            [
                reject_message(
                    ".a > div {}",
                    None,
                    "Too many type selectors in \".a > div\", maximum 0",
                ),
                reject_message(
                    ".a { div:is(&) {} }",
                    None,
                    "Too many type selectors in \"div:is(&)\", maximum 0",
                ),
                reject_message(
                    ".a { :is(div) {} }",
                    None,
                    "Too many type selectors in \":is(div)\", maximum 0",
                ),
            ],
        )
        .test();
    }

    #[test]
    fn distinguishes_child_and_next_sibling() {
        Tester::new(
            "selector-max-type",
            json!([0, {"ignore": ["child"]}]),
            [".a > .b + div {}"],
            &[],
        )
        .test();
        Tester::new(
            "selector-max-type",
            json!([0, {"ignore": ["next-sibling"]}]),
            [".a { & .b + div {} }"],
            [reject_message(
                ".a + .b div {}",
                None,
                "Too many type selectors in \".a + .b div\", maximum 0",
            )],
        )
        .test();
    }

    #[test]
    fn ignores_compounded_types() {
        Tester::new(
            "selector-max-type",
            json!([0, {"ignore": ["compounded"]}]),
            ["a.foo {}", "div[data-foo] {}"],
            [reject_message(
                ".a div {}",
                None,
                "Too many type selectors in \".a div\", maximum 0",
            )],
        )
        .test();
    }
}
