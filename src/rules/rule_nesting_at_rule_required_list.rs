use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{OptionMatcher, options_matches, parse_primary_option_matchers},
};
use oxc_css_parser::ast::QualifiedRule;

pub struct RuleNestingAtRuleRequiredList {
    required: Vec<OptionMatcher>,
    patterns: Vec<String>,
}
impl RuleNestingAtRuleRequiredList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        let patterns = match primary {
            serde_json::Value::String(s) => vec![s.clone()],
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            _ => Vec::new(),
        };
        Self {
            required: parse_primary_option_matchers(primary),
            patterns,
        }
    }
}
pub fn expected_message(patterns: &[String]) -> String {
    let joined = patterns
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Expected rule to be nested in an at-rule matching pattern(s): {joined}")
}
impl Rule for RuleNestingAtRuleRequiredList {
    fn name(&self) -> &'static str {
        "rule-nesting-at-rule-required-list"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        let found = ctx.ancestors.iter().any(
            |p| matches!(p, AstParent::AtRule(a) if options_matches(&self.required, a.name.name)),
        );
        if found {
            return;
        }
        ctx.report(self.name(), rule.span, expected_message(&self.patterns));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec![
            "@layer { a { color: red; } }",
            "@supports (color: red) { a { color: red; } }",
            "@scope (a) { a { color: red; } }",
            "@media all {}",
            "@layer { @media all { a { color: red; } } }",
            "@font-face { font-family: \"foo\"; }",
        ];
        let fail = vec![
            reject(
                "a { color: red; }",
                None,
                expected_message(&["layer".into(), "/^s/".into()]),
                (1, 1, 1, 18),
            ),
            reject(
                "@media all { a { color: red; } }",
                None,
                expected_message(&["layer".into(), "/^s/".into()]),
                (1, 14, 1, 31),
            ),
            reject(
                "a { @layer { color: red; } }",
                None,
                expected_message(&["layer".into(), "/^s/".into()]),
                (1, 1, 1, 29),
            ),
        ];
        Tester::new(
            "rule-nesting-at-rule-required-list",
            json!(["layer", "/^s/"]),
            &pass,
            &fail,
        )
        .test();
    }
}
