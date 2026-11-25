use std::collections::HashMap;

use oxc_css_parser::{
    Allocator, ParserBuilder,
    ast::{
        ComplexSelector, ComplexSelectorChild, QualifiedRule, SelectorList, SimpleSelector,
        Statement, Stylesheet,
    },
};
use serde_json::{Map, Value};

use super::{
    selector_max_specificity::{Specificity, is_nested, specificity},
    selector_no_qualifying_type::resolve_nested_selectors,
};
use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, OptionMatcher, ident_name, in_set_ignore_case,
        is_standard_syntax_rule, options_matches, parse_option_matchers,
    },
};

crate::define_stx_lint! {
    /// Report a less specific selector that follows a more specific selector with
    /// a comparable final compound.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/no-descending-specificity>
    ///
    /// Enable with `true`. The `ignore` option accepts exact selectors or
    /// slash-delimited regexes; `selectors-within-list` skips selector lists with
    /// more than one selector. Nested selectors are resolved, and each at-rule
    /// block is checked separately. This compares selectors in rules with
    /// declarations, not which properties they set or whether their elements overlap.
    ///
    /// `a` is reported here:
    ///
    /// ```css
    /// nav a { color: red; }
    /// a { color: blue; }
    /// ```
    "no-descending-specificity", suspicious, recommended = true;
    pub struct NoDescendingSpecificity {
        ignore: Vec<OptionMatcher>,
        ignore_selectors_within_list: bool,
    }
}

fn has_declarations(statements: &[Statement<'_>]) -> bool {
    statements.iter().any(|stmt| match stmt {
        Statement::Declaration(_) => true,
        Statement::AtRule(rule) => rule
            .block
            .as_ref()
            .is_some_and(|block| has_declarations(&block.statements)),
        _ => false,
    })
}

fn comparison_key(selector: &ComplexSelector<'_>, source: &str) -> Option<String> {
    let ComplexSelectorChild::CompoundSelector(compound) = selector.children.last()? else {
        return None;
    };
    let mut parts: Vec<_> = compound
        .children
        .iter()
        .filter_map(|node| {
            let pseudo_element = match node {
                SimpleSelector::PseudoClass(pseudo) => {
                    if !ident_name(&pseudo.name).is_some_and(|name| {
                        in_set_ignore_case(LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS, name)
                    }) {
                        return None;
                    }
                    true
                }
                SimpleSelector::PseudoElement(_) => true,
                _ => false,
            };
            let span = node.span();
            Some((
                pseudo_element,
                &source[span.start as usize..span.end as usize],
            ))
        })
        .collect();
    if parts.is_empty() {
        return None;
    }
    for segment in parts.split_mut(|(pseudo_element, _)| *pseudo_element) {
        segment.sort_unstable();
    }
    Some(parts.into_iter().map(|(_, raw)| raw).collect())
}

struct Entry {
    label: String,
    specificity: Specificity,
    line: usize,
}

type Seen = HashMap<String, Vec<Entry>>;

impl NoDescendingSpecificity {
    pub fn from_config(secondary: Option<&Map<String, Value>>) -> Self {
        let ignore = parse_option_matchers(secondary, "ignore");
        Self {
            ignore_selectors_within_list: options_matches(&ignore, "selectors-within-list"),
            ignore,
        }
    }

    fn check_rule<'a>(&self, rule: &QualifiedRule<'a>, seen: &mut Seen, ctx: &mut LintContext<'a>) {
        if (self.ignore_selectors_within_list && rule.selector.selectors.len() > 1)
            || !is_standard_syntax_rule(ctx.source, rule)
            || !has_declarations(&rule.block.statements)
        {
            return;
        }
        let line = ctx.line_index.line_col(ctx.source, rule.span.start).0;
        let resolved = is_nested(ctx).then(|| resolve_nested_selectors(rule, ctx));
        for (i, sel) in rule.selector.selectors.iter().enumerate() {
            let raw = ctx.slice(sel.span).trim();
            if options_matches(&self.ignore, raw) {
                continue;
            }
            let resolved = resolved
                .as_ref()
                .map_or(raw, |selectors| selectors[i].trim());
            let source = ctx.source;
            let mut compare = |selector: &ComplexSelector<'_>, source: &str| {
                let Some(key) = comparison_key(selector, source) else {
                    return;
                };
                let spec = specificity(selector);
                let entries = seen.entry(key).or_default();
                let label = if raw == resolved {
                    format!("\"{raw}\"")
                } else {
                    format!("\"{raw}\" (\"{resolved}\")")
                };
                if let Some(prev) = entries.iter().find(|prev| prev.specificity > spec) {
                    ctx.report(
                        self.name(),
                        sel.span,
                        format!(
                            "Expected selector {label} to come before selector {}, at line {}",
                            prev.label, prev.line
                        ),
                    );
                }
                entries.push(Entry {
                    label,
                    specificity: spec,
                    line,
                });
            };
            if raw == resolved {
                compare(sel, source);
            } else {
                let allocator = Allocator::default();
                if let Ok(list) = ParserBuilder::new(&allocator, resolved)
                    .build()
                    .parse::<SelectorList>()
                {
                    for selector in &list.selectors {
                        compare(selector, resolved);
                    }
                }
            }
        }
    }

    fn check_statements<'a>(
        &self,
        statements: &'a [Statement<'a>],
        seen: &mut Seen,
        ctx: &mut LintContext<'a>,
    ) {
        for stmt in statements {
            match stmt {
                Statement::QualifiedRule(rule) => {
                    self.check_rule(rule, seen, ctx);
                    ctx.ancestors.push(AstParent::QualifiedRule(rule));
                    self.check_statements(&rule.block.statements, seen, ctx);
                    ctx.ancestors.pop();
                }
                Statement::AtRule(rule) => {
                    if let Some(block) = &rule.block {
                        ctx.ancestors.push(AstParent::AtRule(rule));
                        self.check_statements(&block.statements, &mut Seen::new(), ctx);
                        ctx.ancestors.pop();
                    }
                }
                _ => {}
            }
        }
    }
}

