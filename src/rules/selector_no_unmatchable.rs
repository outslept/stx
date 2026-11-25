use oxc_css_parser::{
    Allocator, ParserBuilder, Span,
    ast::{
        ComplexSelector, ComplexSelectorChild, InterpolableIdent, PseudoClassSelectorArgKind,
        PseudoElementSelectorArgKind, QualifiedRule, SelectorList, SimpleSelector, TypeSelector,
    },
};

use super::selector_no_qualifying_type::resolve_nested_selectors;
use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        ELEMENT_REPRESENTING_PSEUDO_ELEMENTS, HTML_TYPE_SELECTORS,
        LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, TREE_STRUCTURAL_PSEUDO_CLASSES, ident_name,
        in_set_ignore_case, is_standard_syntax_rule, normalize_combinator, strip_comments,
    },
};

crate::define_stx_lint! {
    /// Reports selector combinations that cannot match an element or pseudo-element.
    ///
    /// Set to `true`. Checks known incompatible selectors, including after resolving
    /// nesting. It does not determine whether a selector matches a particular document
    /// and does not fix violations.
    ///
    /// Example: `:is(::before) {}` cannot match because `:is()` cannot represent pseudo-elements.
    "selector-no-unmatchable", correctness;
    pub struct SelectorNoUnmatchable;
}

fn text(source: &str, span: Span) -> &str {
    &source[span.start as usize..span.end as usize]
}

fn pseudo_element<'a>(node: &'a SimpleSelector<'_>) -> Option<&'a str> {
    match node {
        SimpleSelector::PseudoElement(pseudo) => Some(ident_name(&pseudo.name).unwrap_or_default()),
        SimpleSelector::PseudoClass(pseudo) => ident_name(&pseudo.name)
            .filter(|name| in_set_ignore_case(LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, name)),
        _ => None,
    }
}

fn applicable_elements(name: &str) -> &'static [&'static str] {
    match name {
        "any-link" | "link" | "visited" => &["a", "area"],
        "autofill" | "optional" | "required" => &["input", "select", "textarea"],
        "checked" => &["input", "option"],
        "default" => &["button", "input", "option"],
        "disabled" | "enabled" => &[
            "button", "fieldset", "input", "optgroup", "option", "select", "textarea",
        ],
        "in-range" | "out-of-range" => &["input"],
        "invalid" | "user-invalid" | "user-valid" | "valid" => {
            &["button", "fieldset", "form", "input", "select", "textarea"]
        }
        "placeholder-shown" => &["input", "textarea"],
        _ => &[],
    }
}

fn html_tag<'a>(nodes: &'a [SimpleSelector<'_>]) -> Option<(&'a str, &'a str)> {
    for node in nodes {
        match node {
            SimpleSelector::Type(TypeSelector::TagName(tag)) if tag.name.prefix.is_none() => {
                if let InterpolableIdent::Literal(name) = &tag.name.name
                    && in_set_ignore_case(HTML_TYPE_SELECTORS, name.name)
                {
                    return Some((name.name, name.raw));
                }
            }
            SimpleSelector::PseudoClass(pseudo)
                if ident_name(&pseudo.name)
                    .is_some_and(|name| in_set_ignore_case(&["is", "where"], name)) =>
            {
                if let Some(arg) = &pseudo.arg
                    && let PseudoClassSelectorArgKind::SelectorList(list) = &arg.kind
                    && let [selector] = list.selectors.as_slice()
                    && let [ComplexSelectorChild::CompoundSelector(compound)] =
                        selector.children.as_slice()
                    && let Some(tag) = html_tag(&compound.children)
                {
                    return Some(tag);
                }
            }
            _ => {}
        }
    }
    None
}

fn is_host(node: &SimpleSelector<'_>) -> bool {
    matches!(node, SimpleSelector::PseudoClass(pseudo) if ident_name(&pseudo.name).is_some_and(|name| name.eq_ignore_ascii_case("host")))
}

#[derive(Default)]
struct Reasons {
    forgiving: Option<&'static str>,
    shadow: Option<String>,
    structural: Option<String>,
    applicability: Option<String>,
}

impl Reasons {
    fn complex(
        &mut self,
        source: &str,
        selector: &ComplexSelector<'_>,
        forgiving: Option<&'static str>,
        relational: bool,
    ) {
        let mut after_combinator = false;
        for child in &selector.children {
            match child {
                ComplexSelectorChild::Combinator(_) => after_combinator = true,
                ComplexSelectorChild::CompoundSelector(compound) => self.compound(
                    source,
                    &compound.children,
                    forgiving,
                    relational,
                    after_combinator,
                ),
            }
        }
    }

