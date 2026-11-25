use std::borrow::Cow;

use oxc_css_parser::{
    Allocator, ParserBuilder, Span,
    ast::{
        ComplexSelector, ComplexSelectorChild, Ident, InterpolableIdent, PseudoClassSelector,
        PseudoClassSelectorArgKind, QualifiedRule, SelectorList, SimpleSelector, TypeSelector,
    },
};

use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, ident_name, in_set_ignore_case, is_standard_syntax_rule,
        secondary_has_flag,
    },
};

const NAME: &str = SelectorNoQualifyingType::NAME;
const MAX_ALTERNATIVES: usize = 10_000;

crate::define_stx_lint! {
    /// Disallows qualifying an ID, class, or attribute selector with a type selector.
    ///
    /// Set to `true`. Use `ignore` with `"id"`, `"class"`, or `"attribute"` to exempt
    /// those qualifiers. Checks compounds after resolving nesting; a type separated
    /// by a combinator is not a qualifier. Does not fix violations.
    ///
    /// Example: `a[href='place']:hover {}` is reported, but `a:hover [attribute] {}` is not.
    "selector-no-qualifying-type", restriction;
    pub struct SelectorNoQualifyingType {
        ignore_id: bool,
        ignore_class: bool,
        ignore_attribute: bool,
    }
}

#[derive(Clone, Copy)]
struct Tag<'a> {
    ident: &'a Ident<'a>,
    original: &'a [SimpleSelector<'a>],
}

#[derive(Clone, Default)]
struct Branch<'a> {
    has_node: bool,
    qualified: bool,
    tags: Vec<Tag<'a>>,
}

fn is_pseudo_element(node: &SimpleSelector<'_>) -> bool {
    match node {
        SimpleSelector::PseudoElement(_) => true,
        SimpleSelector::PseudoClass(pseudo) => ident_name(&pseudo.name)
            .is_some_and(|name| in_set_ignore_case(LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, name)),
        _ => false,
    }
}

fn joins_compound(pseudo: &PseudoClassSelector<'_>) -> bool {
    pseudo.arg.is_some()
        && ident_name(&pseudo.name).is_some_and(|name| {
            in_set_ignore_case(&["is", "matches", "not", "where", "host"], name)
        })
}

fn keep_in_message(node: &SimpleSelector<'_>) -> bool {
    match node {
        SimpleSelector::Type(TypeSelector::TagName(_))
        | SimpleSelector::Class(_)
        | SimpleSelector::Id(_)
        | SimpleSelector::Attribute(_) => true,
        SimpleSelector::PseudoClass(pseudo) => joins_compound(pseudo),
        _ => false,
    }
}

fn append_qualified<'a>(branches: Vec<Branch<'a>>, tags: &mut Vec<Tag<'a>>) {
    for branch in branches {
        if branch.qualified {
            tags.extend(branch.tags);
        }
    }
}

fn text(source: &str, span: Span) -> &str {
    &source[span.start as usize..span.end as usize]
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SelectorOrder {
    Universal,
    Tag,
    PseudoElement,
    Nesting,
    Id,
    Class,
    Attribute,
    Other,
}

fn selector_order(node: &SimpleSelector<'_>) -> SelectorOrder {
    match node {
        SimpleSelector::Type(TypeSelector::Universal(_)) => SelectorOrder::Universal,
        SimpleSelector::Type(TypeSelector::TagName(_)) => SelectorOrder::Tag,
        SimpleSelector::Nesting(_) => SelectorOrder::Nesting,
        SimpleSelector::Id(_) => SelectorOrder::Id,
        SimpleSelector::Class(_) => SelectorOrder::Class,
        SimpleSelector::Attribute(_) => SelectorOrder::Attribute,
        _ if is_pseudo_element(node) => SelectorOrder::PseudoElement,
        _ => SelectorOrder::Other,
    }
}

struct Nesting<'a> {
    source: &'a str,
    parent: &'a str,
    wrapped: String,
    parent_nodes: &'a [SimpleSelector<'a>],
}

