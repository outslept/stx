use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        OptionMatcher, options_matches, parse_option_matchers, secondary_has_flag,
        selector_pseudos_nested,
    },
};
use oxc_css_parser::ast::{AtRule, QualifiedRule, Statement};
pub struct MaxNestingDepth {
    max: u64,
    ignore_at_rules: Vec<OptionMatcher>,
    ignore_rules: Vec<OptionMatcher>,
    ignore_pseudo_classes: Vec<OptionMatcher>,
    ignore_blockless_at_rules: bool,
    ignore_pseudo_only_rules: bool,
}
impl MaxNestingDepth {
    pub fn from_config(
        p: &serde_json::Value,
        s: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        Some(Self {
            max: p.as_u64()?,
            ignore_at_rules: parse_option_matchers(s, "ignoreAtRules"),
            ignore_rules: parse_option_matchers(s, "ignoreRules"),
            ignore_pseudo_classes: parse_option_matchers(s, "ignorePseudoClasses"),
            ignore_blockless_at_rules: secondary_has_flag(s, "ignore", "blockless-at-rules"),
            ignore_pseudo_only_rules: secondary_has_flag(s, "ignore", "pseudo-classes"),
        })
    }
    fn is_blockless(&self, at_rule: &AtRule<'_>) -> bool {
        self.ignore_blockless_at_rules
            && at_rule.block.as_ref().is_some_and(|block| {
                block
                    .statements
                    .iter()
                    .all(|statement| !matches!(statement, Statement::Declaration(_)))
            })
    }

    fn ignored_at_rule(&self, at_rule: &AtRule<'_>) -> bool {
        options_matches(&self.ignore_at_rules, at_rule.name.name) || self.is_blockless(at_rule)
    }

    fn ignored_pseudo_rule(&self, selector: &str) -> bool {
        selector_pseudos_nested(selector)
            .iter()
            .any(|(_, _, start, end, element)| {
                !element
                    && (self.ignore_pseudo_only_rules
                        || options_matches(&self.ignore_pseudo_classes, &selector[*start..*end]))
            })
    }

    fn ignored_rule(&self, rule: &QualifiedRule<'_>, ctx: &LintContext<'_>) -> bool {
        rule.selector.selectors.iter().all(|selector| {
            let raw = ctx.slice(selector.span);
            options_matches(&self.ignore_rules, raw) || self.ignored_pseudo_rule(raw)
        })
    }

    fn depth(&self, ctx: &LintContext<'_>) -> usize {
        if ctx.ancestors.iter().any(|parent| {
            matches!(parent, AstParent::AtRule(at_rule) if options_matches(&self.ignore_at_rules, at_rule.name.name))
        }) {
            return 0;
        }
        let mut depth = ctx.ancestors.len();
        if matches!(ctx.ancestors.first(), Some(AstParent::AtRule(_))) {
            depth = depth.saturating_sub(1);
        }
        let ignored_ancestors = ctx
            .ancestors
            .iter()
            .filter(|parent| match parent {
                AstParent::AtRule(at_rule) => self.is_blockless(at_rule),
                AstParent::QualifiedRule(rule) => self.ignored_rule(rule, ctx),
                _ => false,
            })
            .count();
        depth.saturating_sub(ignored_ancestors)
    }
}
impl Rule for MaxNestingDepth {
    fn name(&self) -> &'static str {
        "max-nesting-depth"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if self.ignored_rule(rule, ctx) {
            return;
        }
        if self.depth(ctx) as u64 > self.max {
            ctx.report(
                self.name(),
                rule.span,
                format!("Too deep nesting, maximum {}", self.max),
            );
        }
    }
    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if at_rule.block.is_none() {
            return;
        }
        if self.ignored_at_rule(at_rule) {
            return;
        }
        if self.depth(ctx) as u64 > self.max {
            ctx.report(
                self.name(),
                at_rule.span,
                format!("Too deep nesting, maximum {}", self.max),
            );
        }
    }
}
#[cfg(test)]
mod tests {

    use crate::tester::{Tester, reject_message};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec![
            "a { b { top: 0; }}",
            "@media print { a { b { top: 0; }}}",
            "a { top: 0; b { top: 0; }}",
            "a { @nest b { top: 0; }}",
        ];
        let fail = vec![
            reject_message(
                "a { b { c { top: 0; }}}",
                None,
                "Too deep nesting, maximum 1",
            ),
            reject_message(
                "@media print { a { b { c { top: 0; }}}}",
                None,
                "Too deep nesting, maximum 1",
            ),
            reject_message(
                "a { b { top: 0; c { top: 0; }} top: 0; }",
                None,
                "Too deep nesting, maximum 1",
            ),
        ];
        Tester::new("max-nesting-depth", json!([1]), &pass, &fail).test();
    }

    #[test]
    fn ignored_pseudo_class_does_not_hide_deeper_nesting() {
        let fail = [reject_message(
            "a { &:hover { b { c {} } } }",
            None,
            "Too deep nesting, maximum 1",
        )];
        Tester::new(
            "max-nesting-depth",
            json!([1, {"ignorePseudoClasses": ["hover"]}]),
            [],
            &fail,
        )
        .test();
    }
}
