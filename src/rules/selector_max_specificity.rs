use oxc_css_parser::{
    Allocator, ParserBuilder,
    ast::{
        ComplexSelector, ComplexSelectorChild, CompoundSelector, PseudoClassSelectorArgKind as Arg,
        PseudoElementSelectorArgKind, QualifiedRule, SelectorList, SimpleSelector, TypeSelector,
    },
};

use super::selector_no_qualifying_type::resolve_nested_selectors;
use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, ident_name, in_set_ignore_case, is_standard_syntax_rule,
    },
};

pub(super) fn is_nested(ctx: &LintContext<'_>) -> bool {
    ctx.ancestors.iter().any(|parent| {
        matches!(parent, AstParent::QualifiedRule(_))
            || matches!(parent, AstParent::AtRule(rule) if rule.name.name.eq_ignore_ascii_case("scope"))
    })
}

pub(super) type Specificity = (u64, u64, u64);

fn add((a, b, c): Specificity, (x, y, z): Specificity) -> Specificity {
    (a + x, b + y, c + z)
}

fn max_specificity(list: &SelectorList<'_>) -> Specificity {
    list.selectors
        .iter()
        .map(specificity)
        .max()
        .unwrap_or_default()
}

pub(super) fn specificity(selector: &ComplexSelector<'_>) -> Specificity {
    selector
        .children
        .iter()
        .filter_map(|child| match child {
            ComplexSelectorChild::CompoundSelector(compound) => {
                Some(compound_specificity(compound))
            }
            ComplexSelectorChild::Combinator(_) => None,
        })
        .fold(Specificity::default(), add)
}

fn compound_specificity(compound: &CompoundSelector<'_>) -> Specificity {
    compound
        .children
        .iter()
        .map(simple_specificity)
        .fold(Specificity::default(), add)
}

fn simple_specificity(node: &SimpleSelector<'_>) -> Specificity {
    match node {
        SimpleSelector::Id(_) => (1, 0, 0),
        SimpleSelector::Class(_) | SimpleSelector::Attribute(_) => (0, 1, 0),
        SimpleSelector::Type(TypeSelector::TagName(_)) => (0, 0, 1),
        SimpleSelector::PseudoElement(pseudo) => {
            if let Some(arg) = &pseudo.arg {
                match &arg.kind {
                    PseudoElementSelectorArgKind::CompoundSelector(compound)
                        if ident_name(&pseudo.name)
                            .is_some_and(|name| name.eq_ignore_ascii_case("slotted")) =>
                    {
                        return add((0, 0, 1), compound_specificity(compound));
                    }
                    _ => {}
                }
            }
            (0, 0, 1)
        }
        SimpleSelector::PseudoClass(pseudo) => {
            let Some(name) = ident_name(&pseudo.name) else {
                return (0, 0, 0);
            };
            if in_set_ignore_case(LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, name) {
                return (0, 0, 1);
            }
            let arg = pseudo.arg.as_ref().map(|arg| &arg.kind);
            match name.to_ascii_lowercase().as_str() {
                "where" => (0, 0, 0),
                "is" | "not" | "matches" | "-moz-any" | "has" | "host" | "host-context" => {
                    let argument = match arg {
                        Some(Arg::SelectorList(list)) => max_specificity(list),
                        Some(Arg::RelativeSelectorList(list)) => list
                            .selectors
                            .iter()
                            .map(|relative| specificity(&relative.complex_selector))
                            .max()
                            .unwrap_or_default(),
                        Some(Arg::CompoundSelectorList(list)) => list
                            .selectors
                            .iter()
                            .map(compound_specificity)
                            .max()
                            .unwrap_or_default(),
                        _ => (0, 0, 0),
                    };
                    let host = name.eq_ignore_ascii_case("host")
                        || name.eq_ignore_ascii_case("host-context");
                    add((0, u64::from(host), 0), argument)
                }
                "nth-child" | "nth-last-child" => {
                    let argument = match arg {
                        Some(Arg::Nth(nth)) => nth
                            .matcher
                            .as_ref()
                            .and_then(|m| m.selector.as_ref())
                            .map(max_specificity)
                            .unwrap_or_default(),
                        _ => (0, 0, 0),
                    };
                    add((0, 1, 0), argument)
                }
                _ => (0, 1, 0),
            }
        }
        _ => (0, 0, 0),
    }
}