impl<'a> Nesting<'a> {
    fn render(&self, selector: &ComplexSelector<'_>, in_has: bool, nested: &mut bool) -> String {
        let mut output = String::new();
        for child in &selector.children {
            let compound = match child {
                ComplexSelectorChild::CompoundSelector(compound) => compound,
                ComplexSelectorChild::Combinator(combinator) => {
                    output.push(' ');
                    let name = text(self.source, combinator.span).trim();
                    if !name.is_empty() {
                        output.push_str(name);
                        output.push(' ');
                    }
                    continue;
                }
            };
            let mut parts = Vec::with_capacity(compound.children.len());
            let mut substituted = false;
            for node in &compound.children {
                if let SimpleSelector::Nesting(nesting) = node
                    && !self.parent.is_empty()
                {
                    *nested = true;
                    if nesting.suffix.is_none() {
                        substituted = true;
                        if in_has || self.parent_nodes.is_empty() {
                            parts
                                .push((SelectorOrder::Other, Cow::Borrowed(self.wrapped.as_str())));
                        } else {
                            parts.extend(self.parent_nodes.iter().map(|node| {
                                let raw = text(self.parent, node.span());
                                (selector_order(node), Cow::Borrowed(raw))
                            }));
                        }
                        continue;
                    }
                }
                parts.push((selector_order(node), self.render_simple(node, nested)));
            }
            if substituted {
                for segment in parts.split_mut(|(order, _)| *order == SelectorOrder::PseudoElement)
                {
                    let mut seen_tag = false;
                    for (order, value) in &mut *segment {
                        if *order == SelectorOrder::Tag && std::mem::replace(&mut seen_tag, true) {
                            *order = SelectorOrder::Other;
                            *value = Cow::Owned(format!(":is({value})"));
                        }
                    }
                    segment.sort_by_key(|(order, _)| *order);
                }
            }
            for (_, part) in parts {
                output.push_str(&part);
            }
        }
        output
    }

    fn render_simple<'b>(&'b self, node: &SimpleSelector<'_>, nested: &mut bool) -> Cow<'b, str> {
        let raw = text(self.source, node.span());
        let SimpleSelector::PseudoClass(pseudo) = node else {
            return Cow::Borrowed(raw);
        };
        let Some(arg) = &pseudo.arg else {
            return Cow::Borrowed(raw);
        };
        let args: Vec<_> = match &arg.kind {
            PseudoClassSelectorArgKind::SelectorList(list) => list
                .selectors
                .iter()
                .map(|selector| self.render(selector, false, nested))
                .collect(),
            PseudoClassSelectorArgKind::RelativeSelectorList(list) => list
                .selectors
                .iter()
                .map(|relative| {
                    let selector = self.render(&relative.complex_selector, true, nested);
                    if let Some(combinator) = &relative.combinator {
                        format!(" {} {selector}", text(self.source, combinator.span).trim())
                    } else {
                        selector
                    }
                })
                .collect(),
            _ => return Cow::Borrowed(raw),
        };
        let prefix = &raw[..(arg.l_paren.end - pseudo.span.start) as usize];
        Cow::Owned(format!("{prefix}{})", args.join(",")))
    }

    fn resolve(&self, selector: &ComplexSelector<'_>) -> String {
        let mut nested = false;
        let resolved = self.render(selector, false, &mut nested);
        let leading_combinator = matches!(
            selector.children.first(),
            Some(ComplexSelectorChild::Combinator(_))
        );
        if !self.parent.is_empty() && (!nested || leading_combinator) {
            let parent = if self.parent_nodes.is_empty() {
                &self.wrapped
            } else {
                self.parent
            };
            format!("{parent} {}", resolved.trim_start())
        } else {
            resolved
        }
    }
}

