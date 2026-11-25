use std::collections::HashMap;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, options_matches, parse_option_matchers, secondary_bool},
};
use oxc_css_parser::ast::{SelectorList, Statement, Stylesheet};
use serde_json::{Map, Value};

pub struct NoDuplicateSelectors {
    disallow_in_list: bool,
    ignore_selectors: Vec<OptionMatcher>,
}

impl NoDuplicateSelectors {
    pub fn from_config(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            disallow_in_list: secondary_bool(secondary, "disallowInList").unwrap_or(false),
            ignore_selectors: parse_option_matchers(secondary, "ignoreSelectors"),
        }
    }
}

fn normalize_selector(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    let mut in_attribute = false;
    let mut quote = 0;
    let mut pending_space = false;

    while i < bytes.len() {
        let byte = bytes[i];
        if byte == b'\\' && i + 1 < bytes.len() {
            if pending_space && !in_attribute {
                out.push(' ');
                pending_space = false;
            }
            i += 1;
            let start = i;
            let mut codepoint = 0u32;
            while i < bytes.len() && i - start < 6 && bytes[i].is_ascii_hexdigit() {
                codepoint = codepoint * 16 + (bytes[i] as char).to_digit(16).unwrap_or(0);
                i += 1;
            }
            if i > start {
                if i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if let Some(ch) = char::from_u32(codepoint) {
                    out.push(ch);
                }
            } else {
                let ch = raw[i..].chars().next().unwrap();
                out.push(ch);
                i += ch.len_utf8();
            }
            continue;
        }
        if quote != 0 {
            let ch = raw[i..].chars().next().unwrap();
            out.push(ch);
            if byte == quote {
                quote = 0;
            }
            i += ch.len_utf8();
            continue;
        }
        if in_attribute && matches!(byte, b'\'' | b'"') {
            quote = byte;
        }
        if byte == b'[' {
            in_attribute = true;
        } else if byte == b']' {
            in_attribute = false;
        }
        if byte.is_ascii_whitespace() {
            pending_space = true;
            i += 1;
            continue;
        }
        if pending_space {
            if in_attribute {
                if !matches!(byte, b'=' | b']') && !out.ends_with(['[', '=']) {
                    out.push(' ');
                }
            } else if !matches!(byte, b'>' | b'+' | b'~' | b',' | b')' | b'(' | b']')
                && !out.ends_with(['>', '+', '~', ',', '('])
                && !out.is_empty()
            {
                out.push(' ');
            }
            pending_space = false;
        }
        let ch = raw[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }

    out.trim().to_string()
}

fn replace_nesting(selector: &str, parent: &str) -> Option<String> {
    let mut out = String::with_capacity(selector.len() + parent.len());
    let mut in_attribute = false;
    let mut quote = None;
    let mut escaped = false;
    let mut replaced = false;
    for ch in selector.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            out.push(ch);
            escaped = true;
        } else if let Some(active_quote) = quote {
            out.push(ch);
            if ch == active_quote {
                quote = None;
            }
        } else if in_attribute && matches!(ch, '\'' | '"') {
            out.push(ch);
            quote = Some(ch);
        } else if ch == '[' {
            out.push(ch);
            in_attribute = true;
        } else if ch == ']' {
            out.push(ch);
            in_attribute = false;
        } else if ch == '&' && !in_attribute {
            out.push_str(parent);
            replaced = true;
        } else {
            out.push(ch);
        }
    }
    replaced.then_some(out)
}

fn expand_nested(parents: &[String], selectors: &[String]) -> Vec<String> {
    if parents.is_empty() {
        return selectors.to_vec();
    }
    parents
        .iter()
        .flat_map(|parent| {
            selectors.iter().map(move |selector| {
                replace_nesting(selector, parent).unwrap_or_else(|| format!("{parent} {selector}"))
            })
        })
        .collect()
}

fn check_selector_list(
    list: &SelectorList<'_>,
    parents: &[String],
    seen: &mut HashMap<String, usize>,
    rule: &NoDuplicateSelectors,
    ctx: &mut LintContext<'_>,
) -> Vec<String> {
    let selectors: Vec<_> = list
        .selectors
        .iter()
        .map(|selector| ctx.slice(selector.span).trim().to_string())
        .filter(|selector| !selector.is_empty())
        .collect();
    if selectors.is_empty() {
        return Vec::new();
    }
    let expanded = expand_nested(parents, &selectors);

    if rule.disallow_in_list {
        let mut new_selectors = HashMap::new();
        for (index, selector) in expanded.iter().enumerate() {
            let selector_span = list.selectors[index % list.selectors.len()].span;
            let normalized = normalize_selector(selector);
            if options_matches(&rule.ignore_selectors, selector) {
                continue;
            }
            let line = ctx.line_index.line_col(ctx.source, selector_span.start).0;
            if let Some(first_line) = seen.get(&normalized).copied() {
                let raw = selectors[index % selectors.len()].as_str();
                ctx.report(
                    rule.name(),
                    selector_span,
                    format!("Duplicate selector \"{raw}\", first used at line {first_line}"),
                );
            } else {
                new_selectors.entry(normalized).or_insert(line);
            }
        }
        seen.extend(new_selectors);
        return expanded;
    }

    let mut in_list = HashMap::new();
    for (index, selector) in expanded.iter().enumerate() {
        let selector_span = list.selectors[index % list.selectors.len()].span;
        let normalized = normalize_selector(selector);
        if options_matches(&rule.ignore_selectors, selector) {
            continue;
        }
        if let Some(first_line) = in_list.get(&normalized).copied() {
            ctx.report(
                rule.name(),
                selector_span,
                format!(
                    "Duplicate selector \"{}\", first used at line {first_line}",
                    selectors[index % selectors.len()]
                ),
            );
        } else {
            let line = ctx.line_index.line_col(ctx.source, selector_span.start).0;
            in_list.insert(normalized, line);
        }
    }

    if expanded
        .iter()
        .any(|selector| options_matches(&rule.ignore_selectors, selector))
    {
        return expanded;
    }
    let mut normalized: Vec<_> = expanded
        .iter()
        .map(|selector| normalize_selector(selector))
        .collect();
    normalized.sort();
    let key = normalized.join(",");
    let first = list.selectors.first().unwrap().span;
    let last = list.selectors.last().unwrap().span;
    let mut span = oxc_css_parser::Span {
        start: first.start,
        end: last.end,
    };
    while span.end > span.start
        && ctx.source.as_bytes()[span.end as usize - 1].is_ascii_whitespace()
    {
        span.end -= 1;
    }
    let line = ctx.line_index.line_col(ctx.source, span.start).0;
    if let Some(first_line) = seen.get(&key).copied() {
        ctx.report(
            rule.name(),
            span,
            format!(
                "Duplicate selector \"{}\", first used at line {first_line}",
                ctx.slice(span).trim()
            ),
        );
    } else {
        seen.insert(key, line);
    }

    expanded
}

