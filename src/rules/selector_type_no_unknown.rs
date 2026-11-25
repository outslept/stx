use oxc_css_parser::{
    Allocator, ParserBuilder, Span,
    ast::{
        ComplexSelector, ComplexSelectorChild, CompoundSelector, InterpolableIdent, NsPrefixKind,
        PseudoClassSelectorArgKind, PseudoElementSelectorArgKind, QualifiedRule, SelectorList,
        SimpleSelector, TokenSeq, TypeSelector,
    },
};
use serde_json::{Map, Value};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        AN_PLUS_B_NOTATION_PSEUDO_CLASSES, AN_PLUS_B_OF_S_NOTATION_PSEUDO_CLASSES,
        HTML_TYPE_SELECTORS, LINGUISTIC_PSEUDO_CLASSES, MATHML_TYPE_SELECTORS, OptionMatcher,
        SHADOW_TREE_PSEUDO_ELEMENTS, SVG_TYPE_SELECTORS, ident_name, in_set_ignore_case,
        is_custom_element, is_standard_syntax_rule, options_matches, parse_option_matchers,
        secondary_has_flag,
    },
};

const NAME: &str = "selector-type-no-unknown";
const MAX_FRAGMENT_PARENS: usize = 64;

#[derive(Clone, Copy, Default)]
pub(super) struct SelectorPosition<'a> {
    offset: u32,
    argument_of: Option<(&'a str, bool)>,
    pub(super) previous_element: Option<&'a str>,
    pub(super) has_nesting: bool,
}

impl SelectorPosition<'_> {
    pub(super) fn span(self, start: u32, end: u32) -> Span {
        Span {
            start: self.offset + start,
            end: self.offset + end,
        }
    }
}

pub(super) fn walk_selectors(
    source: &str,
    list: &SelectorList<'_>,
    check: impl FnMut(&SimpleSelector<'_>, SelectorPosition<'_>),
) {
    SelectorWalker { source, check }.list(list, SelectorPosition::default());
}

pub(super) fn walk_selector_fragment(
    source: &str,
    tokens: &TokenSeq<'_>,
    check: impl FnMut(&SimpleSelector<'_>, SelectorPosition<'_>),
) {
    SelectorWalker { source, check }.fragment(tokens, SelectorPosition::default());
}

struct SelectorWalker<'a, F> {
    source: &'a str,
    check: F,
}

