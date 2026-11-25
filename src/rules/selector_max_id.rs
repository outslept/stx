use crate::context::LintContext;
use crate::rules::Rule;
use oxc_css_parser::Span;
use oxc_css_parser::ast::*;
use regex::Regex;
use serde_json::{Map, Value};

pub struct SelectorMaxId {
    max: usize,
    ignore_functions: Vec<String>,
    ignore_functions_regexes: Vec<Regex>,
    ignore_context_functional_pseudo_classes: Vec<String>,
    ignore_context_regexes: Vec<Regex>,
}

impl SelectorMaxId {
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Self {
        let max = primary.as_u64().unwrap_or(0) as usize;

        let mut ignore_functions = Vec::new();
        let mut ignore_functions_regexes = Vec::new();
        let mut ignore_context_functional_pseudo_classes = Vec::new();
        let mut ignore_context_regexes = Vec::new();

        if let Some(sec) = secondary {
            if let Some(Value::Array(arr)) = sec.get("ignoreFunctions") {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        if let Some(re) = parse_regex_string(s) {
                            ignore_functions_regexes.push(re);
                        } else {
                            ignore_functions.push(s.to_string());
                        }
                    }
                }
            }
            if let Some(Value::Array(arr)) = sec.get("ignoreContextFunctionalPseudoClasses") {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        if let Some(re) = parse_regex_string(s) {
                            ignore_context_regexes.push(re);
                        } else {
                            ignore_context_functional_pseudo_classes.push(s.to_string());
                        }
                    }
                }
            }
        }

        Self {
            max,
            ignore_functions,
            ignore_functions_regexes,
            ignore_context_functional_pseudo_classes,
            ignore_context_regexes,
        }
    }

    fn is_ignored_function(&self, func_name: &str) -> bool {
        if self.ignore_functions.iter().any(|f| f == func_name) {
            return true;
        }
        self.ignore_functions_regexes
            .iter()
            .any(|re| re.is_match(func_name))
    }

    fn is_ignored_context_pseudo(&self, pseudo_raw: &str) -> bool {
        if self
            .ignore_context_functional_pseudo_classes
            .iter()
            .any(|p| p == pseudo_raw)
        {
            return true;
        }
        self.ignore_context_regexes
            .iter()
            .any(|re| re.is_match(pseudo_raw))
    }

    fn count_ids_in_selector(&self, selector_str: &str) -> usize {
        let stripped = strip_comments_and_strings(selector_str);
        self.count_ids_recursive(&stripped)
    }

    fn count_ids_recursive(&self, s: &str) -> usize {
        let bytes = s.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        let mut count = 0;

        while i < len {
            let b = bytes[i];

            if b == b'[' {
                i += 1;
                let mut depth = 1;
                while i < len && depth > 0 {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'[' {
                        depth += 1;
                    } else if bytes[i] == b']' {
                        depth -= 1;
                    }
                    i += 1;
                }
                continue;
            }

            if b == b':' {
                let start = i;
                i += 1;
                if i < len && bytes[i] == b':' {
                    i += 1;
                }
                let name_start = start;
                while i < len && is_ident_char(bytes[i]) {
                    i += 1;
                }
                let pseudo_raw = &s[name_start..i];
                if i < len && bytes[i] == b'(' {
                    let args_start = i + 1;
                    let close_idx = find_matching_paren(bytes, i);
                    let inner = &s[args_start..close_idx];
                    i = if close_idx < len { close_idx + 1 } else { len };

                    if self.is_ignored_context_pseudo(pseudo_raw) {
                        continue;
                    }

                    for arg in split_top_level_commas(inner) {
                        count += self.count_ids_recursive(arg);
                    }
                    continue;
                }
                continue;
            }

            if is_ident_start(b) {
                let start = i;
                while i < len && is_ident_char(bytes[i]) {
                    i += 1;
                }
                let ident = &s[start..i];
                if i < len && bytes[i] == b'(' {
                    let args_start = i + 1;
                    let close_idx = find_matching_paren(bytes, i);
                    let inner = &s[args_start..close_idx];
                    i = if close_idx < len { close_idx + 1 } else { len };

                    if self.is_ignored_function(ident) {
                        continue;
                    }

                    for arg in split_top_level_commas(inner) {
                        count += self.count_ids_recursive(arg);
                    }
                    continue;
                }
                continue;
            }

            if b == b'#' {
                if i + 1 < len && bytes[i + 1] == b'{' {
                    let close_idx = find_matching_brace(bytes, i + 1);
                    i = if close_idx < len { close_idx + 1 } else { len };
                    continue;
                }
                i += 1;
                let id_start = i;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 2;
                    } else if is_ident_char(bytes[i]) {
                        i += 1;
                    } else {
                        break;
                    }
                }
                if i > id_start {
                    count += 1;
                }
                continue;
            }

            if b == b'\\' && i + 1 < len {
                i += 2;
                continue;
            }

            i += 1;
        }

        count
    }
}

