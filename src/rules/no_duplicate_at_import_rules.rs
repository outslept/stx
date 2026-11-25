use std::collections::{HashMap, HashSet};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{split_top_level, strip_comments},
};
use oxc_css_parser::ast::{
    AtRulePrelude, ImportPreludeHref, InterpolableStr, Statement, Stylesheet, UrlValue,
};

pub struct NoDuplicateAtImportRules;

impl Rule for NoDuplicateAtImportRules {
    fn name(&self) -> &'static str {
        "no-duplicate-at-import-rules"
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let mut seen: HashMap<&str, HashSet<String>> = HashMap::new();

        for stmt in &stylesheet.statements {
            let Statement::AtRule(at_rule) = stmt else {
                continue;
            };
            if !at_rule.name.name.eq_ignore_ascii_case("import") {
                continue;
            }

            let (uri, conditions) = if let Some(AtRulePrelude::Import(import)) = &at_rule.prelude {
                let uri = match &import.href {
                    ImportPreludeHref::Str(InterpolableStr::Literal(href)) => unquote(href.raw),
                    ImportPreludeHref::Url(url) => match &url.value {
                        Some(UrlValue::Str(InterpolableStr::Literal(href))) => unquote(href.raw),
                        Some(UrlValue::Raw(href)) => href.raw.trim(),
                        _ => continue,
                    },
                    _ => continue,
                };
                let prefix: Vec<_> = [
                    import.layer.as_ref().map(|layer| layer.span()),
                    import.supports.as_ref().map(|supports| supports.span),
                ]
                .into_iter()
                .flatten()
                .map(|span| condition_key(ctx.slice(span)))
                .collect();
                let prefix = prefix.join(" ");
                let conditions = if let Some(media) = &import.media {
                    media
                        .queries
                        .iter()
                        .map(|query| {
                            let key = condition_key(ctx.slice(query.span()));
                            if prefix.is_empty() {
                                key
                            } else {
                                format!("{prefix} {key}")
                            }
                        })
                        .collect()
                } else if let Some(modifiers) = &import.modifiers {
                    media_keys(ctx.slice(modifiers.span))
                } else if prefix.is_empty() {
                    Vec::new()
                } else {
                    vec![prefix]
                };
                (uri, conditions)
            } else {
                let prelude = ctx.slice_range(at_rule.name.span.end, at_rule.span.end);
                let Some((uri, rest)) = parse_import_prelude(prelude.trim().trim_end_matches(';'))
                else {
                    continue;
                };
                (uri, media_keys(rest))
            };

            if seen.get(uri).is_some_and(|previous| {
                conditions.is_empty() || conditions.iter().any(|key| previous.contains(key))
            }) {
                ctx.report(
                    self.name(),
                    at_rule.span,
                    format!("Duplicate @import rule {uri}"),
                );
            } else {
                seen.entry(uri).or_default().extend(conditions);
            }
        }
    }
}

fn condition_key(condition: &str) -> String {
    strip_comments(condition).split_whitespace().collect()
}

fn media_keys(rest: &str) -> Vec<String> {
    split_top_level(rest, b',')
        .into_iter()
        .map(|(_, _, media)| condition_key(media))
        .filter(|key| !key.is_empty())
        .collect()
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix(['\'', '"'])
        .and_then(|inner| inner.strip_suffix(value.as_bytes()[0] as char))
        .unwrap_or(value)
}

fn parse_import_prelude(prelude: &str) -> Option<(&str, &str)> {
    let s = prelude.trim();
    if s.is_empty() {
        return None;
    }

    if s.get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("url("))
    {
        let after_url = &s[4..];
        let close_idx = after_url.find(')')?;
        let inner = after_url[..close_idx].trim();
        let unquoted = unquote(inner);
        let rest = after_url[close_idx + 1..].trim();
        return Some((unquoted, rest));
    }

    let first = s.as_bytes()[0];
    if first == b'"' || first == b'\'' {
        let quote = first as char;
        let rest_slice = &s[1..];
        let end_quote = rest_slice.find(quote)?;
        let uri = &rest_slice[..end_quote];
        let rest = rest_slice[end_quote + 1..].trim();
        return Some((uri, rest));
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_start};
    use serde_json::json;

    #[test]
    fn compares_normalized_conditions_with_their_layer_and_supports() {
        let message = "Duplicate @import rule a.css";
        Tester::new(
            "no-duplicate-at-import-rules",
            json!(true),
            [
                "@import 'a.css' layer(one) screen, tv; @import 'a.css' layer(two) tv;",
                "@import 'a.css' supports(selector(:is(a, b))) tv; @import 'a.css' supports(selector(:is(c, b))) tv;",
                "@import 'a.css' screen, screen;",
                "@import 'a.css'; @import 'a.css' screen;",
            ],
            [
                reject_start(
                    "@import 'a.css' screen;\n@import 'a.css' screen, tv;\n@import 'a.css' tv;",
                    None,
                    message,
                    (2, 1),
                ),
                reject_start(
                    "@import 'a.css' screen;\n@import 'a.css';",
                    None,
                    message,
                    (2, 1),
                ),
                reject_start(
                    "@import url('a.css') /* a comment */ tv; @import 'a.css' tv /* a comment */;",
                    None,
                    message,
                    (1, 42),
                ),
                reject_start(
                    "@import 'a.css' (min-width : 500px);@import url(a.css) (  min-width:500px   );",
                    None,
                    message,
                    (1, 37),
                ),
                reject(
                    "@import url(\"a.css\") layer(base) supports(display: grid) screen, tv; @import url(\"a.css\") layer(base) supports(display: grid) tv;",
                    None,
                    message,
                    (1, 70, 1, 129),
                ),
            ],
        )
        .test();
    }

    #[test]
    fn duplicate_import_message_matches_stylelint_18() {
        let pass = ["@import 'a.css'; @import 'b.css';"];
        let fail = [reject(
            "@import 'a.css'; @import 'a.css';",
            None,
            "Duplicate @import rule a.css",
            (1, 18, 1, 33),
        )];
        Tester::new("no-duplicate-at-import-rules", json!(true), pass, &fail).test();
    }
}