impl<F: FnMut(&SimpleSelector<'_>, SelectorPosition<'_>)> SelectorWalker<'_, F> {
    fn list(&mut self, list: &SelectorList<'_>, at: SelectorPosition<'_>) {
        for selector in &list.selectors {
            self.complex(selector, at);
        }
    }

    fn complex<'a>(&mut self, selector: &ComplexSelector<'a>, mut at: SelectorPosition<'a>) {
        for child in &selector.children {
            if let ComplexSelectorChild::CompoundSelector(compound) = child {
                self.compound(compound, &mut at);
            }
        }
    }

    fn compound<'a>(&mut self, compound: &CompoundSelector<'a>, at: &mut SelectorPosition<'a>) {
        at.has_nesting = false;
        for node in &compound.children {
            (self.check)(node, *at);
            let mut inner = SelectorPosition {
                offset: at.offset,
                ..SelectorPosition::default()
            };
            match node {
                SimpleSelector::Nesting(_) => at.has_nesting = true,
                SimpleSelector::PseudoClass(pseudo) => {
                    if let InterpolableIdent::Literal(name) = &pseudo.name {
                        inner.argument_of = Some((name.raw, false));
                    }
                    let Some(arg) = &pseudo.arg else { continue };
                    match &arg.kind {
                        PseudoClassSelectorArgKind::SelectorList(list) => self.list(list, inner),
                        PseudoClassSelectorArgKind::RelativeSelectorList(list) => {
                            for relative in &list.selectors {
                                self.complex(&relative.complex_selector, inner);
                            }
                        }
                        PseudoClassSelectorArgKind::CompoundSelectorList(list) => {
                            for compound in &list.selectors {
                                self.compound(compound, &mut inner.clone());
                            }
                        }
                        PseudoClassSelectorArgKind::Nth(nth) => {
                            if let Some(list) =
                                nth.matcher.as_ref().and_then(|m| m.selector.as_ref())
                            {
                                self.list(list, inner);
                            }
                        }
                        PseudoClassSelectorArgKind::TokenSeq(tokens) => {
                            self.fragment(tokens, inner)
                        }
                        _ => {}
                    }
                }
                SimpleSelector::PseudoElement(pseudo) => {
                    if let InterpolableIdent::Literal(name) = &pseudo.name {
                        at.previous_element = Some(name.raw);
                        inner.argument_of = Some((name.raw, true));
                    } else {
                        at.previous_element = Some("");
                    }
                    let Some(arg) = &pseudo.arg else { continue };
                    match &arg.kind {
                        PseudoElementSelectorArgKind::CompoundSelector(compound) => {
                            self.compound(compound, &mut inner);
                        }
                        PseudoElementSelectorArgKind::CompoundSelectorList(list) => {
                            for compound in &list.selectors {
                                self.compound(compound, &mut inner.clone());
                            }
                        }
                        PseudoElementSelectorArgKind::TokenSeq(tokens) => {
                            self.fragment(tokens, inner);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn fragment(&mut self, tokens: &TokenSeq<'_>, mut at: SelectorPosition<'_>) {
        if tokens.tokens.is_empty() {
            return;
        }
        let span = at.span(tokens.span.start, tokens.span.end);
        let Some(raw) = self.source.get(span.start as usize..span.end as usize) else {
            return;
        };
        if raw.matches('(').nth(MAX_FRAGMENT_PARENS - 1).is_some() {
            return;
        }
        let allocator = Allocator::default();
        let mut parser = ParserBuilder::new(&allocator, raw).build();
        if let Ok(list) = parser.parse::<SelectorList>() {
            at.offset = span.start;
            self.list(&list, at);
        }
    }
}

fn ignores_argument_type(name: &str, element: bool) -> bool {
    in_set_ignore_case(AN_PLUS_B_NOTATION_PSEUDO_CLASSES, name)
        || in_set_ignore_case(AN_PLUS_B_OF_S_NOTATION_PSEUDO_CLASSES, name)
        || in_set_ignore_case(LINGUISTIC_PSEUDO_CLASSES, name)
        || in_set_ignore_case(SHADOW_TREE_PSEUDO_ELEMENTS, name)
        || matches!(
            (name, element),
            (
                "highlight"
                    | "view-transition-group"
                    | "view-transition-image-pair"
                    | "view-transition-new"
                    | "view-transition-old",
                true
            ) | (
                "active-view-transition-type" | "-moz-locale-dir" | "state",
                false
            )
        )
}

fn may_have_type(selector: &str) -> bool {
    let bytes = selector.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_alphabetic)
        || bytes.windows(2).any(|pair| {
            pair[1].is_ascii_alphabetic()
                && !pair[0].is_ascii_alphabetic()
                && !b".#[:-".contains(&pair[0])
        })
}

pub struct SelectorTypeNoUnknown {
    ignore_custom_elements: bool,
    ignore_default_namespace: bool,
    ignore_namespaces: Vec<OptionMatcher>,
    ignore_types: Vec<OptionMatcher>,
}

impl SelectorTypeNoUnknown {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_custom_elements: secondary_has_flag(secondary, "ignore", "custom-elements"),
            ignore_default_namespace: secondary_has_flag(secondary, "ignore", "default-namespace"),
            ignore_namespaces: parse_option_matchers(secondary, "ignoreNamespaces"),
            ignore_types: parse_option_matchers(secondary, "ignoreTypes"),
        }
    }
}

impl Rule for SelectorTypeNoUnknown {
    fn name(&self) -> &'static str {
        NAME
    }

    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if ctx.in_keyframes()
            || !is_standard_syntax_rule(ctx.source, rule)
            || !may_have_type(ctx.slice(rule.selector.span))
        {
            return;
        }

        walk_selectors(ctx.source, &rule.selector, |node, at| {
            let SimpleSelector::Type(TypeSelector::TagName(tag)) = node else {
                return;
            };
            let InterpolableIdent::Literal(ident) = &tag.name.name else {
                return;
            };
            let namespace = match tag
                .name
                .prefix
                .as_ref()
                .and_then(|prefix| prefix.kind.as_ref())
            {
                Some(NsPrefixKind::Universal(_)) => Some("*"),
                Some(NsPrefixKind::Ident(name)) => ident_name(name),
                None => None,
            };
            let name = ident.name;
            if (self.ignore_default_namespace && namespace.is_none())
                || namespace.is_some_and(|ns| options_matches(&self.ignore_namespaces, ns))
                || options_matches(&self.ignore_types, name)
                || (self.ignore_custom_elements && is_custom_element(name))
                || at
                    .argument_of
                    .is_some_and(|(name, element)| ignores_argument_type(name, element))
                || name.starts_with('%')
                || (name.starts_with('/') && name.ends_with('/'))
            {
                return;
            }
            let lower = name.to_lowercase();
            if HTML_TYPE_SELECTORS.contains(&lower.as_str())
                || SVG_TYPE_SELECTORS.contains(&name)
                || MATHML_TYPE_SELECTORS.contains(&lower.as_str())
            {
                return;
            }

            let end = if name == ident.raw {
                ident.span.end
            } else {
                ident
                    .raw
                    .char_indices()
                    .nth(name.chars().count())
                    .map_or(ident.span.end, |(offset, _)| {
                        ident.span.start + offset as u32
                    })
            };
            ctx.report(
                NAME,
                at.span(ident.span.start, end),
                format!("Unknown type selector \"{name}\""),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::NAME;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn known_types_and_function_arguments() {
        Tester::new(
            NAME,
            json!(true),
            [
                "acronym, HGrOuP, foreignObject, color-profile, ABS {}",
                "a:nth-child(2n of mystery)::part(panel), ::highlight(example), :state(opened) {}",
                "@keyframes fade { from {} to {} }",
            ],
            [
                reject(
                    "input:not(unknown) {}",
                    None,
                    "Unknown type selector \"unknown\"",
                    (1, 11, 1, 18),
                ),
                reject(
                    "a::slotted(mystery) {}",
                    None,
                    "Unknown type selector \"mystery\"",
                    (1, 12, 1, 19),
                ),
                reject(
                    "svg|foreignobject {}",
                    None,
                    "Unknown type selector \"foreignobject\"",
                    (1, 5, 1, 18),
                ),
                reject(
                    "x-widget {}",
                    None,
                    "Unknown type selector \"x-widget\"",
                    (1, 1, 1, 9),
                ),
                reject(
                    r"t\61 g {}",
                    None,
                    "Unknown type selector \"tag\"",
                    (1, 1, 1, 4),
                ),
            ],
        )
        .test();
    }

    #[test]
    fn namespace_and_type_ignores() {
        Tester::new(
            NAME,
            json!([true, {
                "ignore": ["custom-elements", "default-namespace"],
                "ignoreNamespaces": ["*", "/^my-/i"],
                "ignoreTypes": ["Ignored"],
            }]),
            ["bare, |bare, *|bare, MY-ns|bare, ns|x-widget, ns|Ignored {}"],
            [
                reject(
                    "ns|x-Widget {}",
                    None,
                    "Unknown type selector \"x-Widget\"",
                    (1, 4, 1, 12),
                ),
                reject(
                    "ns|ignored {}",
                    None,
                    "Unknown type selector \"ignored\"",
                    (1, 4, 1, 11),
                ),
            ],
        )
        .test();
    }
}
