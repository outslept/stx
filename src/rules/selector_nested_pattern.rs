use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{has_interpolation, secondary_bool},
};
use oxc_css_parser::ast::QualifiedRule;
use regex::Regex;

fn compile(primary: &serde_json::Value) -> Option<(Regex, String)> {
    let raw = primary.as_str()?;
    if raw.starts_with('/')
        && let Some(last) = raw.rfind('/')
        && last > 0
    {
        let pattern = &raw[1..last];
        let flags = &raw[last + 1..];
        let pat = if flags.contains('i') {
            format!("(?i){}", pattern)
        } else {
            pattern.to_string()
        };
        return Regex::new(&pat).ok().map(|r| (r, raw.to_string()));
    }
    Regex::new(raw).ok().map(|r| (r, raw.to_string()))
}

crate::define_stx_lint! {
    /// Requires selectors nested directly inside another style rule to match a regular expression.
    ///
    /// Set to a bare regex string or `/regex/i`. Matches the whole trimmed selector list
    /// by default; set `splitList: true` to check each selector separately. Interpolated
    /// text is skipped. Does not fix violations.
    ///
    /// Example: `a { b {} }` does not match `"/^[A-Z]+$/"` because `b` is lowercase.
    "selector-nested-pattern", style;
    pub struct SelectorNestedPattern {
        regex: Option<Regex>,
        raw: String,
        split_list: bool,
    }
}
impl SelectorNestedPattern {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        let (regex, raw) =
            compile(primary).unwrap_or_else(|| (Regex::new(".*").unwrap(), primary.to_string()));
        Self {
            regex: Some(regex),
            raw,
            split_list: secondary_bool(secondary, "splitList").unwrap_or(false),
        }
    }
    fn check<'a>(&self, value: &str, span: oxc_css_parser::Span, ctx: &mut LintContext<'a>) {
        if has_interpolation(value) {
            return;
        }
        if let Some(re) = &self.regex
            && re.is_match(value)
        {
            return;
        }
        ctx.report(
            self.name(),
            span,
            format!("Expected \"{value}\" to match pattern \"{}\"", self.raw),
        );
    }
}
impl Rule for SelectorNestedPattern {
    fn name(&self) -> &'static str {
        Self::NAME
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if !matches!(ctx.ancestors.last(), Some(AstParent::QualifiedRule(_))) {
            return;
        }
        if self.split_list {
            for sel in &rule.selector.selectors {
                let raw = ctx.slice(sel.span);
                self.check(raw.trim(), sel.span, ctx);
            }
        } else {
            let raw = ctx.slice(rule.selector.span);
            self.check(raw.trim(), rule.selector.span, ctx);
        }
    }
}
#[cfg(test)]
mod tests {

    use crate::tester::{Tester, reject};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec![
            "a {}",
            "a {} b {}",
            "@media print { a {} b {} }",
            "a { B {} }",
            "a { DIV { SPAN {} } }",
        ];
        let fail = vec![
            reject(
                "a { b {} }",
                None,
                "Expected \"b\" to match pattern \"/^[A-Z]+$/\"",
                (1, 5, 1, 6),
            ),
            reject(
                "@media print { a { b {} } }",
                None,
                "Expected \"b\" to match pattern \"/^[A-Z]+$/\"",
                (1, 20, 1, 21),
            ),
        ];
        Tester::new(
            "selector-nested-pattern",
            json!(["/^[A-Z]+$/"]),
            &pass,
            &fail,
        )
        .test();
    }
    #[test]
    fn ignores_scss_interpolation() {
        Tester::new(
            "selector-nested-pattern",
            json!(["/^[A-Z]+$/"]),
            ["a { &:#{ $state } {} }"],
            [],
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
