use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, at_rule_prelude_span, decl_value_span, ident_name,
        options_matches, parse_option_matchers, tokenize_value,
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
pub struct UnitAllowedList {
    allowed: Vec<String>,
    ignore_functions: Vec<OptionMatcher>,
    ignore_properties: Vec<(String, Vec<OptionMatcher>)>,
}
impl UnitAllowedList {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        Self {
            allowed: strings_lower(primary),
            ignore_functions: parse_option_matchers(secondary, "ignoreFunctions"),
            ignore_properties: parse_ignore_map(secondary, "ignoreProperties"),
        }
    }
    fn property_ignores(&self, unit: &str, prop: &str) -> bool {
        let lower = unit.to_ascii_lowercase();
        self.ignore_properties
            .iter()
            .filter(|(u, _)| *u == lower)
            .any(|(_, ms)| options_matches(ms, prop))
    }
    fn check_tokens<'a>(
        &self,
        tokens: &[crate::utils::ValueToken<'a>],
        prop: Option<&str>,
        ctx: &mut LintContext<'a>,
    ) {
        let mut depth = 0usize;
        let mut skip_from: Option<usize> = None;
        for (i, token) in tokens.iter().enumerate() {
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
                    // postcss-value-parser words run on into adjacent closing braces, e.g. `2rem}`.
                    let mut span = token.span;
                    for next in &tokens[i + 1..] {
                        if next.span.start != span.end || !matches!(next.raw, "}" | "]") {
                            break;
                        }
                        span.end = next.span.end;
                    }
                    self.check_unit(&token.raw[num_len..], span, num_len, prop, ctx);
                }
                ValueTokenKind::Percentage => self.check_percent(token.span, prop, ctx),
                _ => {}
            }
        }
    }
    fn check_unit<'a>(
        &self,
        unit: &str,
        span: Span,
        num_len: usize,
        prop: Option<&str>,
        ctx: &mut LintContext<'a>,
    ) {
        if self.allowed.iter().any(|u| u.eq_ignore_ascii_case(unit)) {
            return;
        }
        if let Some(p) = prop
            && self.property_ignores(unit, p)
        {
            return;
        }
        ctx.report(
            self.name(),
            Span {
                start: span.start + num_len as u32,
                end: span.end,
            },
            format!("Disallowed unit \"{unit}\""),
        );
    }
    fn check_percent<'a>(&self, span: Span, prop: Option<&str>, ctx: &mut LintContext<'a>) {
        if self.allowed.iter().any(|u| u == "%") {
            return;
        }
        if let Some(p) = prop
            && self.property_ignores("%", p)
        {
            return;
        }
        ctx.report(
            self.name(),
            Span {
                start: span.end - 1,
                end: span.end,
            },
            "Disallowed unit \"%\"".to_string(),
        );
    }
}
impl Rule for UnitAllowedList {
    fn name(&self) -> &'static str {
        "unit-allowed-list"
    }
    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let prop = ident_name(&decl.name);
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
        let tokens = tokenize_value(ctx.source, sp);
        self.check_tokens(&tokens, None, ctx);
    }
}
#[cfg(test)]
mod tests {

    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;
    #[test]
    fn percent_span() {
        let fail = [reject(
            "a { font-size: 80%; }",
            None,
            "Disallowed unit \"%\"",
            (1, 18, 1, 19),
        )];
        Tester::new("unit-allowed-list", json!(["px"]), [], &fail).test();
        let fail = [reject(
            "a { margin: calc(100px - #{$margin * 2rem}); }",
            None,
            "Disallowed unit \"rem\"",
            (1, 39, 1, 43),
        )];
        Tester::new("unit-allowed-list", json!(["px", "em"]), [], &fail).test();
    }
    #[test]
    fn default_config() {
        let pass = vec![
            "a { margin: 10px; }",
            "a { margin: 1em; }",
            "a { margin: 0; }",
            "a { color: #000; }",
            "@media (min-width: 10px) {}",
        ];
        let fail = vec![
            reject_message("a { margin: 10%; }", None, "Disallowed unit \"%\""),
            reject_message("a { margin: 10rem; }", None, "Disallowed unit \"rem\""),
            reject_message(
                "a { transform: rotate(45deg); }",
                None,
                "Disallowed unit \"deg\"",
            ),
        ];
        Tester::new("unit-allowed-list", json!(["px", "em"]), &pass, &fail).test();
    }
}
