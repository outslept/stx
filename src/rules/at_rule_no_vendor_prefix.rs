use crate::{context::LintContext, fix::Fix, rules::Rule};
use oxc_css_parser::{Span, ast::AtRule};

pub struct AtRuleNoVendorPrefix;

const PREFIXED_AT_RULES: &[&str] = &[
    "-webkit-keyframes",
    "-moz-keyframes",
    "-o-keyframes",
    "-ms-keyframes",
    "-webkit-viewport",
    "-moz-viewport",
    "-ms-viewport",
];

fn strip_prefix(s: &str) -> Option<&str> {
    let lower = s.to_ascii_lowercase();
    for p in ["-webkit-", "-moz-", "-ms-", "-o-"] {
        if lower.starts_with(p) {
            return Some(&s[p.len()..]);
        }
    }
    None
}

impl Rule for AtRuleNoVendorPrefix {
    fn name(&self) -> &'static str {
        "at-rule-no-vendor-prefix"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if PREFIXED_AT_RULES
            .iter()
            .any(|r| r.eq_ignore_ascii_case(name))
        {
            if let Some(unprefixed) = strip_prefix(name) {
                let report_span = Span {
                    start: at_rule.span.start,
                    end: at_rule.name.span.end,
                };
                ctx.report_with_fix(
                    self.name(),
                    report_span,
                    format!("Unexpected vendor-prefixed at-rule \"@{}\"", name),
                    Some(Fix::single(at_rule.name.span, unprefixed.to_string())),
                );
            }
        }
    }
}
