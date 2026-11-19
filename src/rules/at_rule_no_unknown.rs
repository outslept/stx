use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::AtRule;

pub struct AtRuleNoUnknown;

const KNOWN_AT_RULES: &[&str] = &[
    "charset",
    "color-profile",
    "container",
    "counter-style",
    "custom-media",
    "document",
    "font-face",
    "font-feature-values",
    "font-palette-values",
    "import",
    "keyframes",
    "layer",
    "media",
    "namespace",
    "nest",
    "page",
    "position-try",
    "property",
    "scope",
    "starting-style",
    "supports",
    "view-transition",
    "viewport",
    "annotation",
    "bottom-center",
    "character-variant",
    "left-bottom",
    "left-middle",
    "left-top",
    "ornaments",
    "right-bottom",
    "right-middle",
    "right-top",
    "styleset",
    "stylistic",
    "swash",
    "top-center",
    "-webkit-keyframes",
    "-moz-keyframes",
    "-o-keyframes",
    "-ms-keyframes",
    "-moz-document",
    "-ms-viewport",
    "-o-viewport",
    "at-root",
    "content",
    "debug",
    "each",
    "else",
    "error",
    "extend",
    "for",
    "forward",
    "function",
    "if",
    "include",
    "mixin",
    "return",
    "use",
    "warn",
    "while",
    "plugin",
];

impl Rule for AtRuleNoUnknown {
    fn name(&self) -> &'static str {
        "at-rule-no-unknown"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let raw_name: &str = at_rule.name.name;
        let lower = raw_name.to_ascii_lowercase();

        if KNOWN_AT_RULES.contains(&lower.as_str()) {
            return;
        }

        ctx.report(
            self.name(),
            at_rule.span,
            format!("Unexpected unknown at-rule \"@{raw_name}\""),
        );
    }
}
