use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        value_utils::{ValueTokenKind, at_rule_prelude_span, tokenize_value},
    },
};
use oxc_css_parser::ast::AtRule;

pub struct MediaFeatureNameNoVendorPrefix;

const PREFIXED_MEDIA_FEATURES: &[&str] = &[
    "-webkit-min-device-pixel-ratio",
    "-webkit-max-device-pixel-ratio",
    "-webkit-device-pixel-ratio",
    "-moz-min-device-pixel-ratio",
    "-moz-max-device-pixel-ratio",
    "-moz-device-pixel-ratio",
    "-o-min-device-pixel-ratio",
    "-o-max-device-pixel-ratio",
    "-o-device-pixel-ratio",
    "-webkit-transform-3d",
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

impl Rule for MediaFeatureNameNoVendorPrefix {
    fn name(&self) -> &'static str {
        "media-feature-name-no-vendor-prefix"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if !name.eq_ignore_ascii_case("media") {
            return;
        }
        let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };

        let tokens = tokenize_value(ctx.source, p_span);
        for tok in tokens {
            if tok.kind == ValueTokenKind::Ident
                && PREFIXED_MEDIA_FEATURES
                    .iter()
                    .any(|f| f.eq_ignore_ascii_case(tok.raw))
            {
                if let Some(unprefixed) = strip_prefix(tok.raw) {
                    ctx.report_with_fix(
                        self.name(),
                        tok.span,
                        "Unexpected vendor-prefix".to_string(),
                        Some(Fix::single(tok.span, unprefixed.to_string())),
                    );
                }
            }
        }
    }
}
