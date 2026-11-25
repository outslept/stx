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
    /// Disallow configured media feature names in `@media` queries.
    ///
    /// Supply a string or array of strings as the primary option. Entries are
    /// case-sensitive literal names or `/regex/` patterns. Names are checked
    /// as written, including `min-`, `max-`, and vendor prefixes. Custom media
    /// names beginning with `--` are skipped. No rule-specific secondary options.
    ///
    /// With `["max-width"]`, `@media (max-width: 50em) {}` is reported and
    /// `@media (min-width: 50em) {}` passes.
    "media-feature-name-disallowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct MediaFeatureNameDisallowedList {
        disallowed: Vec<OptionMatcher>,
    }
}

impl MediaFeatureNameDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            disallowed: parse_primary_option_matchers(primary),
        }
    }
}

pub fn rejected_message(name: &str) -> String {
    format!("Disallowed media feature name \"{name}\"")
}

impl Rule for MediaFeatureNameDisallowedList {
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
            if !options_matches(&self.disallowed, name) {
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
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "@media (min-width: 50em) { }",
            "@media (MaX-wIdTh: 50em) { }",
            "@media (MiN-wIdTh: 50em) { }",
            "@media (height <= 50em) { }",
            "@media (400px < height < 1000px) { }",
            "@media (--wide-viewport) { }",
            "@media (/* max-width: 50em */ min-width: 50em) { }",
            "@media (monochrome) { }",
        ];

        let fail = vec![
            reject(
                "@media (max-width: 50em) { }",
                None,
                rejected_message("max-width"),
                (1, 9, 1, 18),
            ),
            reject(
                "@media print and (max-width: 50em) { }",
                None,
                rejected_message("max-width"),
                (1, 19, 1, 28),
            ),
            reject(
                "@media print and (/* a comment */max-width: 50em) { }",
                None,
                rejected_message("max-width"),
                (1, 34, 1, 43),
            ),
            reject(
                "@media print and /* a comment */(max-width: 50em) { }",
                None,
                rejected_message("max-width"),
                (1, 34, 1, 43),
            ),
            reject(
                "@media handheld and (min-width: 20em), screen and (max-width: 20em) { }",
                None,
                rejected_message("max-width"),
                (1, 52, 1, 61),
            ),
            reject(
                "@media (my-width: 50em) { }",
                None,
                rejected_message("my-width"),
                (1, 9, 1, 17),
            ),
            reject(
                "@media (color) { }",
                None,
                rejected_message("color"),
                (1, 9, 1, 14),
            ),
            reject(
                "@media (width: 50em) { }",
                None,
                rejected_message("width"),
                (1, 9, 1, 14),
            ),
            reject(
                "@media (20em < width <= 50em) { }",
                None,
                rejected_message("width"),
                (1, 16, 1, 21),
            ),
            reject_warnings(
                "@media (10em < max-width <= 50em) and (width > 50em) { }",
                None,
                vec![
                    (rejected_message("max-width"), (1, 16, 1, 25)),
                    (rejected_message("width"), (1, 40, 1, 45)),
                ],
            ),
        ];

        Tester::new(
            "media-feature-name-disallowed-list",
            json!(["max-width", "--wide-viewport", "width", "/^my-/", "color"]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn regex_config() {
        let pass = vec!["@media (min-width: 50em) { }"];
        let fail = vec![
            reject(
                "@media (my-width: 50em) { }",
                None,
                rejected_message("my-width"),
                (1, 9, 1, 17),
            ),
            reject(
                "@media (my-width >= 50em) { }",
                None,
                rejected_message("my-width"),
                (1, 9, 1, 17),
            ),
            reject(
                "@media (10em < my-width <= 50em) { }",
                None,
                rejected_message("my-width"),
                (1, 16, 1, 24),
            ),
            reject(
                "@media (50em < my-width) { }",
                None,
                rejected_message("my-width"),
                (1, 16, 1, 24),
            ),
        ];
        Tester::new(
            "media-feature-name-disallowed-list",
            json!("/^my-/"),
            &pass,
            &fail,
        )
        .test();
    }
}
