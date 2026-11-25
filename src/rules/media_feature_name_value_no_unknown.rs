use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        RESOLUTION_UNITS, at_rule_prelude_span, media_feature_groups,
        media_feature_name_allowed_value_keywords, media_feature_name_allowed_value_types,
    },
};
use oxc_css_parser::ast::AtRule;
fn is_length_unit(u: &str) -> bool {
    matches!(
        u,
        "px" | "em"
            | "rem"
            | "ex"
            | "ch"
            | "vh"
            | "vw"
            | "vmin"
            | "vmax"
            | "vi"
            | "vb"
            | "cm"
            | "mm"
            | "in"
            | "pt"
            | "pc"
            | "q"
            | "cap"
            | "ic"
            | "lh"
            | "rlh"
            | "svh"
            | "svw"
            | "lvh"
            | "lvw"
            | "dvh"
            | "dvw"
            | "cqi"
            | "cqb"
            | "cqw"
            | "cqh"
            | "cqmin"
            | "cqmax"
            | "svi"
            | "svb"
            | "lvi"
            | "lvb"
            | "dvi"
            | "dvb"
    )
}
pub struct MediaFeatureNameValueNoUnknown;
impl Rule for MediaFeatureNameValueNoUnknown {
    fn name(&self) -> &'static str {
        "media-feature-name-value-no-unknown"
    }
    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("media") {
            return;
        }
        let Some(sp) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let params = ctx.slice(sp);
        for group in media_feature_groups(params) {
            let name = &params[group.name_start..group.name_end];
            let lower = name.to_ascii_lowercase();
            let base = lower
                .strip_prefix("min-")
                .or_else(|| lower.strip_prefix("max-"))
                .unwrap_or(&lower);
            let keywords = media_feature_name_allowed_value_keywords(base);
            let types = media_feature_name_allowed_value_types(base);
            for (start, end) in group.values {
                let value = &params[start..end];
                let is_ident = value
                    .as_bytes()
                    .first()
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'-');
                let ok = if is_ident {
                    keywords.is_some_and(|ks| ks.iter().any(|k| k.eq_ignore_ascii_case(value)))
                } else if let Some(last) = value.as_bytes().last() {
                    if *last == b'%' {
                        types.is_some_and(|ts| ts.contains(&"percentage"))
                    } else {
                        let num_end = value
                            .char_indices()
                            .find(|(_, c)| {
                                !(c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
                            })
                            .map(|(i, _)| i)
                            .unwrap_or(value.len());
                        let number = &value[..num_end];
                        let unit = &value[num_end..];
                        if unit.is_empty() {
                            let is_int = number.parse::<i64>().is_ok();
                            types.is_some_and(|ts| {
                                ts.contains(&"ratio")
                                    || (is_int && ts.contains(&"integer"))
                                    || ((number == "0" || number == "1")
                                        && ts.contains(&"mq-boolean"))
                                    || (number == "0"
                                        && (ts.contains(&"length") || ts.contains(&"resolution")))
                            })
                        } else {
                            let u = unit.to_ascii_lowercase();
                            types.is_some_and(|ts| {
                                (ts.contains(&"length") && is_length_unit(&u))
                                    || (ts.contains(&"resolution")
                                        && RESOLUTION_UNITS.contains(&u.as_str()))
                            })
                        }
                    }
                } else {
                    true
                };
                if !ok {
                    ctx.report(
                        self.name(),
                        oxc_css_parser::Span {
                            start: sp.start + start as u32,
                            end: sp.start + end as u32,
                        },
                        format!("Unknown media feature value \"{value}\" for name \"{name}\""),
                    );
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject_message};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec![
            "@media (min-width: 10px) {}",
            "@media (prefers-color-scheme: dark) {}",
            "@media (orientation: landscape) {}",
            "@media (width: 0) {}",
        ];
        let fail = vec![
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
}