impl Rule for SelectorMaxId {
    fn name(&self) -> &'static str {
        "selector-max-id"
    }

    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        let prelude_raw = ctx.slice(rule.selector.span);
        if prelude_raw.contains("#{") || prelude_raw.contains("@{") {
            return;
        }
        for sel in &rule.selector.selectors {
            let (stripped_span, selector_str) = strip_selector_span(ctx.source, sel.span);
            let count = self.count_ids_in_selector(selector_str);
            if count > self.max {
                ctx.report(
                    self.name(),
                    stripped_span,
                    format!(
                        "Too many ID selectors in \"{}\", maximum {}",
                        selector_str, self.max
                    ),
                );
            }
        }
    }
}

fn parse_regex_string(s: &str) -> Option<Regex> {
    if s.starts_with('/') && s.len() >= 2 {
        if let Some(last_slash) = s.rfind('/') {
            if last_slash > 0 {
                let pattern = &s[1..last_slash];
                let flags = &s[last_slash + 1..];
                let full_pattern = if flags.contains('i') {
                    format!("(?i){}", pattern)
                } else {
                    pattern.to_string()
                };
                return Regex::new(&full_pattern).ok();
            }
        }
    }
    None
}

fn strip_selector_span<'a>(source: &'a str, span: Span) -> (Span, &'a str) {
    let raw = &source[span.start as usize..span.end as usize];
    let trimmed_start = raw.trim_start();
    let leading = raw.len() - trimmed_start.len();
    let trimmed = trimmed_start.trim_end();
    let new_start = span.start + leading as u32;
    let new_end = new_start + trimmed.len() as u32;
    (
        Span {
            start: new_start,
            end: new_end,
        },
        trimmed,
    )
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-' || b >= 0x80
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b >= 0x80
}

