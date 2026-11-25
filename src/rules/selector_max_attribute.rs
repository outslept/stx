use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, has_interpolation, options_matches, parse_option_matchers,
        selector_attribute_blocks_nested,
    },
};
use oxc_css_parser::ast::QualifiedRule;
pub struct SelectorMaxAttribute {
    max: u64,
    ignore: Vec<OptionMatcher>,
}
impl SelectorMaxAttribute {
    pub fn from_config(
        p: &serde_json::Value,
        s: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        Some(Self {
            max: p.as_u64()?,
            ignore: parse_option_matchers(s, "ignoreAttributes"),
        })
    }
    fn name_in(raw: &str, (start, end): (usize, usize)) -> &str {
        let bytes = raw.as_bytes();
        let mut i = start;
        while i < end && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let s = i;
        while i < end
            && !bytes[i].is_ascii_whitespace()
            && !matches!(bytes[i], b'~' | b'|' | b'^' | b'$' | b'*' | b'=')
        {
            i += 1;
        }
        &raw[s..i]
    }
}
impl Rule for SelectorMaxAttribute {
    fn name(&self) -> &'static str {
        "selector-max-attribute"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        for sel in &rule.selector.selectors {
            let raw = ctx.slice(sel.span);
            let s = raw.trim();
            if has_interpolation(s) {
                continue;
            }
            let mut count = 0u64;
            for block in selector_attribute_blocks_nested(s) {
                let name = Self::name_in(s, block);
                if options_matches(&self.ignore, name) {
                    continue;
                }
                count += 1;
            }
            if count > self.max {
                ctx.report(
                    self.name(),
                    sel.span,
                    format!(
                        "Too many attribute selectors in \"{s}\", maximum {}",
                        self.max
                    ),
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject_message};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec!["foo {}", ".bar {}", "foo .bar {}"];
        let fail = vec![
            reject_message(
                "[foo] {}",
                None,
                "Too many attribute selectors in \"[foo]\", maximum 0",
            ),
            reject_message(
                "a[rel=\"external\"] {}",
                None,
                "Too many attribute selectors in \"a[rel=\"external\"]\", maximum 0",
            ),
        ];
        Tester::new("selector-max-attribute", json!([0]), &pass, &fail).test();
    }
    #[test]
    fn ignores_scss_interpolation() {
        Tester::new(
            "selector-max-attribute",
            json!([0]),
            ["#{$selector}[foo] {}"],
            [],
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
