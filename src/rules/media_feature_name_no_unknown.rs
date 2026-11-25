use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        MEDIA_FEATURE_NAMES, OptionMatcher, ValueToken, ValueTokenKind, at_rule_prelude_span,
        in_set_ignore_case, options_matches, parse_option_matchers, tokenize_value, vendor_prefix,
    },
};
use oxc_css_parser::{Span, ast::AtRule};
use serde_json::{Map, Value};

crate::define_stx_lint! {
    /// Disallow unknown media feature names in `@media` queries.
    ///
    /// Enable with `true`. `ignoreMediaFeatureNames` accepts a name or array
    /// of names and `/regex/` patterns matched against the original spelling.
    /// Built-in names are recognized case-insensitively. Vendor-prefixed
    /// features and custom media names beginning with `--` are skipped.
    ///
    /// Example: `@media (10em < unknown <= 50em) {}` is reported.
    "media-feature-name-no-unknown", correctness, recommended = true;
    pub struct MediaFeatureNameNoUnknown {
        ignore: Vec<OptionMatcher>,
    }
}

impl MediaFeatureNameNoUnknown {
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore: parse_option_matchers(secondary, "ignoreMediaFeatureNames"),
        }
    }
}

impl Rule for MediaFeatureNameNoUnknown {
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
        for_each_media_feature(ctx.source, prelude, |name, _| {
            let raw = name.raw;
            if !raw.starts_with("--")
                && vendor_prefix(raw).is_none()
                && !in_set_ignore_case(MEDIA_FEATURE_NAMES, raw)
                && !options_matches(&self.ignore, raw)
            {
                ctx.report(
                    self.name(),
                    name.span,
                    format!("Unknown media feature name \"{raw}\""),
                );
            }
        });
    }
}

pub(super) fn for_each_media_feature<'a>(
    source: &'a str,
    prelude: Span,
    mut visit: impl FnMut(&ValueToken<'a>, &[&[ValueToken<'a>]]),
) {
    let tokens = tokenize_value(source, prelude);
    let mut closes = vec![None; tokens.len()];
    let mut stack = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        match token.raw {
            "(" | "[" | "{" => stack.push((i, token.raw)),
            ")" | "]" | "}" => {
                let open = match token.raw {
                    ")" => "(",
                    "]" => "[",
                    _ => "{",
                };
                if stack.last().is_some_and(|&(_, kind)| kind == open) {
                    let (start, _) = stack.pop().unwrap();
                    closes[start] = Some(i);
                }
            }
            _ if token.kind == ValueTokenKind::Function => stack.push((i, "(")),
            _ => {}
        }
    }

    let mut components = Vec::new();
    let mut i = 0;
    while let Some(token) = tokens.get(i) {
        let Some(close) = closes[i] else {
            i += 1;
            continue;
        };
        if token.raw != "(" {
            i = close + 1;
            continue;
        }
        if tokens[i + 1].raw == "(" || tokens[i + 1].raw.eq_ignore_ascii_case("not") {
            i += 1;
            continue;
        }
        components.clear();
        i += 1;
        while i < close {
            let mut component = tokens[i].clone();
            if let Some(end) = closes[i] {
                i = end;
                component.span.end = tokens[end].span.end;
            }
            components.push(component);
            i += 1;
        }
        visit_feature(&components, &mut visit);
        i = close + 1;
    }
}

fn feature_name<'t, 'a>(tokens: &'t [ValueToken<'a>]) -> Option<&'t ValueToken<'a>> {
    let [token] = tokens else { return None };
    (token.kind == ValueTokenKind::Ident
        && !token.raw.starts_with(['$', '@'])
        && !["not", "only", "and", "or"]
            .iter()
            .any(|k| token.raw.eq_ignore_ascii_case(k)))
    .then_some(token)
}

fn visit_feature<'a>(
    tokens: &[ValueToken<'a>],
    visit: &mut impl FnMut(&ValueToken<'a>, &[&[ValueToken<'a>]]),
) {
    let mut operands = Vec::new();
    let mut start = 0;
    let mut i = 0;
    let end = tokens.len();
    while i < end {
        let token = &tokens[i];
        if token.kind == ValueTokenKind::Colon && operands.is_empty() {
            if let Some(name) = feature_name(&tokens[start..i]) {
                visit(name, &[&tokens[i + 1..end]]);
            }
            return;
        }
        if matches!(token.raw, "<" | ">" | "=") {
            if i == start || operands.len() == 2 {
                return;
            }
            operands.push(&tokens[start..i]);
            if token.raw != "="
                && i + 1 < end
                && tokens[i + 1].raw == "="
                && token.span.end == tokens[i + 1].span.start
            {
                i += 1;
            }
            start = i + 1;
        }
        i += 1;
    }
    if start == end {
        return;
    }
    let last = &tokens[start..end];
    match operands.as_slice() {
        [] => {
            if let Some(name) = feature_name(last) {
                visit(name, &[]);
            }
        }
        [first] => {
            if let Some(name) = feature_name(first) {
                visit(name, &[last]);
            } else if let Some(name) = feature_name(last) {
                visit(name, &[first]);
            }
        }
        [first, middle] => {
            if let Some(name) = feature_name(middle) {
                visit(name, &[first, last]);
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    const RULE: &str = "media-feature-name-no-unknown";

    #[test]
    fn finds_the_name_in_commented_and_reversed_features() {
        Tester::new(
            RULE,
            json!(true),
            ["@media (width: calc(1px + (bogus))) and (--custom) {}"],
            [
                reject(
                    "@media (/* ) */unknown) {}",
                    None,
                    "Unknown media feature name \"unknown\"",
                    (1, 16, 1, 23),
                ),
                reject(
                    "@media (10em < unknown <= 50em) {}",
                    None,
                    "Unknown media feature name \"unknown\"",
                    (1, 16, 1, 23),
                ),
            ],
        )
        .test();

        for (syntax, code, pass) in [
            (
                "postcss-scss",
                "@media ($var = unknown) {}",
                "@media (ns.$min < width < ns.$max) {}",
            ),
            (
                "postcss-less",
                "@media (@var = unknown) {}",
                "@media (1px < @feature < 2px) {}",
            ),
        ] {
            Tester::new(
                RULE,
                json!(true),
                [pass],
                [reject(
                    code,
                    None,
                    "Unknown media feature name \"unknown\"",
                    (1, 16, 1, 23),
                )],
            )
            .with_custom_syntax(syntax)
            .test();
        }
    }

    #[test]
    fn name_ignores_match_the_original_spelling() {
        Tester::new(
            RULE,
            json!([true, {"ignoreMediaFeatureNames": ["custom", "/^my-/", "/^YOUR-/i"]}]),
            ["@media (custom) and (my-feature) and (your-feature) {}"],
            [reject(
                "@media (CUSTOM) {}",
                None,
                "Unknown media feature name \"CUSTOM\"",
                (1, 9, 1, 15),
            )],
        )
        .test();
    }
}
