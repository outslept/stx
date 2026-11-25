use crate::{context::LintContext, rules::Rule, utils::*};
use oxc_css_parser::ast::QualifiedRule;
pub struct R {
    max: u64,
    ignore_types: Vec<OptionMatcher>,
    ignore_descendant: bool,
    ignore_child: bool,
    ignore_compounded: bool,
    ignore_next_sibling: bool,
    ignore_custom_elements: bool,
}
impl R {
    pub fn from_config(
        p: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        Some(Self {
            max: p.as_u64()?,
            ignore_types: parse_option_matchers(secondary, "ignoreTypes"),
            ignore_descendant: secondary_has_flag(secondary, "ignore", "descendant"),
            ignore_child: secondary_has_flag(secondary, "ignore", "child"),
            ignore_compounded: secondary_has_flag(secondary, "ignore", "compounded"),
            ignore_next_sibling: secondary_has_flag(secondary, "ignore", "next-sibling"),
            ignore_custom_elements: secondary_has_flag(secondary, "ignore", "custom-elements"),
        })
    }
}
fn in_anb_argument(raw: &str, position: usize) -> bool {
    let bytes = raw.as_bytes();
    for (open, _) in raw[..position].match_indices('(') {
        let prefix = &raw[..open];
        let name_start = prefix
            .bytes()
            .rposition(|b| !(b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
            .map_or(0, |i| i + 1);
        if !prefix[..name_start].ends_with(':')
            || !matches!(
                prefix[name_start..].to_ascii_lowercase().as_str(),
                "nth-child" | "nth-last-child" | "nth-of-type" | "nth-last-of-type"
            )
        {
            continue;
        }
        let mut depth = 0usize;
        let mut i = open + 1;
        while i <= position {
            if depth == 0
                && bytes[i..]
                    .get(..2)
                    .is_some_and(|word| word.eq_ignore_ascii_case(b"of"))
                && i > open + 1
                && bytes[i - 1].is_ascii_whitespace()
                && bytes.get(i + 2).is_some_and(u8::is_ascii_whitespace)
            {
                return position < i + 2;
            }
            match bytes[i] {
                b'(' => depth += 1,
                b')' if depth == 0 => break,
                b')' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        if i > position {
            return true;
        }
    }
    false
}
fn count_types(raw: &str, rule: &R) -> usize {
    let bytes = raw.as_bytes();
    let mut count = 0;
    let mut prev_sig = 0;
    let mut combinator = 0;
    let mut descendant = false;
    let mut i = 0;
    let mut attr = 0;
    let mut quote = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if quote != 0 {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if attr > 0 {
            if b == b'\'' || b == b'"' {
                quote = b;
            } else if b == b'[' {
                attr += 1;
            } else if b == b']' {
                attr -= 1;
            }
            i += 1;
            continue;
        }
        if b == b'[' {
            attr = 1;
            prev_sig = b;
            i += 1;
            continue;
        }
        if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        if b.is_ascii_whitespace() {
            let next = bytes[i..]
                .iter()
                .copied()
                .find(|byte| !byte.is_ascii_whitespace());
            if prev_sig != 0 && !matches!(next, Some(b'>' | b'+' | b'~' | b',')) {
                descendant = true;
            }
            i += 1;
            continue;
        }
        if b.is_ascii_alphabetic() || b == b'_' || b == b'-' || b >= 0x80 {
            let start = i;
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'-'
                    || bytes[i] == b'_'
                    || bytes[i] >= 0x80)
            {
                i += 1;
            }
            let name = &raw[start..i];
            let starts_type =
                matches!(prev_sig, 0 | b'>' | b'+' | b'~' | b',' | b'(' | b'|') || descendant;
            let compounded = !descendant && matches!(prev_sig, b'a' | b'.' | b'#' | b':');
            let after = bytes[i..]
                .iter()
                .copied()
                .find(|byte| !byte.is_ascii_whitespace());
            let compound_after = matches!(after, Some(b'.' | b'#' | b':'));
            let ignored = options_matches(&rule.ignore_types, name)
                || (rule.ignore_descendant && descendant)
                || (rule.ignore_child && combinator == b'>')
                || (rule.ignore_next_sibling && combinator == b'+')
                || (rule.ignore_compounded && (compounded || compound_after))
                || (rule.ignore_custom_elements && is_custom_element(name));
            if starts_type
                && bytes.get(i) != Some(&b'(')
                && bytes.get(i) != Some(&b'|')
                && !ignored
                && !in_anb_argument(raw, start)
            {
                count += 1;
            }
            prev_sig = b'a';
            combinator = 0;
            descendant = false;
        } else {
            if matches!(b, b'>' | b'+' | b'~') {
                combinator = b;
                descendant = false;
            } else if b == b',' {
                combinator = 0;
                descendant = false;
            }
            prev_sig = b;
            i += 1;
        }
    }
    count
}
impl Rule for R {
    fn name(&self) -> &'static str {
        "selector-max-type"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        for sel in &rule.selector.selectors {
            let raw = ctx.slice(sel.span);
            let s = raw.trim();
            if has_interpolation(s) {
                continue;
            }
            let c = count_types(s, self);
            if c as u64 > self.max {
                ctx.report(
                    self.name(),
                    sel.span,
                    format!("Too many type selectors in \"{}\", maximum {}", s, self.max),
                );
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
        let pass = vec![".a {}"];
        let fail = vec![reject_message(
            "a {}",
            None,
            "Too many type selectors in \"a\", maximum 0",
        )];
        Tester::new("selector-max-type", json!([0]), &pass, &fail).test();
    }

    #[test]
    fn ignores_interpolation_and_anb_keywords() {
        Tester::new(
            "selector-max-type",
            json!([0]),
            [
                ":nth-child(odd) {}",
                ":nth-child(2n + 1) {}",
                "#{$selector} {}",
            ],
            &[],
        )
        .with_custom_syntax("postcss-scss")
        .test();
        Tester::new(
            "selector-max-type",
            json!([0]),
            &[],
            [reject_message(
                ":nth-child(2n of div) {}",
                None,
                "Too many type selectors in \":nth-child(2n of div)\", maximum 0",
            )],
        )
        .test();
    }

    #[test]
    fn ignores_configured_selector_categories() {
        Tester::new(
            "selector-max-type",
            json!([1, {"ignore": ["descendant", "child"]}]),
            ["a b {}", "a > b {}"],
            &[],
        )
        .test();
        Tester::new(
            "selector-max-type",
            json!([0, {"ignore": ["compounded"]}]),
            ["a.foo {}"],
            &[],
        )
        .test();
        Tester::new(
            "selector-max-type",
            json!([0, {"ignore": ["custom-elements"]}]),
            ["my-element {}"],
            &[],
        )
        .test();
    }
}
