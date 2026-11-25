use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, options_matches, parse_primary_option_matchers, secondary_bool,
        secondary_has_flag,
    },
};
use oxc_css_parser::{Span, ast::QualifiedRule};

pub fn reject_msg(selector: &str) -> String {
    format!("Disallowed selector \"{selector}\"")
}

pub struct SelectorDisallowedList {
    disallowed: Vec<OptionMatcher>,
    ignore_inside_block: bool,
    split_list: bool,
}
impl SelectorDisallowedList {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        Self {
            disallowed: parse_primary_option_matchers(primary),
            ignore_inside_block: secondary_has_flag(secondary, "ignore", "inside-block"),
            split_list: secondary_bool(secondary, "splitList").unwrap_or(false),
        }
    }
}
impl Rule for SelectorDisallowedList {
    fn name(&self) -> &'static str {
        "selector-disallowed-list"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if self.ignore_inside_block && !ctx.ancestors.is_empty() {
            return;
        }
        if self.split_list {
            for sel in &rule.selector.selectors {
                let raw = ctx.slice(sel.span);
                let trimmed = raw.trim();
                if options_matches(&self.disallowed, trimmed) {
                    ctx.report(self.name(), sel.span, reject_msg(trimmed));
                }
            }
        } else {
            let raw = ctx.slice(rule.selector.span);
            if options_matches(&self.disallowed, raw.trim()) {
                ctx.report(self.name(), rule.selector.span, reject_msg(raw.trim()));
            }
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
            "a {}",
            ".foo {}",
            "a\n>\n.foo {}",
            ".bar > a > .foo {}",
            ".foo > a {}",
            ".data-auto {}",
            "a[href] {}",
        ];
        let fail = vec![
            reject("a > .foo {}", None, reject_msg("a > .foo"), (1, 1, 1, 9)),
            reject(
                "a[data-auto=\"1\"] {}",
                None,
                reject_msg("a[data-auto=\"1\"]"),
                (1, 1, 1, 17),
            ),
            reject(
                "a[data-auto] {}",
                None,
                reject_msg("a[data-auto]"),
                (1, 1, 1, 13),
            ),
            reject(
                ".foo, [data-auto=\"1\"] {}",
                None,
                reject_msg(".foo, [data-auto=\"1\"]"),
                (1, 1, 1, 22),
            ),
        ];
        Tester::new(
            "selector-disallowed-list",
            json!(["a > .foo", "/\\[data-.+\\]/"]),
            &pass,
            &fail,
        )
        .test();
    }
}