fn resolve_nested(parent: &str, list: &SelectorList<'_>, source: &str) -> Vec<String> {
    let repeats = list.selectors.len()
        + text(source, list.span)
            .bytes()
            .filter(|&byte| byte == b'&')
            .count();
    let parent = if parent.len().saturating_mul(repeats) > 1 << 20 {
        ""
    } else {
        parent
    };
    let allocator = Allocator::default();
    let parsed = ParserBuilder::new(&allocator, parent)
        .build()
        .parse::<SelectorList>();
    let parent_nodes = parsed
        .as_ref()
        .ok()
        .and_then(|parents| {
            let [selector] = parents.selectors.as_slice() else {
                return None;
            };
            let [ComplexSelectorChild::CompoundSelector(compound)] = selector.children.as_slice()
            else {
                return None;
            };
            (!compound.children.iter().any(is_pseudo_element))
                .then_some(compound.children.as_slice())
        })
        .unwrap_or_default();
    let nesting = Nesting {
        source,
        parent,
        wrapped: format!(":is({parent})"),
        parent_nodes,
    };
    list.selectors
        .iter()
        .map(|selector| nesting.resolve(selector))
        .collect()
}

pub(crate) fn resolve_nested_selectors(
    rule: &QualifiedRule<'_>,
    ctx: &LintContext<'_>,
) -> Vec<String> {
    let mut parent = String::new();
    for ancestor in &ctx.ancestors {
        match ancestor {
            AstParent::QualifiedRule(rule) => {
                parent = resolve_nested(&parent, &rule.selector, ctx.source).join(",");
            }
            AstParent::AtRule(rule) if rule.name.name.eq_ignore_ascii_case("scope") => {
                parent = ":where(:scope)".into();
            }
            _ => {}
        }
    }
    resolve_nested(&parent, &rule.selector, ctx.source)
}

impl SelectorNoQualifyingType {
    pub fn new(s: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        Self {
            ignore_id: secondary_has_flag(s, "ignore", "id"),
            ignore_class: secondary_has_flag(s, "ignore", "class"),
            ignore_attribute: secondary_has_flag(s, "ignore", "attribute"),
        }
    }