fn check_statements<'a>(
    statements: &'a [Statement<'a>],
    parents: &[String],
    seen: &mut HashMap<String, usize>,
    rule: &NoDuplicateSelectors,
    ctx: &mut LintContext<'a>,
) {
    for statement in statements {
        match statement {
            Statement::QualifiedRule(qrule) => {
                let selectors = check_selector_list(&qrule.selector, parents, seen, rule, ctx);
                check_statements(&qrule.block.statements, &selectors, seen, rule, ctx);
            }
            Statement::AtRule(at_rule) => {
                if let Some(block) = &at_rule.block {
                    let mut scoped_seen = HashMap::new();
                    check_statements(&block.statements, parents, &mut scoped_seen, rule, ctx);
                }
            }
            Statement::KeyframeBlock(_) => {}
            _ => {}
        }
    }
}

impl Rule for NoDuplicateSelectors {
    fn name(&self) -> &'static str {
        "no-duplicate-selectors"
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let mut seen = HashMap::new();
        check_statements(&stylesheet.statements, &[], &mut seen, self, ctx);
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;

    #[test]
    fn reports_first_duplicate_line() {
        let fail = [reject_message(
            "a {}\n\na {}",
            None,
            "Duplicate selector \"a\", first used at line 1",
        )];
        Tester::new("no-duplicate-selectors", json!(true), &[], &fail).test();
        let fail = [reject_message(
            "b {}\n  b {}",
            None,
            "Duplicate selector \"b\", first used at line 1",
        )];
        Tester::new("no-duplicate-selectors", json!(true), [], &fail).test();
    }

    #[test]
    fn normalizes_lists_escapes_and_attribute_values() {
        let fail = [reject_message(
            ".foo {} .\\66 oo {}",
            None,
            "Duplicate selector \".\\66 oo\", first used at line 1",
        )];
        Tester::new("no-duplicate-selectors", json!(true), [], &fail).test();
        Tester::new(
            "no-duplicate-selectors",
            json!(true),
            ["[data-x=\"a  b\"] {} [data-x=\"a b\"] {}"],
            &[],
        )
        .test();
    }

    #[test]
    fn nested_selectors_and_options() {
        let fail = [
            reject_message(
                "a { & .b {} } a .b {}",
                None,
                "Duplicate selector \"a .b\", first used at line 1",
            ),
            reject_message(
                "div { [data-x=\"&\"] {} } div [data-x=\"&\"] {}",
                None,
                "Duplicate selector \"div [data-x=\"&\"]\", first used at line 1",
            ),
        ];
        Tester::new("no-duplicate-selectors", json!(true), [], &fail).test();
        let duplicate_in_list = [reject_message(
            "a, a {}",
            None,
            "Duplicate selector \"a\", first used at line 1",
        )];
        Tester::new(
            "no-duplicate-selectors",
            json!(true),
            [],
            &duplicate_in_list,
        )
        .test();
        Tester::new(
            "no-duplicate-selectors",
            json!([true, {"disallowInList": true}]),
            ["a, a {}"],
            &[],
        )
        .test();
        Tester::new(
            "no-duplicate-selectors",
            json!(true),
            ["a {}\n@media print { a {} }"],
            &[],
        )
        .test();
        Tester::new(
            "no-duplicate-selectors",
            json!([true, {"ignoreSelectors": ["a", "/^\\.skip/"]}]),
            ["a {} a {} .skip {} .skip {}"],
            &[],
        )
        .test();
    }

    #[test]
    fn excludes_trailing_whitespace_from_report_spans() {
        let fail = [reject(
            ".u-m\\2b {} .u-m\\00002b {}",
            None,
            "Duplicate selector \".u-m\\00002b\", first used at line 1",
            (1, 12, 1, 23),
        )];
        Tester::new("no-duplicate-selectors", json!(true), [], &fail).test();
    }
}
