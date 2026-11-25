use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, options_matches, parse_option_matchers},
};
use oxc_css_parser::ast::{SimpleBlock, Statement, Stylesheet};
use serde_json::{Map, Value};

pub struct NoDescendingSpecificity {
    ignore: Vec<OptionMatcher>,
    ignore_selectors_within_list: bool,
}

impl NoDescendingSpecificity {
    pub fn from_config(secondary: Option<&Map<String, Value>>) -> Self {
        let ignore = parse_option_matchers(secondary, "ignore");
        Self {
            ignore_selectors_within_list: options_matches(&ignore, "selectors-within-list"),
            ignore,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Specificity(usize, usize, usize);

fn compute_specificity_and_tail(selector: &str) -> Option<(Specificity, String)> {
    let trimmed = selector.trim();
    if trimmed.is_empty() || trimmed.contains('(') {
        return None;
    }

    let bytes = trimmed.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    let mut a = 0usize;
    let mut b = 0usize;
    let mut c = 0usize;
    let mut at_compound_start = true;

    while i < len {
        if bytes[i].is_ascii_whitespace() || matches!(bytes[i], b'>' | b'+' | b'~') {
            at_compound_start = true;
            i += 1;
            continue;
        }
        if bytes[i] == b'[' {
            at_compound_start = false;
            b += 1;
            i += 1;
            while i < len && bytes[i] != b']' {
                i += 1;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'#' {
            at_compound_start = false;
            a += 1;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'.' {
            at_compound_start = false;
            b += 1;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b':' {
            at_compound_start = false;
            if i + 1 < len && bytes[i + 1] == b':' {
                c += 1;
                i += 2;
            } else {
                let name_start = i + 1;
                let mut j = name_start;
                while j < len
                    && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'-' || bytes[j] == b'_')
                {
                    j += 1;
                }
                let pname = &trimmed[name_start..j];
                if pname.eq_ignore_ascii_case("before")
                    || pname.eq_ignore_ascii_case("after")
                    || pname.eq_ignore_ascii_case("first-line")
                    || pname.eq_ignore_ascii_case("first-letter")
                {
                    c += 1;
                } else {
                    b += 1;
                }
                i = j;
                continue;
            }
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        if at_compound_start && bytes[i].is_ascii_alphabetic() {
            c += 1;
            at_compound_start = false;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        at_compound_start = false;
        i += 1;
    }

    let last_compound = trimmed
        .rsplit(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '>' | '+' | '~'))
        .find(|s| !s.is_empty())?;
    let bytes = last_compound.as_bytes();
    let mut tail = String::with_capacity(last_compound.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' {
            let double = bytes.get(i + 1) == Some(&b':');
            let name_start = i + if double { 2 } else { 1 };
            let mut name_end = name_start;
            while name_end < bytes.len()
                && (bytes[name_end].is_ascii_alphanumeric()
                    || bytes[name_end] == b'-'
                    || bytes[name_end] == b'_')
            {
                name_end += 1;
            }
            let name = &last_compound[name_start..name_end];
            let pseudo_element = double
                || (!double
                    && ["before", "after", "first-line", "first-letter"]
                        .iter()
                        .any(|candidate| name.eq_ignore_ascii_case(candidate)));
            if pseudo_element {
                i = name_end;
                continue;
            }
        }
        let ch = last_compound[i..].chars().next()?;
        tail.push(ch);
        i += ch.len_utf8();
    }
    let tail_key = tail.to_ascii_lowercase();
    if tail_key.is_empty() {
        return None;
    }

    Some((Specificity(a, b, c), tail_key))
}

fn check_statements_specificity<'a>(
    statements: &'a [Statement<'a>],
    rule_name: &'static str,
    ignore: &[OptionMatcher],
    ignore_selectors_within_list: bool,
    ctx: &mut LintContext<'a>,
) {
    let mut seen: Vec<(String, String, Specificity, usize, u32)> = Vec::new();

    for stmt in statements {
        if let Statement::QualifiedRule(qrule) = stmt {
            let selector_list_start = qrule.selector.span.start;
            for complex in &qrule.selector.selectors {
                let raw = ctx.slice(complex.span).trim();
                if options_matches(ignore, raw) {
                    continue;
                }
                let Some((spec, tail_key)) = compute_specificity_and_tail(raw) else {
                    continue;
                };

                if let Some((_, prev_raw, _, prev_line, _)) =
                    seen.iter()
                        .find(|(prev_tail, _, prev_spec, _, prev_list_start)| {
                            prev_tail == &tail_key
                                && *prev_spec > spec
                                && (!ignore_selectors_within_list
                                    || *prev_list_start != selector_list_start)
                        })
                {
                    ctx.report(
                        rule_name,
                        complex.span,
                        format!(
                            "Expected selector \"{raw}\" to come before selector \"{prev_raw}\", at line {prev_line}"
                        ),
                    );
                }

                let line = ctx.line_index.line_col(ctx.source, complex.span.start).0;
                seen.push((tail_key, raw.to_string(), spec, line, selector_list_start));
            }
        }
    }
}

impl Rule for NoDescendingSpecificity {
    fn name(&self) -> &'static str {
        "no-descending-specificity"
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        check_statements_specificity(
            &stylesheet.statements,
            self.name(),
            &self.ignore,
            self.ignore_selectors_within_list,
            ctx,
        );
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        check_statements_specificity(
            &block.statements,
            self.name(),
            &self.ignore,
            self.ignore_selectors_within_list,
            ctx,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_message};
    use serde_json::json;

    #[test]
    fn includes_previous_selector_line() {
        let fail = [reject_message(
            "#a .bar {}\n.foo .bar {}",
            None,
            "Expected selector \".foo .bar\" to come before selector \"#a .bar\", at line 1",
        )];
        Tester::new("no-descending-specificity", json!(true), [], &fail).test();
    }

    #[test]
    fn compares_pseudo_classes_without_pseudo_elements() {
        Tester::new(
            "no-descending-specificity",
            json!(true),
            ["a:focus::before {} a:hover {}"],
            &[],
        )
        .test();
        let fail = [reject_message(
            "#a:hover::before {} #a:hover {}",
            None,
            "Expected selector \"#a:hover\" to come before selector \"#a:hover::before\", at line 1",
        )];
        Tester::new("no-descending-specificity", json!(true), [], &fail).test();
    }

    #[test]
    fn ignores_configured_selectors() {
        Tester::new(
            "no-descending-specificity",
            json!([true, {"ignore": ["#a .bar", "/^\\.foo/"]}]),
            ["#a .bar {} .foo .bar {}"],
            &[],
        )
        .test();
        Tester::new(
            "no-descending-specificity",
            json!([true, {"ignore": ["selectors-within-list"]}]),
            ["#a .bar, .foo .bar {}"],
            &[],
        )
        .test();
    }
}
