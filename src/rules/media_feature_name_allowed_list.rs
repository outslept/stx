use oxc_css_parser::{Span, ast::AtRule};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, at_rule_prelude_span, media_feature_names, options_matches,
        parse_primary_option_matchers,
    },
};

crate::define_stx_lint! {
    /// Allow only configured media feature names in `@media` queries.
    ///
    /// Supply a string or array of strings as the primary option. Entries are
    /// case-sensitive literal names or `/regex/` patterns. Names are checked
    /// as written, including `min-`, `max-`, and vendor prefixes. Custom media
    /// names beginning with `--` are skipped. No rule-specific secondary options.
    ///
    /// With `["max-width"]`, `@media (max-width: 50em) {}` passes and
    /// `@media (min-width: 50em) {}` is reported.
    "media-feature-name-allowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct MediaFeatureNameAllowedList {
        allowed: Vec<OptionMatcher>,
    }
}

impl MediaFeatureNameAllowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            allowed: parse_primary_option_matchers(primary),
        }
    }
}

pub fn rejected_message(name: &str) -> String {
    format!("Disallowed media feature name \"{name}\"")
}

impl Rule for MediaFeatureNameAllowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("media") {
            return;
        }
        let Some(prelude) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let params = ctx.slice(prelude);
        for (start, end) in media_feature_names(params) {
            let name = &params[start..end];
            if options_matches(&self.allowed, name) {
                continue;
            }
            ctx.report(
                self.name(),
                Span {
                    start: prelude.start + start as u32,
                    end: prelude.start + end as u32,
                },
                rejected_message(name),
            );
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
            "@media (max-width: 50em) { }",
            "@media (--wide-viewport) { }",
            "@media (/* min-width: 50em */ max-width: 50em) { }",
            "@media (max-width <= 50em) { }",
            "@media (400px < my-width < 1000px) { }",
            "@media (my-width: 50em) { }",
            "@media (my-max-width: 50em) { }",
            "@media print and (max-width: 50em) { }",
            "@media (color) { }",
            "@media only screen and(min-width: 640px) { }",
        ];

        let fail = vec![
            reject(
                "@media (MaX-wIdTh: 50em) { }",
                None,
                rejected_message("MaX-wIdTh"),
                (1, 9, 1, 18),
            ),
            reject(
                "@media (min-width: 50em) { }",
                None,
                rejected_message("min-width"),
                (1, 9, 1, 18),
            ),
            reject(
                "@media screen and (/* a comment */min-width: 50em) { }",
                None,
                rejected_message("min-width"),
                (1, 35, 1, 44),
            ),
            reject(
                "@media screen and /* a comment */(min-width: 50em) { }",
                None,
                rejected_message("min-width"),
                (1, 35, 1, 44),
            ),
            reject(
                "@media (-webkit-min-device-pixel-ratio: 2) { }",
                None,
                rejected_message("-webkit-min-device-pixel-ratio"),
                (1, 9, 1, 39),
            ),
            reject(
                "@media handheld and (max-width: 20em), screen and (min-width: 20em) { }",
                None,
                rejected_message("min-width"),
                (1, 52, 1, 61),
            ),
            reject(
                "@media (monochrome) { }",
                None,
                rejected_message("monochrome"),
                (1, 9, 1, 19),
            ),
            reject(
                "@media (width: 50em) { }",
                None,
                rejected_message("width"),
                (1, 9, 1, 14),
            ),
            reject(
                "@media (50em < width) { }",
                None,
                rejected_message("width"),
                (1, 16, 1, 21),
            ),
            reject(
                "@media (10em < width <= 50em) { }",
                None,
                rejected_message("width"),
                (1, 16, 1, 21),
            ),
            reject(
                "@media (max-width <= 50em) and (10em < min-width < 50em) { }",
                None,
                rejected_message("min-width"),
                (1, 40, 1, 49),
            ),
        ];

        Tester::new(
            "media-feature-name-allowed-list",
            json!(["max-width", "/^my-/", "color"]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn regex_config() {
        let pass = vec![
            "@media (my-width: 50em) { }",
            "@media (my-max-width: 50em) { }",
            "@media (my-width >= 50em) { }",
            "@media (10em < my-max-width <= 50em) { }",
        ];
        let fail = vec![reject(
            "@media (MaX-wIdTh: 50em) { }",
            None,
            rejected_message("MaX-wIdTh"),
            (1, 9, 1, 18),
        )];
        Tester::new(
            "media-feature-name-allowed-list",
            json!("/^my-/"),
            &pass,
            &fail,
        )
        .test();
    }
}
