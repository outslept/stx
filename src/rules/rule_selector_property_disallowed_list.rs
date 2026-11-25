use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, ident_name, options_matches},
};
use oxc_css_parser::ast::{Declaration, QualifiedRule, SimpleBlock, Statement};

pub struct RuleSelectorPropertyDisallowedList {
    entries: Vec<(OptionMatcher, Vec<OptionMatcher>)>,
}
impl RuleSelectorPropertyDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Option<Self> {
        let object = primary.as_object()?;
        let mut entries = Vec::with_capacity(object.len());
        for (key, value) in object {
            let mut props = Vec::new();
            match value {
                serde_json::Value::String(s) => props.push(OptionMatcher::parse(s)),
                serde_json::Value::Array(items) => {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            props.push(OptionMatcher::parse(s));
                        }
                    }
                }
                _ => {}
            }
            entries.push((OptionMatcher::parse(key), props));
        }
        Some(Self { entries })
    }
    fn check_block<'a>(
        &self,
        block: &'a SimpleBlock<'a>,
        selector: &str,
        props: &[OptionMatcher],
        ctx: &mut LintContext<'a>,
    ) {
        for statement in &block.statements {
            match statement {
                Statement::Declaration(Declaration { name, span, .. }) => {
                    if let Some(prop) = ident_name(name) {
                        if options_matches(props, prop) {
                            ctx.report(
                                self.name(),
                                oxc_css_parser::Span {
                                    start: span.start,
                                    end: span.start + prop.len() as u32,
                                },
                                format!(
                                    "Disallowed property \"{prop}\" for selector \"{selector}\""
                                ),
                            );
                        }
                    }
                }
                Statement::AtRule(at_rule) => {
                    if let Some(inner) = &at_rule.block {
                        self.check_block(inner, selector, props, ctx);
                    }
                }
                Statement::KeyframeBlock(kf) => self.check_block(&kf.block, selector, props, ctx),
                _ => {}
            }
        }
    }
}
impl Rule for RuleSelectorPropertyDisallowedList {
    fn name(&self) -> &'static str {
        "rule-selector-property-disallowed-list"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        let selector = ctx.slice(rule.selector.span);
        let mut found = None;
        for (matcher, props) in &self.entries {
            if matcher.is_match(selector) {
                found = Some(props);
                break;
            }
        }
        if let Some(props) = found {
            self.check_block(&rule.block, selector, props, ctx);
        }
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
            "a { background: red; }",
            "a { padding-top: 0px; }",
            "a { background-color: red; }",
            "a { b { color: red; } }",
            "b { & a { color: red; } }",
            "a[href=\"#\"] { color: red; }",
            "html.foo { color: red; }",
            "html[data-foo] { color: red; }",
        ];
        let fail = vec![
            reject(
                "a { color: red; }",
                None,
                "Disallowed property \"color\" for selector \"a\"",
                (1, 5, 1, 10),
            ),
            reject(
                "a { background: red; color: red; }",
                None,
                "Disallowed property \"color\" for selector \"a\"",
                (1, 22, 1, 27),
            ),
            reject(
                "a { @media screen { color: red; } }",
                None,
                "Disallowed property \"color\" for selector \"a\"",
                (1, 21, 1, 26),
            ),
            reject(
                "a { margin-top: 0px; }",
                None,
                "Disallowed property \"margin-top\" for selector \"a\"",
                (1, 5, 1, 15),
            ),
        ];
        Tester::new(
            "rule-selector-property-disallowed-list",
            json!({"a": ["color", "/margin/"], "/foo/": "/size/"}),
            &pass,
            &fail,
        )
        .test();
    }
}
