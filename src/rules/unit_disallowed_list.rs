use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, at_rule_prelude_span, decl_value_span, ident_name,
        is_unicode_range_descriptor, media_feature_groups, options_matches, parse_option_matchers,
        tokenize_value,
    },
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, Declaration},
};

fn strings_lower(v: &serde_json::Value) -> Vec<String> {
    match v {
        serde_json::Value::String(s) => vec![s.to_ascii_lowercase()],
        serde_json::Value::Array(a) => a
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.to_ascii_lowercase()))
            .collect(),
        _ => vec![],
    }
}
fn parse_ignore_map(
    secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Vec<(String, Vec<OptionMatcher>)> {
    let Some(obj) = secondary
        .and_then(|m| m.get(key))
        .and_then(|v| v.as_object())
    else {
        return vec![];
    };
    let mut out = Vec::new();
    for (unit, val) in obj {
        let mut ms = Vec::new();
        match val {
            serde_json::Value::String(s) => ms.push(OptionMatcher::parse(s)),
            serde_json::Value::Array(items) => {
                for it in items {
                    if let Some(s) = it.as_str() {
                        ms.push(OptionMatcher::parse(s));
                    }
                }
            }
            _ => {}
        }
        out.push((unit.to_ascii_lowercase(), ms));
    }
    out
}
pub struct UnitDisallowedList {
    disallowed: Vec<String>,
    ignore_functions: Vec<OptionMatcher>,
    ignore_properties: Vec<(String, Vec<OptionMatcher>)>,
    ignore_media: Vec<(String, Vec<OptionMatcher>)>,
}
impl UnitDisallowedList {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        Self {
            disallowed: strings_lower(primary),
            ignore_functions: parse_option_matchers(secondary, "ignoreFunctions"),
            ignore_properties: parse_ignore_map(secondary, "ignoreProperties"),
            ignore_media: parse_ignore_map(secondary, "ignoreMediaFeatureNames"),
        }
    }
    fn is_disallowed(&self, unit: &str) -> bool {
        self.disallowed.iter().any(|u| u.eq_ignore_ascii_case(unit))
    }
    fn property_ignores(&self, unit: &str, prop: &str) -> bool {
        let lower = unit.to_ascii_lowercase();
        self.ignore_properties
            .iter()
            .filter(|(u, _)| *u == lower)
            .any(|(_, ms)| options_matches(ms, prop))
    }
    fn media_ignores(&self, unit: &str, name: &str) -> bool {
        let lower = unit.to_ascii_lowercase();
        self.ignore_media
            .iter()
            .filter(|(u, _)| *u == lower)
            .any(|(_, ms)| options_matches(ms, name))
    }
    fn check_tokens<'a>(
        &self,
        tokens: &[crate::utils::ValueToken<'a>],
        prop: &str,
        ctx: &mut LintContext<'a>,
    ) {
        let mut depth = 0usize;
        let mut skip_from: Option<usize> = None;
        for token in tokens {
            match token.kind {
                ValueTokenKind::Function => {
                    depth += 1;
                    if skip_from.is_none() {
                        let name = token
                            .raw
                            .strip_suffix('(')
                            .unwrap_or(token.raw)
                            .to_ascii_lowercase();
                        if name == "url" || options_matches(&self.ignore_functions, &name) {
                            skip_from = Some(depth);
                        }
                    }
                }
                ValueTokenKind::CloseParen => {
                    depth = depth.saturating_sub(1);
                    if skip_from.is_some_and(|f| depth < f) {
                        skip_from = None;
                    }
                }
                _ if skip_from.is_some() => {}
                ValueTokenKind::Dimension { num_len } => {
                    let unit = &token.raw[num_len..];
                    if !self.is_disallowed(unit) || self.property_ignores(unit, prop) {
                        continue;
                    }
                    ctx.report(
                        self.name(),
                        Span {
                            start: token.span.start + num_len as u32,
                            end: token.span.end,
                        },
                        format!("Disallowed unit \"{unit}\""),
                    );
                }
                ValueTokenKind::Percentage
                    if self.is_disallowed("%") && !self.property_ignores("%", prop) =>
                {
                    ctx.report(
                        self.name(),
                        Span {
                            start: token.span.end - 1,
                            end: token.span.end,
                        },
                        "Disallowed unit \"%\"".to_string(),
                    );
                }
                _ => {}
            }
        }
    }
}
impl Rule for UnitDisallowedList {
    fn name(&self) -> &'static str {
        "unit-disallowed-list"
    }
    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if is_unicode_range_descriptor(prop, &ctx.ancestors) {
            return;
        }
        let Some(sp) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let tokens = tokenize_value(ctx.source, sp);
        self.check_tokens(&tokens, prop, ctx);
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
            let name = params[group.name_start..group.name_end].to_ascii_lowercase();
            let mut dimensions = group.dimensions.into_iter().peekable();
            for (start, end) in group.values {
                let unit_start = if dimensions.peek().is_some_and(|d| d.0 == start) {
                    dimensions.next().unwrap().2
                } else if params.as_bytes()[end - 1] == b'%' {
                    end - 1
                } else {
                    continue;
                };
                let unit = &params[unit_start..end];
                if !self.is_disallowed(unit) || self.media_ignores(unit, &name) {
                    continue;
                }
                ctx.report(
                    self.name(),
                    Span {
                        start: sp.start + unit_start as u32,
                        end: sp.start + end as u32,
                    },
                    format!("Disallowed unit \"{unit}\""),
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {

    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;
    #[test]
    fn unit_spans() {
        let fail = vec![
            reject(
                "a { font-size: 80%; }",
                None,
                "Disallowed unit \"%\"",
                (1, 18, 1, 19),
            ),
            reject(
                "@media (min-width: 10px) {}",
                None,
                "Disallowed unit \"px\"",
                (1, 22, 1, 24),
            ),
        ];
        Tester::new("unit-disallowed-list", json!(["%", "px"]), [], &fail).test();
    }
    #[test]
    fn default_config() {
        let pass = vec![
            "a { margin: 1em; }",
            "a { margin: 0; }",
            "a { color: #000; }",
            "a { transform: rotate(45deg); }",
        ];
        let fail = vec![
            reject_message("a { margin: 10px; }", None, "Disallowed unit \"px\""),
            reject_message("a { margin: 10vmin; }", None, "Disallowed unit \"vmin\""),
            reject_message("a { margin: 1f; }", None, "Disallowed unit \"f\""),
        ];
        Tester::new(
            "unit-disallowed-list",
            json!(["px", "vmin", "f"]),
            &pass,
            &fail,
        )
        .test();
    }
    #[test]
    fn unicode_range_is_ignored_only_as_a_font_face_descriptor() {
        Tester::new(
            "unit-disallowed-list",
            json!(["f"]),
            ["@font-face { unicode-range: U+0100-024F; }"],
            [reject(
                "a { unicode-range: U+0100-024F; }",
                None,
                "Disallowed unit \"F\"",
                (1, 30, 1, 31),
            )],
        )
        .test();
    }

    #[test]
    fn media_percentages_and_case_insensitive_feature_names() {
        Tester::new(
            "unit-disallowed-list",
            json!([["px", "%"], {"ignoreMediaFeatureNames": {"px": ["min-width"], "%": ["width"]}}]),
            ["@media (MIN-WIDTH: 960px), (WIDTH > 20%) {}"],
            [reject(
                "@media only speech and (max-device-width > 20%) { }",
                None,
                "Disallowed unit \"%\"",
                (1, 46, 1, 47),
            )],
        )
        .test();
    }

    #[test]
    fn with_ignore_properties() {
        let fail = vec![reject(
            "a { margin: 10px; }",
            None,
            "Disallowed unit \"px\"",
            (1, 15, 1, 17),
        )];
        Tester::new(
            "unit-disallowed-list",
            json!(["px", {"ignoreProperties": {"px": ["padding"]}}]),
            ["a { padding: 10px; }"],
            &fail,
        )
        .test();
    }
}
