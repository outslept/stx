use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        LENGTH_UNITS, MEDIA_FEATURE_NAMES, OptionMatcher, RESOLUTION_UNITS, ValueToken,
        ValueTokenKind, at_rule_prelude_span, in_set_ignore_case, is_math_function,
        media_feature_name_allowed_value_keywords, media_feature_name_allowed_value_types,
        options_matches, parse_primary_option_matchers, unprefixed,
    },
};
use oxc_css_parser::{Span, ast::AtRule};
use serde_json::{Map, Value};

use super::media_feature_name_no_unknown::for_each_media_feature;

crate::define_stx_lint! {
    /// Disallow unsupported keywords and value types for known media features.
    ///
    /// Enable with `true`. The secondary `ignoreMediaFeatureNameValues` object
    /// maps feature names or `/regex/` keys to a keyword string or array of
    /// keyword strings and `/regex/` patterns. It matches lowercase names and
    /// keywords without vendor prefixes and strips `min-` or `max-` from names.
    /// This is not a full query validator: unknown features and unrecognized
    /// multi-token values are skipped, and `env()` is accepted as a value.
    ///
    /// Example: `@media (orientation: sideways) {}` is reported.
    "media-feature-name-value-no-unknown", correctness;
    pub struct MediaFeatureNameValueNoUnknown {
        ignore: Vec<(OptionMatcher, Vec<OptionMatcher>)>,
    }
}

impl MediaFeatureNameValueNoUnknown {
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        let ignore = secondary
            .and_then(|options| options.get("ignoreMediaFeatureNameValues"))
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .map(|(name, values)| {
                (
                    OptionMatcher::parse(name),
                    parse_primary_option_matchers(values),
                )
            })
            .collect();
        Self { ignore }
    }

    fn accepts_value(&self, name: &str, tokens: &[ValueToken<'_>]) -> bool {
        let types = media_feature_name_allowed_value_types(name).unwrap_or_default();
        let has = |ty| types.contains(&ty);
        let first = match tokens {
            [token] => token,
            [left, slash, right]
                if slash.kind == ValueTokenKind::Slash
                    && is_ratio_part(left)
                    && is_ratio_part(right) =>
            {
                return has("ratio");
            }
            _ => return true,
        };
        if let Some(function) = function_name(first) {
            return function.eq_ignore_ascii_case("env")
                || (is_math_function(function)
                    && ["integer", "length", "ratio", "resolution"]
                        .iter()
                        .any(|ty| has(ty)));
        }
        match first.kind {
            ValueTokenKind::Ident if !first.raw.starts_with(['$', '@']) => {
                let lower = first.raw.to_ascii_lowercase();
                let keyword = unprefixed(&lower);
                self.ignore
                    .iter()
                    .any(|(key, values)| key.is_match(name) && options_matches(values, keyword))
                    || media_feature_name_allowed_value_keywords(name)
                        .is_some_and(|keywords| keywords.contains(&keyword))
            }
            ValueTokenKind::Number => {
                let integer = !first.raw.contains(['.', 'e', 'E']);
                let number = first.raw.parse::<f64>().ok();
                has("ratio")
                    || (integer && has("integer"))
                    || (integer && has("mq-boolean") && matches!(number, Some(0.0 | 1.0)))
                    || (number == Some(0.0) && (has("length") || has("resolution")))
            }
            ValueTokenKind::Dimension { num_len } => {
                let unit = &first.raw[num_len..];
                (has("length") && in_set_ignore_case(LENGTH_UNITS, unit))
                    || (has("resolution") && in_set_ignore_case(RESOLUTION_UNITS, unit))
            }
            _ => true,
        }
    }
}

fn function_name<'a>(token: &ValueToken<'a>) -> Option<&'a str> {
    (token.kind == ValueTokenKind::Function).then(|| &token.raw[..token.raw.len() - 1])
}

fn is_ratio_part(token: &ValueToken<'_>) -> bool {
    token.kind == ValueTokenKind::Number
        || function_name(token)
            .is_some_and(|name| name.eq_ignore_ascii_case("env") || is_math_function(name))
}

impl Rule for MediaFeatureNameValueNoUnknown {
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
        for_each_media_feature(ctx.source, prelude, |name, values| {
            let lower = name.raw.to_ascii_lowercase();
            let normalized = unprefixed(&lower);
            if !MEDIA_FEATURE_NAMES.contains(&normalized) {
                return;
            }
            let base = normalized
                .strip_prefix("min-")
                .or_else(|| normalized.strip_prefix("max-"))
                .unwrap_or(normalized);
            for &tokens in values {
                if self.accepts_value(base, tokens) {
                    continue;
                }
                let (Some(first), Some(last)) = (tokens.first(), tokens.last()) else {
                    continue;
                };
                let span = Span {
                    start: first.span.start,
                    end: last.span.end,
                };
                let value = ctx.slice(span);
                let name = name.raw;
                ctx.report(
                    self.name(),
                    span,
                    format!("Unknown media feature value \"{value}\" for name \"{name}\""),
                );
            }
        });
    }
}
#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "@media (min-width: 10px) {}",
            "@media (prefers-color-scheme: dark) {}",
            "@media (orientation: landscape) {}",
            "@media (width: 0) {}",
            "@media (-moz-min-resolution: 2dpcm) and (-webkit-hover: -webkit-hover) {}",
            "@media (unknown: 100px) and (width: $sm) and (hover: #foo) and (hover: 10%) {}",
            "@media (aspect-ratio: 1 / calc(2)) and (width: max(1px, 2px)) and (hover: env(x)) {}",
        ];
        let fail = vec![
            reject(
                "@media (color: 1 / 1) {}",
                None,
                "Unknown media feature value \"1 / 1\" for name \"color\"",
                (1, 16, 1, 21),
            ),
            reject(
                "@media (hover: sin(90deg)) {}",
                None,
                "Unknown media feature value \"sin(90deg)\" for name \"hover\"",
                (1, 16, 1, 26),
            ),
            reject(
                "@media (1khz < width <= 2px) {}",
                None,
                "Unknown media feature value \"1khz\" for name \"width\"",
                (1, 9, 1, 13),
            ),
            reject_message(
                "@media (min-width: 10foo) {}",
                None,
                "Unknown media feature value \"10foo\" for name \"min-width\"",
            ),
            reject_message(
                "@media (prefers-color-scheme: foo) {}",
                None,
                "Unknown media feature value \"foo\" for name \"prefers-color-scheme\"",
            ),
            reject_message(
                "@media (orientation: sideways) {}",
                None,
                "Unknown media feature value \"sideways\" for name \"orientation\"",
            ),
        ];
        Tester::new(
            "media-feature-name-value-no-unknown",
            json!(true),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn value_ignores_match_normalized_features_and_keywords() {
        Tester::new(
            "media-feature-name-value-no-unknown",
            json!([true, {"ignoreMediaFeatureNameValues": {
                "color": ["--foo"],
                "/^color/": ["/^--bar/"]
            }}]),
            ["@media (-moz-min-color: --FOO) and (COLOR-GAMUT: --BAR-baz) {}"],
            [reject(
                "@media (color: --baz) {}",
                None,
                "Unknown media feature value \"--baz\" for name \"color\"",
                (1, 16, 1, 21),
            )],
        )
        .test();
    }
}
