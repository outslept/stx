use oxc_css_parser::{Span, ast::AtRule};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, at_rule_prelude_span, media_feature_groups, options_matches},
};

crate::define_stx_lint! {
    /// Restrict the values used with configured media features in `@media` queries.
    ///
    /// The primary option is an object mapping feature names or `/regex/` keys
    /// to a value string or an array of value strings. Values can also be
    /// `/regex/` patterns. The first matching key wins, and literal matches
    /// are case-sensitive. The rule checks identifiers, numbers, percentages,
    /// and dimensions outside function arguments. Boolean features have no
    /// value to check, even with an empty allowlist. No rule-specific secondary options.
    ///
    /// With `{"min-width": "768px"}`, `@media (min-width: 1000px) {}` is reported.
    "media-feature-name-value-allowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct MediaFeatureNameValueAllowedList {
        entries: Vec<(OptionMatcher, Vec<OptionMatcher>)>,
    }
}

impl MediaFeatureNameValueAllowedList {
    pub fn from_config(primary: &serde_json::Value) -> Option<Self> {
        let object = primary.as_object()?;
        let mut entries = Vec::with_capacity(object.len());
        for (key, value) in object {
            let mut allowed = Vec::new();
            match value {
                serde_json::Value::String(s) => allowed.push(OptionMatcher::parse(s)),
                serde_json::Value::Array(items) => {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            allowed.push(OptionMatcher::parse(s));
                        }
                    }
                }
                _ => {}
            }
            entries.push((OptionMatcher::parse(key), allowed));
        }
        Some(Self { entries })
    }
}

pub fn rejected_message(name: &str, value: &str) -> String {
    format!("Disallowed value \"{value}\" for name \"{name}\"")
}

impl Rule for MediaFeatureNameValueAllowedList {
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
        for group in media_feature_groups(params) {
            let name = &params[group.name_start..group.name_end];
            let mut allowed = None;
            for (matcher, value) in &self.entries {
                if matcher.is_match(name) {
                    allowed = Some(value);
                    break;
                }
            }
            let Some(allowed) = allowed else {
                continue;
            };
            for (start, end) in group.values {
                let value = &params[start..end];
                if options_matches(allowed, value) {
                    continue;
                }
                ctx.report(
                    self.name(),
                    Span {
                        start: prelude.start + start as u32,
                        end: prelude.start + end as u32,
                    },
                    rejected_message(name, value),
                );
            }
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
            "@media screen and (min-width: 768px) {}",
            "@media screen and  (   min-width  :   768px ) {}",
            "@media screen and (max-width: 1000px) {}",
            "@media screen and ( min-resolution :  2dpcm ) {}",
            "@media screen and (resolution: 10.1dpcm) {}",
            "@media screen and (-webkit-resolution: 10.1dpcm) {}",
            "@media (color) {}",
            "@media (update) {}",
            "@media (update /* pw:need */) {}",
            "@media screen and (min-width <= 768px) {}",
        ];

        let fail = vec![
            reject(
                "@media screen and (min-width: 1000px) {}",
                None,
                rejected_message("min-width", "1000px"),
                (1, 31, 1, 37),
            ),
            reject(
                "@media screen and (min-width: 768px) and (min-width: 1000px) {}",
                None,
                rejected_message("min-width", "1000px"),
                (1, 54, 1, 60),
            ),
            reject(
                "@media screen and (min-width: 768px)\nand (min-width: 1000px) {}",
                None,
                rejected_message("min-width", "1000px"),
                (2, 17, 2, 23),
            ),
            reject(
                "@media screen and (min-width: 768PX) {}",
                None,
                rejected_message("min-width", "768PX"),
                (1, 31, 1, 36),
            ),
            reject(
                "@media screen and (min-resolution:  2dpi) {}",
                None,
                rejected_message("min-resolution", "2dpi"),
                (1, 37, 1, 41),
            ),
            reject(
                "@media screen and (-webkit-min-resolution:  2dpi) {}",
                None,
                rejected_message("-webkit-min-resolution", "2dpi"),
                (1, 45, 1, 49),
            ),
            reject(
                "@media screen and (min-width > 500px) {}",
                None,
                rejected_message("min-width", "500px"),
                (1, 32, 1, 37),
            ),
            reject(
                "@media screen and (400px < min-width) {}",
                None,
                rejected_message("min-width", "400px"),
                (1, 20, 1, 25),
            ),
            reject_warnings(
                "@media (400px < min-width < 500px) and (min-width < 1200px) {}",
                None,
                vec![
                    (rejected_message("min-width", "400px"), (1, 9, 1, 14)),
                    (rejected_message("min-width", "500px"), (1, 29, 1, 34)),
                    (rejected_message("min-width", "1200px"), (1, 53, 1, 59)),
                ],
            ),
        ];

        Tester::new(
            "media-feature-name-value-allowed-list",
            json!([{
                "min-width": ["768px"],
                "/^(-webkit-)?min-resolution$/": "/dpcm$/",
                "color": [],
                "width": [],
            }]),
            &pass,
            &fail,
        )
        .test();
    }
}
