use std::sync::OnceLock;

use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{OptionMatcher, at_rule_prelude_span, options_matches, parse_option_matchers},
};
use oxc_css_parser::{Span, ast::AtRule};
use regex::Regex;
use serde_json::{Map, Value};

pub struct MediaFeatureNameNoVendorPrefix {
    ignore_media_feature_names: Vec<OptionMatcher>,
}

impl MediaFeatureNameNoVendorPrefix {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_media_feature_names: parse_option_matchers(secondary, "ignoreMediaFeatureNames"),
        }
    }
}

fn features() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(concat!(
            "(?i)-webkit-device-pixel-ratio|-webkit-min-device-pixel-ratio",
            "|-webkit-max-device-pixel-ratio|-o-device-pixel-ratio|-o-min-device-pixel-ratio",
            "|-o-max-device-pixel-ratio|-moz-device-pixel-ratio|min--moz-device-pixel-ratio",
            "|max--moz-device-pixel-ratio",
        ))
        .unwrap()
    })
}

fn prefix() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("(?i)-moz-|-o-|-webkit-").unwrap())
}

impl Rule for MediaFeatureNameNoVendorPrefix {
    fn name(&self) -> &'static str {
        "media-feature-name-no-vendor-prefix"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("media") {
            return;
        }
        let Some(params_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let params = ctx.slice(params_span);
        if !params.to_ascii_lowercase().contains("device-pixel-ratio") {
            return;
        }
        let node = ctx.slice_range(at_rule.span.start, at_rule.span.end);
        for found in features().find_iter(params) {
            let matched = found.as_str();
            if options_matches(&self.ignore_media_feature_names, matched) {
                continue;
            }
            // Like upstream, both the report and the fix use the first occurrence of the match.
            let first = |haystack: &str, base: u32| {
                haystack.find(matched).map(|i| Span {
                    start: base + i as u32,
                    end: base + (i + matched.len()) as u32,
                })
            };
            let span = first(node, at_rule.span.start).unwrap_or(at_rule.span);
            let fix = first(params, params_span.start)
                .map(|span| Fix::single(span, prefix().replace(matched, "").into_owned()));
            ctx.report_with_fix(self.name(), span, "Vendor-prefixed media feature name", fix);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn reports_autoprefixer_media_features() {
        let msg = "Vendor-prefixed media feature name";
        let fail = [
            reject(
                "@media (min--moz-device-pixel-ratio: 1) {}",
                "@media (min-device-pixel-ratio: 1) {}",
                msg,
                (1, 9, 1, 36),
            ),
            reject(
                "@media (/* a comment */MIN--moz-device-pixel-ratio: 1) {}",
                "@media (/* a comment */MIN-device-pixel-ratio: 1) {}",
                msg,
                (1, 24, 1, 51),
            ),
            reject(
                "@media (-webkit-min-device-pixel-ratio: 1) {}",
                "@media (min-device-pixel-ratio: 1) {}",
                msg,
                (1, 9, 1, 39),
            ),
        ];
        Tester::new("media-feature-name-no-vendor-prefix", json!(true), [], fail)
            .with_fix(true)
            .test();
        Tester::new(
            "media-feature-name-no-vendor-prefix",
            json!([true, {"ignoreMediaFeatureNames": ["-webkit-min-device-pixel-ratio", "/^-o/"]}]),
            [
                "@media (-webkit-min-device-pixel-ratio: 1) {}",
                "@media (-o-device-pixel-ratio > 1) {}",
            ],
            [],
        )
        .test();
    }
}
