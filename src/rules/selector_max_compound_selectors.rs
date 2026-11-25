use crate::{context::LintContext, rules::Rule, utils::*};
use oxc_css_parser::{
    Span,
    ast::{
        Combinator, CombinatorKind, ComplexSelector, ComplexSelectorChild, CompoundSelector,
        PseudoClassSelectorArgKind, PseudoElementSelectorArgKind, QualifiedRule, SimpleSelector,
    },
};
use serde_json::{Map, Value};

pub struct R {
    max: u64,
    ignore: Vec<OptionMatcher>,
}
impl R {
    pub fn from_config(p: &Value, secondary: Option<&Map<String, Value>>) -> Option<Self> {
        Some(Self {
            max: p.as_u64()?,
            ignore: parse_option_matchers(secondary, "ignoreSelectors"),
        })
    }

    fn count(&self, selector: &ComplexSelector<'_>, ctx: &LintContext<'_>) -> usize {
        if self.ignore.is_empty() {
            return selector_combinators_nested(ctx.slice(selector.span).trim()).len() + 1;
        }
        let mut nodes = Vec::new();
        walk_selector(selector, &mut |node| {
            if !options_matches(&self.ignore, node.value(ctx)) {
                nodes.push(matches!(node, SelectorNode::Combinator(_)));
            }
        });
        nodes
            .windows(2)
            .skip(1)
            .filter(|pair| pair[0] && !pair[1])
            .count()
            + 1
    }
}

pub(super) enum SelectorNode<'n, 'a> {
    Combinator(&'n Combinator),
    Simple(&'n SimpleSelector<'a>),
    Nested(Span),
}

impl SelectorNode<'_, '_> {
    pub(super) fn value<'a>(&self, ctx: &LintContext<'a>) -> &'a str {
        match self {
            Self::Combinator(node) if node.kind == CombinatorKind::Descendant => " ",
            Self::Combinator(node) => ctx.slice(node.span),
            Self::Simple(SimpleSelector::PseudoClass(node)) => {
                ctx.slice_range(node.span.start, node.name.span().end)
            }
            Self::Simple(SimpleSelector::PseudoElement(node)) => {
                ctx.slice_range(node.span.start, node.name.span().end)
            }
            Self::Simple(node) => ctx.slice(node.span()),
            Self::Nested(span) => ctx.slice(*span),
        }
    }
}

pub(super) fn walk_selector(
    selector: &ComplexSelector<'_>,
    visit: &mut impl FnMut(SelectorNode<'_, '_>),
) {
    for child in &selector.children {
        match child {
            ComplexSelectorChild::Combinator(node) => visit(SelectorNode::Combinator(node)),
            ComplexSelectorChild::CompoundSelector(node) => walk_compound(node, visit),
        }
    }
}

fn walk_compound(compound: &CompoundSelector<'_>, visit: &mut impl FnMut(SelectorNode<'_, '_>)) {
    for node in &compound.children {
        visit(SelectorNode::Simple(node));
        let complex_list = match node {
            SimpleSelector::PseudoClass(pseudo) => match pseudo.arg.as_ref().map(|arg| &arg.kind) {
                Some(PseudoClassSelectorArgKind::SelectorList(list)) => Some(list),
                Some(PseudoClassSelectorArgKind::Nth(nth)) => {
                    nth.matcher.as_ref().and_then(|m| m.selector.as_ref())
                }
                Some(PseudoClassSelectorArgKind::RelativeSelectorList(list)) => {
                    for selector in &list.selectors {
                        visit(SelectorNode::Nested(selector.span));
                        if let Some(node) = &selector.combinator {
                            visit(SelectorNode::Combinator(node));
                        }
                        walk_selector(&selector.complex_selector, visit);
                    }
                    None
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(list) = complex_list {
            for selector in &list.selectors {
                visit(SelectorNode::Nested(selector.span));
                walk_selector(selector, visit);
            }
        }
        let compounds: &[CompoundSelector<'_>] = match node {
            SimpleSelector::PseudoClass(pseudo) => match pseudo.arg.as_ref().map(|arg| &arg.kind) {
                Some(PseudoClassSelectorArgKind::CompoundSelectorList(list)) => &list.selectors,
                _ => &[],
            },
            SimpleSelector::PseudoElement(pseudo) => match pseudo.arg.as_ref().map(|arg| &arg.kind)
            {
                Some(PseudoElementSelectorArgKind::CompoundSelector(selector)) => {
                    std::slice::from_ref(selector)
                }
                Some(PseudoElementSelectorArgKind::CompoundSelectorList(list)) => &list.selectors,
                _ => &[],
            },
            _ => &[],
        };
        for selector in compounds {
            visit(SelectorNode::Nested(selector.span));
            walk_compound(selector, visit);
        }
    }
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
            let c = self.count(sel, ctx);
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
    fn ignores_matching_nodes_before_counting_compounds() {
        Tester::new(
            "selector-max-compound-selectors",
            json!([2, {"ignoreSelectors": [".ignored", "::v-deep", "/-moz-.*/", "/-screen$/"]}]),
            [
                ".ignored .foo .bar {}",
                ".foo ::v-deep .bar {}",
                ".foo ::v-deep > .bar {}",
                ".foo > ::v-deep .bar {}",
                ".foo :-webkit-full-screen a {}",
                ".foo::v-deep :-webkit-full-screen a ::-moz-placeholder {}",
                ".foo { & ::v-deep > .bar {} }",
                ".foo .bar > .ignored.ignored {}",
                ".foo .bar > .ignored .ignored {}",
                ":not(.foo > .ignored > .bar) {}",
            ],
            [reject_message(
                ".foo .bar > .ignored.baz {}",
                None,
                "Too many compound selectors in \".foo .bar > .ignored.baz\", maximum 2",
            )],
        )
        .test();
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