    fn group_selector<'a>(
        &self,
        selector: &'a ComplexSelector<'a>,
        original: Option<&'a [SimpleSelector<'a>]>,
        tags: &mut Vec<Tag<'a>>,
    ) -> Vec<Branch<'a>> {
        let mut branches = Vec::new();
        for child in &selector.children {
            match child {
                ComplexSelectorChild::CompoundSelector(compound) => {
                    for (index, nodes) in compound.children.split(is_pseudo_element).enumerate() {
                        append_qualified(std::mem::take(&mut branches), tags);
                        branches = self.group_compound(nodes, original, tags);
                        if index != 0 {
                            for branch in &mut branches {
                                branch.has_node = true;
                            }
                        }
                    }
                }
                ComplexSelectorChild::Combinator(_) => {
                    append_qualified(std::mem::take(&mut branches), tags);
                }
            }
        }
        branches
    }

    fn group_compound<'a>(
        &self,
        nodes: &'a [SimpleSelector<'a>],
        original: Option<&'a [SimpleSelector<'a>]>,
        tags: &mut Vec<Tag<'a>>,
    ) -> Vec<Branch<'a>> {
        let original = original.unwrap_or_else(|| {
            let end = nodes.iter().rposition(keep_in_message).map_or(0, |i| i + 1);
            &nodes[..end]
        });
        let mut branches = vec![Branch::default()];
        for node in nodes {
            if let SimpleSelector::PseudoClass(pseudo) = node {
                match pseudo.arg.as_ref().map(|arg| &arg.kind) {
                    Some(PseudoClassSelectorArgKind::SelectorList(list))
                        if joins_compound(pseudo) =>
                    {
                        let mut alternatives = Vec::new();
                        for selector in &list.selectors {
                            for next in self.group_selector(selector, Some(original), tags) {
                                if !next.has_node {
                                    continue;
                                }
                                let remaining = MAX_ALTERNATIVES - alternatives.len();
                                for prior in branches.iter().take(remaining) {
                                    let mut combined = prior.clone();
                                    combined.has_node = true;
                                    combined.qualified |= next.qualified;
                                    combined.tags.extend_from_slice(&next.tags);
                                    alternatives.push(combined);
                                }
                            }
                        }
                        if !alternatives.is_empty() {
                            branches = alternatives;
                        }
                        continue;
                    }
                    Some(PseudoClassSelectorArgKind::RelativeSelectorList(list)) => {
                        for relative in &list.selectors {
                            let relative_branches =
                                self.group_selector(&relative.complex_selector, None, tags);
                            append_qualified(relative_branches, tags);
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            let qualifies = match node {
                SimpleSelector::Id(_) => !self.ignore_id,
                SimpleSelector::Class(_) => !self.ignore_class,
                SimpleSelector::Attribute(_) => !self.ignore_attribute,
                _ => false,
            };
            for branch in &mut branches {
                branch.has_node = true;
                branch.qualified |= qualifies;
                if let SimpleSelector::Type(TypeSelector::TagName(tag)) = node
                    && let InterpolableIdent::Literal(ident) = &tag.name.name
                {
                    branch.tags.push(Tag { ident, original });
                }
            }
        }
        branches
    }

    fn report_selector(
        &self,
        selector: &ComplexSelector<'_>,
        source: &str,
        nested_span: Option<Span>,
        ctx: &mut LintContext<'_>,
    ) {
        let mut tags = Vec::new();
        let branches = self.group_selector(selector, None, &mut tags);
        append_qualified(branches, &mut tags);
        for Tag { ident, original } in tags {
            let compound: String = original
                .iter()
                .map(|node| text(source, node.span()))
                .collect();
            let span = nested_span.unwrap_or_else(|| {
                let mut span = ident.span;
                if ident.raw != ident.name
                    && let Some((end, _)) = ident.raw.char_indices().nth(ident.name.chars().count())
                {
                    span.end = span.start + end as u32;
                }
                span
            });
            ctx.report(
                NAME,
                span,
                format!(
                    "Disallowed qualifying type selector \"{}\" in \"{}\"",
                    ident.name,
                    compound.trim()
                ),
            );
        }
    }
}

impl Rule for SelectorNoQualifyingType {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if (self.ignore_id && self.ignore_class && self.ignore_attribute)
            || ctx.in_keyframes()
            || !is_standard_syntax_rule(ctx.source, rule)
            || !ctx.slice(rule.selector.span).contains(['#', '.', '['])
        {
            return;
        }
        let mut parent = String::new();
        for ancestor in &ctx.ancestors {
            match ancestor {
                AstParent::QualifiedRule(rule) => {
                    if !is_standard_syntax_rule(ctx.source, rule) {
                        return;
                    }
                    parent = resolve_nested(&parent, &rule.selector, ctx.source).join(",");
                }
                AstParent::AtRule(rule) if rule.name.name.eq_ignore_ascii_case("scope") => {
                    parent = ":where(:scope)".into();
                }
                _ => {}
            }
        }
        if parent.is_empty() {
            for selector in &rule.selector.selectors {
                self.report_selector(selector, ctx.source, None, ctx);
            }
            return;
        }
        let resolved = resolve_nested(&parent, &rule.selector, ctx.source);
        for (selector, resolved) in rule.selector.selectors.iter().zip(resolved) {
            let allocator = Allocator::default();
            if let Ok(list) = ParserBuilder::new(&allocator, &resolved)
                .build()
                .parse::<SelectorList>()
            {
                for resolved_selector in &list.selectors {
                    self.report_selector(resolved_selector, &resolved, Some(selector.span), ctx);
                }
            } else {
                self.report_selector(selector, ctx.source, Some(selector.span), ctx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NAME;
    use crate::tester::{FailCase, Tester, reject, reject_warnings};
    use serde_json::json;

    fn rejected(compound: &str, tag: &str) -> String {
        format!("Disallowed qualifying type selector \"{tag}\" in \"{compound}\"")
    }

    fn warning<'a>(
        code: &'a str,
        compound: &str,
        tag: &str,
        span: (usize, usize, usize, usize),
    ) -> FailCase<'a> {
        reject(code, None, rejected(compound, tag), span)
    }

    #[test]
    fn compounds() {
        let pass = [
            "div #id { }",
            "div .class { }",
            "a:hover [attribute] { }",
            "a:active { }",
            "p::after { }",
            ".class#id[target] { }",
        ];
        let fail = [
            warning("div#thing { }", "div#thing", "div", (1, 1, 1, 4)),
            warning("a div#thing { }", "div#thing", "div", (1, 3, 1, 6)),
            warning("div#thing::before { }", "div#thing", "div", (1, 1, 1, 4)),
            warning(
                "a[href='place']:hover { }",
                "a[href='place']",
                "a",
                (1, 1, 1, 2),
            ),
            warning("a/* comment */.class {}", "a.class", "a", (1, 1, 1, 2)),
        ];
        Tester::new(NAME, json!(true), pass, fail).test();
    }

    #[test]
    fn functional_pseudo_classes() {
        let pass = [
            "a:is([attribute] :hover) {}",
            "[attribute]:is(a :hover) {}",
            ":is(a, [attribute]) {}",
            ".foo:has(a) {}",
        ];
        let repeated = "a:is(:is(.b, .c), :is(.d, .e))";
        let fail = [
            warning(
                "a:is([attribute]) {}",
                "a:is([attribute])",
                "a",
                (1, 1, 1, 2),
            ),
            warning(
                ".class/* comment */:is(a) {}",
                ".class:is(a)",
                "a",
                (1, 24, 1, 25),
            ),
            reject_warnings(
                ".foo:has(a.bar, a.baz) {}",
                None,
                vec![
                    (rejected("a.bar", "a"), (1, 10, 1, 11)),
                    (rejected("a.baz", "a"), (1, 17, 1, 18)),
                ],
            ),
            reject_warnings(
                "a :is(a, [attribute]):is(b, .class) a {}",
                None,
                vec![
                    (
                        rejected(":is(a, [attribute]):is(b, .class)", "b"),
                        (1, 26, 1, 27),
                    ),
                    (
                        rejected(":is(a, [attribute]):is(b, .class)", "a"),
                        (1, 7, 1, 8),
                    ),
                ],
            ),
            reject_warnings(
                "a:is(:is(.b, .c), :is(.d, .e)) {}",
                None,
                vec![(rejected(repeated, "a"), (1, 1, 1, 2)); 4],
            ),
        ];
        Tester::new(NAME, json!(true), pass, fail).test();
    }

    #[test]
    fn nested_compounds() {
        let pass = [".class { &.class-two {} }", "a { & .class {} }"];
        let fail = [
            warning("a { &.class {} }", "a.class", "a", (1, 5, 1, 12)),
            warning(
                "div a { &[attribute] {} }",
                "[attribute]:is(div a)",
                "a",
                (1, 9, 1, 21),
            ),
            warning(
                "div { & > a { &[attribute] {} } }",
                "[attribute]:is(div > a)",
                "a",
                (1, 15, 1, 27),
            ),
            warning(
                "div {\n\tdiv {\n\t\ta#id {}\n\t}\n}",
                "a#id",
                "a",
                (3, 3, 3, 7),
            ),
        ];
        Tester::new(NAME, json!(true), pass, &fail).test();
        for syntax in ["postcss-scss", "postcss-less"] {
            Tester::new(NAME, json!(true), pass, &fail)
                .with_custom_syntax(syntax)
                .test();
        }
    }

    #[test]
    fn ignores() {
        let id_case = warning("div#thing { }", "div#thing", "div", (1, 1, 1, 4));
        let class_case = warning("ul.list { }", "ul.list", "ul", (1, 1, 1, 3));
        let attribute_case = warning("a[href='place'] { }", "a[href='place']", "a", (1, 1, 1, 2));
        for (ignore, accepted, rejected) in [
            ("id", &id_case, &class_case),
            ("class", &class_case, &attribute_case),
            ("attribute", &attribute_case, &id_case),
        ] {
            Tester::new(
                NAME,
                json!([true, {"ignore": [ignore]}]),
                [accepted.code],
                std::slice::from_ref(rejected),
            )
            .test();
        }
    }
}