impl Rule for NoDescendingSpecificity {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        self.check_statements(&stylesheet.statements, &mut Seen::new(), ctx);
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;

    #[test]
    fn includes_previous_selector_line() {
        let fail = [reject_message(
            "#a .bar { color: red; }\n.foo .bar { color: blue; }",
            None,
            "Expected selector \".foo .bar\" to come before selector \"#a .bar\", at line 1",
        )];
        Tester::new("no-descending-specificity", json!(true), [], &fail).test();
    }

    #[test]
    fn compares_last_compounds_without_pseudo_classes() {
        Tester::new(
            "no-descending-specificity",
            json!(true),
            ["a:hover::before { color: red; } a:hover { color: red; }"],
            [
                reject(
                    "a:hover::before { color: red; } a::before { color: red; } a { color: red; }",
                    None,
                    "Expected selector \"a::before\" to come before selector \"a:hover::before\", at line 1",
                    (1, 33, 1, 42),
                ),
                reject_message(
                    "#a .foo.bar { color: red; } .bar.foo { color: blue; }",
                    None,
                    "Expected selector \".bar.foo\" to come before selector \"#a .foo.bar\", at line 1",
                ),
            ],
        ).test();
    }

    #[test]
    fn compares_resolved_selectors_within_the_same_at_rule_context() {
        Tester::new(
            "no-descending-specificity",
            json!(true),
            ["b a {} a {}", "@media print { b a { color: red; } } a { color: blue; }"],
            [
                reject_message(
                    "a { & > b { color: red; } } b { color: blue; }",
                    None,
                    "Expected selector \"b\" to come before selector \"& > b\" (\"a > b\"), at line 1",
                ),
                reject_message(
                    ".foo { & h2 { @media { color: red; } } } h1 { & h2 { @media { color: blue; } } }",
                    None,
                    "Expected selector \"& h2\" (\"h1 h2\") to come before selector \"& h2\" (\".foo h2\"), at line 1",
                ),
            ],
        ).test();
    }

    #[test]
    fn ignores_configured_selectors() {
        Tester::new(
            "no-descending-specificity",
            json!([true, {"ignore": ["#a .bar", "/^\\.foo/"]}]),
            ["#a .bar { color: red; } .foo .bar { color: blue; }"],
            &[],
        )
        .test();
        Tester::new(
            "no-descending-specificity",
            json!([true, {"ignore": ["selectors-within-list"]}]),
            [
                "#a .bar, .foo .bar { color: red; } .bar { color: blue; }",
                "b a { color: red; } a, b { color: blue; }",
            ],
            &[],
        )
        .test();
    }
}