pub struct SelectorMaxSpecificity {
    max: Specificity,
    raw: String,
}
impl SelectorMaxSpecificity {
    pub fn from_config(p: &serde_json::Value) -> Option<Self> {
        let raw = p.as_str()?;
        let parts: Vec<u64> = raw
            .split(',')
            .map(|s| s.trim().parse().ok())
            .collect::<Option<_>>()?;
        if parts.len() != 3 {
            return None;
        }
        Some(Self {
            max: (parts[0], parts[1], parts[2]),
            raw: raw.to_string(),
        })
    }
}
impl Rule for SelectorMaxSpecificity {
    fn name(&self) -> &'static str {
        "selector-max-specificity"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if !is_standard_syntax_rule(ctx.source, rule) {
            return;
        }
        let resolved = is_nested(ctx).then(|| resolve_nested_selectors(rule, ctx));
        for (i, sel) in rule.selector.selectors.iter().enumerate() {
            let raw = ctx.slice(sel.span).trim();
            let resolved = resolved
                .as_ref()
                .map_or(raw, |selectors| selectors[i].trim());
            let spec = if raw == resolved {
                specificity(sel)
            } else {
                let allocator = Allocator::default();
                let Ok(list) = ParserBuilder::new(&allocator, resolved)
                    .build()
                    .parse::<SelectorList>()
                else {
                    continue;
                };
                max_specificity(&list)
            };
            if spec > self.max {
                let selector = if raw != resolved {
                    format!("\"{raw}\" (\"{resolved}\")")
                } else {
                    format!("\"{raw}\"")
                };
                ctx.report(
                    self.name(),
                    sel.span,
                    format!(
                        "Too high specificity in {selector}, maximum \"{}\"",
                        self.raw
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;

    #[test]
    fn counts_functional_pseudos() {
        Tester::new(
            "selector-max-specificity",
            json!(["0,1,0"]),
            [
                ":not(.a, .b) {}",
                ":matches(.a, .b) {}",
                ":is(.a) {}",
                ":where(#a p) {}",
                ":nth-child(2n + 1) {}",
            ],
            [
                reject_message(
                    ".a:not(.b, .c) {}",
                    None,
                    "Too high specificity in \".a:not(.b, .c)\", maximum \"0,1,0\"",
                ),
                reject_message(
                    ":has(> .a.b) {}",
                    None,
                    "Too high specificity in \":has(> .a.b)\", maximum \"0,1,0\"",
                ),
                reject_message(
                    ":nth-child(2n of .a) {}",
                    None,
                    "Too high specificity in \":nth-child(2n of .a)\", maximum \"0,1,0\"",
                ),
            ],
        )
        .test();
    }

    #[test]
    fn checks_resolved_specificity_at_the_original_selector() {
        Tester::new(
            "selector-max-specificity",
            json!(["0,2,0"]),
            [".a, .b { & .c {} }", "@scope (#id) { .a {} }"],
            [reject(
                ".a { .b { & > .c {} } }",
                None,
                "Too high specificity in \"& > .c\" (\":is(.a .b) > .c\"), maximum \"0,2,0\"",
                (1, 11, 1, 17),
            )],
        )
        .test();
    }

    #[test]
    fn skips_interpolated_selectors() {
        for (syntax, code) in [
            ("postcss-scss", "#id #{$name} {}"),
            ("postcss-less", "#id @{name} {}"),
        ] {
            Tester::new("selector-max-specificity", json!(["0,1,0"]), [code], &[])
                .with_custom_syntax(syntax)
                .test();
        }
    }
}
