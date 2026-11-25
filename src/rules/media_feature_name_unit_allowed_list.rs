use oxc_css_parser::{Span, ast::AtRule};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, at_rule_prelude_span, media_feature_groups},
};

crate::define_stx_lint! {
    /// Restrict the units used with configured media features in `@media` queries.
    ///
    /// The primary option is an object mapping feature names or `/regex/` keys
    /// to a unit string or an array of unit strings. The first matching key
    /// supplies the allowlist; literal names are case-sensitive, but units are
    /// compared case-insensitively. Only dimensions outside function arguments
    /// are checked; unitless values are not restricted. An empty list disallows
    /// all checked units for that feature. No rule-specific secondary options.
    ///
    /// With `{"width": "em"}`, `@media (20em <= width <= 50rem) {}` reports `50rem`.
    "media-feature-name-unit-allowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct MediaFeatureNameUnitAllowedList {
        entries: Vec<(OptionMatcher, Vec<String>)>,
    }
}

impl MediaFeatureNameUnitAllowedList {
    pub fn from_config(primary: &serde_json::Value) -> Option<Self> {
        let object = primary.as_object()?;
        let mut entries = Vec::with_capacity(object.len());
        for (key, value) in object {
            let mut units = Vec::new();
            match value {
                serde_json::Value::String(s) => units.push(s.to_ascii_lowercase()),
                serde_json::Value::Array(items) => {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            units.push(s.to_ascii_lowercase());
                        }
                    }
                }
                _ => {}
            }
            entries.push((OptionMatcher::parse(key), units));
        }
        Some(Self { entries })
    }
}

pub fn rejected_message(unit: &str, name: &str) -> String {
    format!("Disallowed unit \"{unit}\" for name \"{name}\"")
}

impl Rule for MediaFeatureNameUnitAllowedList {
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
            let mut units = None;
            for (matcher, value) in &self.entries {
                if matcher.is_match(name) {
                    units = Some(value);
                    break;
                }
            }
            let Some(units) = units else {
                continue;
            };
            for (start, end, unit_start) in group.dimensions {
                let unit = &params[unit_start..end];
                if units.iter().any(|u| u.eq_ignore_ascii_case(unit)) {
                    continue;
                }
                ctx.report(
                    self.name(),
                    Span {
                        start: prelude.start + start as u32,
                        end: prelude.start + end as u32,
                    },
                    rejected_message(unit, name),
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
            "@media (width: 50em) {}",
            "@media (width <= 50em) {}",
            "@media (min-width: 50rem) {}",
            "@media (max-width: 50rem) {}",
            "@media (30em <= width <= 50em) {}",
            "@media print and (width: 50em) {}",
            "@media (height: 50rem) {}",
            "@media (min-height: 50rem) {}",
            "@media (max-height: 50rem) {}",
            "@media (30rem <= height <= 50rem) {}",
        ];

        let fail = vec![
            reject(
                "@media (width: 50rem) {}",
                None,
                rejected_message("rem", "width"),
                (1, 16, 1, 21),
            ),
            reject(
                "@media (width <= 50rem) {}",
                None,
                rejected_message("rem", "width"),
                (1, 18, 1, 23),
            ),
            reject_warnings(
                "@media (30rem <= width <= 50rem)  {}",
                None,
                vec![
                    (rejected_message("rem", "width"), (1, 9, 1, 14)),
                    (rejected_message("rem", "width"), (1, 27, 1, 32)),
                ],
            ),
            reject(
                "@media (width: 1000px) {}",
                None,
                rejected_message("px", "width"),
                (1, 16, 1, 22),
            ),
            reject(
                "@media (height: 1000px) {}",
                None,
                rejected_message("px", "height"),
                (1, 17, 1, 23),
            ),
            reject(
                "@media (height <= 1000px) {}",
                None,
                rejected_message("px", "height"),
                (1, 19, 1, 25),
            ),
            reject_warnings(
                "@media (100px <= height <= 1000px)  {}",
                None,
                vec![
                    (rejected_message("px", "height"), (1, 9, 1, 14)),
                    (rejected_message("px", "height"), (1, 28, 1, 34)),
                ],
            ),
            reject(
                "@media (min-height: 1000px) {}",
                None,
                rejected_message("px", "min-height"),
                (1, 21, 1, 27),
            ),
            reject(
                "@media (max-height: 1000px) {}",
                None,
                rejected_message("px", "max-height"),
                (1, 21, 1, 27),
            ),
        ];

        Tester::new(
            "media-feature-name-unit-allowed-list",
            json!({"width": "em", "/height/": ["em", "rem"]}),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn unitless_values() {
        let pass = vec![
            "@media (prefers-reduced-motion) {}",
            "@media (color) {}",
            "@media (grid: 0) {}",
            "@media (video-dynamic-range: high) {}",
        ];
        Tester::new(
            "media-feature-name-unit-allowed-list",
            json!({"grid": [], "prefers-reduced-motion": [], "video-dynamic-range": []}),
            &pass,
            [],
        )
        .test();
    }
}