    fn compound(
        &mut self,
        source: &str,
        nodes: &[SimpleSelector<'_>],
        forgiving: Option<&'static str>,
        relational: bool,
        after_combinator: bool,
    ) {
        let mut element = None;
        let mut applicable = Vec::<(&str, &[&str])>::new();
        for (i, node) in nodes.iter().enumerate() {
            if let Some(name) = pseudo_element(node) {
                element = Some(name);
                if !relational {
                    self.forgiving = self.forgiving.or(forgiving);
                }
            }
            if self.shadow.is_none() && !relational && is_host(node) {
                if after_combinator {
                    self.shadow = Some(
                        "the shadow host has no ancestors or siblings in its shadow tree".into(),
                    );
                } else if let Some(feature) = nodes[..i]
                    .iter()
                    .rev()
                    .chain(
                        nodes[i + 1..]
                            .iter()
                            .take_while(|node| pseudo_element(node).is_none()),
                    )
                    .find(|node| {
                        matches!(
                            node,
                            SimpleSelector::Type(TypeSelector::TagName(_))
                                | SimpleSelector::Class(_)
                                | SimpleSelector::Id(_)
                                | SimpleSelector::Attribute(_)
                        )
                    })
                {
                    self.shadow = Some(format!(
                        "\"{}\" never matches the shadow host",
                        text(source, feature.span())
                    ));
                }
            }
            match node {
                SimpleSelector::PseudoClass(pseudo) => {
                    let Some(name) = ident_name(&pseudo.name) else {
                        continue;
                    };
                    let name = name.to_ascii_lowercase();
                    let value = text(
                        source,
                        Span {
                            start: pseudo.span.start,
                            end: pseudo.name.span().end,
                        },
                    );
                    if let Some(element) = element {
                        if self.structural.is_none()
                            && !in_set_ignore_case(ELEMENT_REPRESENTING_PSEUDO_ELEMENTS, element)
                            && in_set_ignore_case(TREE_STRUCTURAL_PSEUDO_CLASSES, &name)
                        {
                            self.structural =
                                Some(format!("\"{value}\" never matches pseudo-elements"));
                        }
                    } else if !applicable_elements(&name).is_empty() {
                        applicable.push((value, applicable_elements(&name)));
                    }
                    if let Some(arg) = &pseudo.arg
                        && name != "not"
                    {
                        let forgiving = match name.as_str() {
                            "is" => Some("is"),
                            "where" => Some("where"),
                            _ => None,
                        };
                        let relational = relational || matches!(name.as_str(), "has" | "not");
                        match &arg.kind {
                            PseudoClassSelectorArgKind::SelectorList(list) => {
                                for selector in &list.selectors {
                                    self.complex(source, selector, forgiving, relational);
                                }
                            }
                            PseudoClassSelectorArgKind::RelativeSelectorList(list) => {
                                for relative in &list.selectors {
                                    self.complex(
                                        source,
                                        &relative.complex_selector,
                                        forgiving,
                                        relational,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
                SimpleSelector::PseudoElement(pseudo)
                    if self.shadow.is_none()
                        && ident_name(&pseudo.name)
                            .is_some_and(|name| name.eq_ignore_ascii_case("slotted")) =>
                {
                    if let Some(arg) = &pseudo.arg
                        && let PseudoElementSelectorArgKind::CompoundSelectorList(list) = &arg.kind
                        && list
                            .selectors
                            .iter()
                            .any(|compound| compound.children.iter().any(is_host))
                    {
                        self.shadow = Some("slotted elements are never the shadow host".into());
                    }
                }
                _ => {}
            }
        }
        if self.applicability.is_some() || applicable.is_empty() {
            return;
        }
        if let Some((tag, raw)) = html_tag(nodes) {
            for &(value, elements) in &applicable {
                if !in_set_ignore_case(elements, tag) {
                    self.applicability =
                        Some(format!("\"{value}\" never matches \"{raw}\" elements"));
                    return;
                }
            }
        }
        for (i, &(first, elements)) in applicable.iter().enumerate() {
            for &(second, other) in &applicable[i + 1..] {
                if !elements.iter().any(|name| other.contains(name)) {
                    self.applicability = Some(format!(
                        "\"{first}\" and \"{second}\" never match the same element"
                    ));
                    return;
                }
            }
        }
    }
}

impl Rule for SelectorNoUnmatchable {
    fn name(&self) -> &'static str {
        Self::NAME
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if ctx.in_keyframes() || !is_standard_syntax_rule(ctx.source, rule) {
            return;
        }
        let nested = ctx.ancestors.iter().any(|parent| {
            matches!(parent, AstParent::QualifiedRule(_))
                || matches!(parent, AstParent::AtRule(at_rule) if at_rule.name.name.eq_ignore_ascii_case("scope"))
        });
        let resolved = nested.then(|| resolve_nested_selectors(rule, ctx));
        for (i, selector) in rule.selector.selectors.iter().enumerate() {
            let raw = ctx.slice(selector.span).trim_end();
            let mut reasons = Reasons::default();
            reasons.complex(ctx.source, selector, None, false);
            let mut reason = reasons
                .forgiving
                .map(|name| format!("pseudo-elements cannot be represented by \":{name}()\""));
            let resolved = resolved.as_ref().map(|list| list[i].trim());
            let mut display = None;
            if let Some(resolved) = resolved
                && (reason.is_none() || resolved.contains("/*"))
            {
                let allocator = Allocator::default();
                if let Ok(list) = ParserBuilder::new(&allocator, resolved)
                    .build()
                    .parse::<SelectorList>()
                {
                    if reason.is_none() {
                        reasons = Reasons::default();
                        for selector in &list.selectors {
                            reasons.complex(resolved, selector, None, false);
                        }
                        if reasons.forgiving.is_some() {
                            reason = Some("\"&\" cannot represent pseudo-elements".into());
                        }
                    }
                    if resolved.contains("/*") {
                        for selector in list.selectors.iter().rev() {
                            for child in selector.children.iter().rev() {
                                if let ComplexSelectorChild::Combinator(combinator) = child
                                    && let value = text(resolved, combinator.span)
                                    && value.contains("/*")
                                {
                                    display
                                        .get_or_insert_with(|| resolved.to_owned())
                                        .replace_range(
                                            combinator.span.start as usize
                                                ..combinator.span.end as usize,
                                            &normalize_combinator(&strip_comments(value)),
                                        );
                                }
                            }
                        }
                    }
                }
            }
            if let Some(reason) = reason
                .or(reasons.shadow)
                .or(reasons.structural)
                .or(reasons.applicability)
            {
                let name = if let Some(resolved) = display.as_deref().or(resolved) {
                    format!("\"{raw}\" (\"{resolved}\")")
                } else {
                    format!("\"{raw}\"")
                };
                ctx.report(
                    self.name(),
                    Span {
                        start: selector.span.start,
                        end: selector.span.start + raw.len() as u32,
                    },
                    format!("Unmatchable selector {name}, {reason}"),
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_message, reject_warnings};
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

    #[test]
    fn reports_each_unmatchable_selector_without_matching_strings_or_negations() {
        let pass = [
            r#":is([data-value="::before"]) {}"#,
            ":not(:where(::before)) {}",
            ":not(label:enabled) {}",
            ":is(label, button):enabled {}",
            "::slotted(a):first-child {}",
            ":host(.foo) a {}",
        ];
        let fail = [
            reject_warnings(
                "label:enabled, p:checked {}",
                None,
                vec![
                    ("Unmatchable selector \"label:enabled\", \":enabled\" never matches \"label\" elements".into(), (1, 1, 1, 14)),
                    ("Unmatchable selector \"p:checked\", \":checked\" never matches \"p\" elements".into(), (1, 16, 1, 25)),
                ],
            ),
            reject_message(
                "a { & /* foo */ :host {} }",
                None,
                "Unmatchable selector \"& /* foo */ :host\" (\"a :host\"), the shadow host has no ancestors or siblings in its shadow tree",
            ),
            reject_message(
                "a::before { &:hover {} }",
                None,
                "Unmatchable selector \"&:hover\" (\":is(a::before):hover\"), \"&\" cannot represent pseudo-elements",
            ),
            reject_message(
                "::before:first-child {}",
                None,
                "Unmatchable selector \"::before:first-child\", \":first-child\" never matches pseudo-elements",
            ),
        ];
        Tester::new("selector-no-unmatchable", json!(true), pass, fail).test();
    }
}