fn find_matching_paren(bytes: &[u8], open_pos: usize) -> usize {
    let len = bytes.len();
    let mut i = open_pos + 1;
    let mut depth = 1;
    while i < len {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'(' {
            depth += 1;
        } else if bytes[i] == b')' {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
        i += 1;
    }
    len
}

fn find_matching_brace(bytes: &[u8], open_pos: usize) -> usize {
    let len = bytes.len();
    let mut i = open_pos + 1;
    let mut depth = 1;
    while i < len {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'{' {
            depth += 1;
        } else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
        i += 1;
    }
    len
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut result = Vec::new();
    let mut start = 0;
    let mut i = 0;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    while i < len {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        match b {
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b',' if paren_depth == 0 && bracket_depth == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    result.push(&s[start..len]);
    result
}

fn strip_comments_and_strings(s: &str) -> String {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\\' && i + 1 < len {
            out.push(bytes[i]);
            out.push(bytes[i + 1]);
            i += 2;
            continue;
        }
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            out.push(b' ');
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject, reject_warnings};

    const NAME: &str = "selector-max-id";

    fn expected(selector: &str, max: usize) -> String {
        format!("Too many ID selectors in \"{selector}\", maximum {max}")
    }

    #[test]
    fn sanity_checks() {
        let pass = [
            "foo {}",
            ".bar {}",
            "foo .bar {}",
            "[foo] {}",
            ":root { --foo: 1px; }",
            "html { --foo: 1px; }",
            ":root { --custom-property-set: {} }",
            "html { --custom-property-set: {} }",
            "/* stylelint-disable-next-line selector-max-id */\n#foo, /* stylelint-disable-next-line selector-max-id */\n#bar,\n/* stylelint-disable-next-line selector-max-id */\n#baz\n{}",
        ];
        let fail = [
            reject("#foo {}", None, expected("#foo", 0), (1, 1, 1, 5)),
            reject(
                ".bar #foo {}",
                None,
                expected(".bar #foo", 0),
                (1, 1, 1, 10),
            ),
            reject(
                ".bar > #foo {}",
                None,
                expected(".bar > #foo", 0),
                (1, 1, 1, 12),
            ),
            reject("#foo.bar {}", None, expected("#foo.bar", 0), (1, 1, 1, 9)),
            reject(
                ".foo, .bar, #foo.baz {}",
                None,
                expected("#foo.baz", 0),
                (1, 13, 1, 21),
            ),
            reject(
                "#foo [lang^=en] {}",
                None,
                expected("#foo [lang^=en]", 0),
                (1, 1, 1, 16),
            ),
            reject(
                "#foo[lang^=en] {}",
                None,
                expected("#foo[lang^=en]", 0),
                (1, 1, 1, 15),
            ),
            reject("* #foo {}", None, expected("* #foo", 0), (1, 1, 1, 7)),
            reject("#foo * {}", None, expected("#foo *", 0), (1, 1, 1, 7)),
            reject("*#foo {}", None, expected("*#foo", 0), (1, 1, 1, 6)),
            reject(
                "#foo:hover {}",
                None,
                expected("#foo:hover", 0),
                (1, 1, 1, 11),
            ),
            reject(
                ":not(#foo) {}",
                None,
                expected(":not(#foo)", 0),
                (1, 1, 1, 11),
            ),
            reject_warnings(
                "/* a comment */\n#foo, /* a comment */\n#bar,\n/* a comment */\n#foo\n{}",
                None,
                vec![
                    (expected("#foo", 0), (2, 1, 2, 5)),
                    (expected("#bar", 0), (3, 1, 3, 5)),
                    (expected("#foo", 0), (5, 1, 5, 5)),
                ],
            ),
        ];
        Tester::new(NAME, json!([0]), pass, fail).test();
    }

    #[test]
    fn standard_tests() {
        let pass = [
            "#foo {}",
            "#foo:hover {}",
            "#foo #bar {}",
            "#foo, \n#bar {}",
            "#foo #bar, \n#bar #foo {}",
            "#foo { #bar {} }",
            "#foo { #bar > & {} }",
            "#foo, #bar { & > #foo {} }",
            "@media print { #foo #bar {} }",
            "#foo { @media print { #bar {} } }",
            ".foo { @media print { #bar #baz {} } }",
            "#foo { #baz { .quux {} } }",
            "#foo { .baz { #quux {} } }",
        ];
        let fail = [
            reject(
                "#foo #bar #baz {}",
                None,
                expected("#foo #bar #baz", 2),
                (1, 1, 1, 15),
            ),
            reject(
                "#foo #bar:not(#baz) {}",
                None,
                expected("#foo #bar:not(#baz)", 2),
                (1, 1, 1, 20),
            ),
            reject(
                ":--foo(#foo) #bar #baz {}",
                None,
                expected(":--foo(#foo) #bar #baz", 2),
                (1, 1, 1, 23),
            ),
            reject(
                "#foo, \n#bar #baz #foo {}",
                None,
                expected("#bar #baz #foo", 2),
                (2, 1, 2, 15),
            ),
            reject(
                "#foo #bar #baz:not(#quux) {}",
                None,
                expected("#foo #bar #baz:not(#quux)", 2),
                (1, 1, 1, 26),
            ),
            reject(
                ".foo { @media print { #bar #baz #foo {} } }",
                None,
                expected("#bar #baz #foo", 2),
                (1, 23, 1, 37),
            ),
        ];
        Tester::new(NAME, json!([2]), pass, fail).test();
    }

    #[test]
    fn scss_max_0() {
        let pass = [
            "@keyframes spin { #{50% - $n} {} }",
            "@for $n from 1 through 10 { .n-#{$n} { content: \"n: #{1 + 1}\"; } }",
            "@for $n from 1 through 10 { .n#{$n}-#{$n} { content: \"n: #{1 + 1}\"; } }",
            "@for $n from 1 through 10 { .n#{$n}n#{$n} { content: \"n: #{1 + 1}\"; } }",
            "@each $n in $vals { .n-#{$n} { content: \"n: #{1 + 1}\"; } }",
            "@while $n < 10 { .n-#{$n} { content: \"n: #{1 + 1}\"; } }",
            "div:nth-child(#{map-get($foo, bar)}) {}",
            "@for $n from 1 through 10 { .n-#{$n} #foo {} }",
        ];
        let fail = [
            reject(
                "#foo { @include test {} }",
                None,
                expected("#foo", 0),
                (1, 1, 1, 5),
            ),
            reject(
                ".bar #foo { @include test {} }",
                None,
                expected(".bar #foo", 0),
                (1, 1, 1, 10),
            ),
            reject(
                ".bar > #foo { @include test {} }",
                None,
                expected(".bar > #foo", 0),
                (1, 1, 1, 12),
            ),
            reject(
                "#foo.bar { @include test {} }",
                None,
                expected("#foo.bar", 0),
                (1, 1, 1, 9),
            ),
            reject(
                ".foo, .bar, #foo.baz { @include test {} }",
                None,
                expected("#foo.baz", 0),
                (1, 13, 1, 21),
            ),
            reject(
                "#foo [lang^=en] { @include test {} }",
                None,
                expected("#foo [lang^=en]", 0),
                (1, 1, 1, 16),
            ),
            reject(
                "#foo[lang^=en] { @include test {} }",
                None,
                expected("#foo[lang^=en]", 0),
                (1, 1, 1, 15),
            ),
            reject(
                "* #foo { @include test {} }",
                None,
                expected("* #foo", 0),
                (1, 1, 1, 7),
            ),
            reject(
                "#foo * { @include test {} }",
                None,
                expected("#foo *", 0),
                (1, 1, 1, 7),
            ),
            reject(
                "*#foo { @include test {} }",
                None,
                expected("*#foo", 0),
                (1, 1, 1, 6),
            ),
            reject(
                "#foo:hover { @include test {} }",
                None,
                expected("#foo:hover", 0),
                (1, 1, 1, 11),
            ),
            reject(
                ":not(#foo) { @include test {} }",
                None,
                expected(":not(#foo)", 0),
                (1, 1, 1, 11),
            ),
            reject(
                "@include test { #foo {} }",
                None,
                expected("#foo", 0),
                (1, 17, 1, 21),
            ),
        ];
        Tester::new(NAME, json!([0]), pass, fail)
            .with_custom_syntax("postcss-scss")
            .test();
    }

    #[test]
    fn less_max_0() {
        let pass = [
            ".for(@n: 1) when (@n <= 10) { .n-@{n} { content: %(\"n: %d\", 1 + 1); } .for(@n + 1); }",
            ".for(@n: 1) when (@n <= 10) { .n-@{n}-@{n} { content: %(\"n: %d\", 1 + 1); } .for(@n + 1); }",
            ".for(@n: 1) when (@n <= 10) { .n-@{n}n@{n} { content: %(\"n: %d\", 1 + 1); } .for(@n + 1); }",
            ".each(@vals, @n: 1) when (@n <= length(@vals)) { @val: extract(@vals, @n); .n-@{val} { content: %(\"n: %d\", 1 + 1); } .each(@vals, @n + 1); }",
            ".while(@n: 0) when (@n < 10) { .n-@{n} { content: %(\"n: %d\", 1 + 1); } .while(@n + 1) }",
            ".for(@n: 1) when (@n <= 10) { .n-@{n} #foo {} .for(@n + 1) }",
        ];
        Tester::new(NAME, json!([0]), pass, [])
            .with_custom_syntax("postcss-less")
            .test();
    }

    #[test]
    fn scss_max_2() {
        let pass = [
            "#foo { @if ($p == 1) { #bar {} } }",
            ".foo { @if ($p == 1) { #bar #baz {} } }",
        ];
        let fail = [
            reject(
                "#foo #bar #baz { @if ($p == 1) { #bar {} } }",
                None,
                expected("#foo #bar #baz", 2),
                (1, 1, 1, 15),
            ),
            reject(
                ".foo { @if ($p == 1) { #bar #baz #foo {} } }",
                None,
                expected("#bar #baz #foo", 2),
                (1, 24, 1, 38),
            ),
        ];
        Tester::new(NAME, json!([2]), pass, fail)
            .with_custom_syntax("postcss-scss")
            .test();
    }

    #[test]
    fn ignore_context_functional_pseudo_classes() {
        let pass = ["a:not(#foo) {}", "a:has(#foo) {}", "a:not(:is(#foo)) {}"];
        let fail = [reject(
            "a:matches(#foo) {}",
            None,
            expected("a:matches(#foo)", 0),
            (1, 1, 1, 16),
        )];
        Tester::new(
            NAME,
            json!([0, { "ignoreContextFunctionalPseudoClasses": [":not", "/^:(h|H)as$/"] }]),
            pass,
            fail,
        )
        .test();
    }
}
