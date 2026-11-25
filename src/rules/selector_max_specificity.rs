use oxc_css_parser::ast::QualifiedRule;

use super::selector_no_qualifying_type::resolve_nested_selectors;
use crate::{context::LintContext, rules::Rule};

fn specificity(raw: &str) -> (u64, u64, u64) {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut a = 0;
    let mut b = 0;
    let mut c = 0;
    let mut i = 0usize;
    let mut quote = 0u8;
    let mut attr = 0usize;
    while i < len {
        let ch = bytes[i];
        if quote != 0 {
            if ch == b'\\' {
                i += 2;
                continue;
            }
            if ch == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if attr > 0 {
            if ch == b'"' || ch == b'\'' {
                quote = ch;
            } else if ch == b'[' {
                attr += 1;
            } else if ch == b']' {
                attr -= 1;
                if attr == 0 {
                    b += 1;
                }
            }
            i += 1;
            continue;
        }
        if ch == b'[' {
            attr += 1;
            i += 1;
            continue;
        }
        if ch == b'#' {
            a += 1;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        if ch == b'.' {
            b += 1;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        if ch == b':' {
            let elem = i + 1 < len && bytes[i + 1] == b':';
            i += if elem { 2 } else { 1 };
            let ns = i;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            let name = raw[ns..i].to_ascii_lowercase();
            if elem
                || matches!(
                    name.as_str(),
                    "before" | "after" | "first-line" | "first-letter"
                )
            {
                c += 1;
            } else {
                b += 1;
            }
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == b'_' || ch == b'-' || ch >= 0x80 {
            let ns = i;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'-'
                    || bytes[i] == b'_'
                    || bytes[i] >= 0x80)
            {
                i += 1;
            }
            if i >= len || bytes[i] != b'(' {
                let _ = ns;
                c += 1;
            }
            continue;
        }
        i += 1;
    }
    (a, b, c)
}

pub struct SelectorMaxSpecificity {
    max: (u64, u64, u64),
    raw: String,
}
impl SelectorMaxSpecificity {
    pub fn from_config(p: &serde_json::Value) -> Option<Self> {
        let raw = p.as_str()?;
        let parts: Vec<u64> = raw
            .split(',')
            .map(|s| s.trim().parse().ok())
            .collect::<Option<_>>()?;
        if parts.len() != 3 {
            return None;
        }
        Some(Self {
            max: (parts[0], parts[1], parts[2]),
            raw: raw.to_string(),
        })
    }
}
impl Rule for SelectorMaxSpecificity {
    fn name(&self) -> &'static str {
        "selector-max-specificity"
    }
    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        let nested = ctx.ancestors.iter().any(|parent| {
            matches!(parent, crate::context::AstParent::QualifiedRule(_))
                || matches!(parent, crate::context::AstParent::AtRule(at_rule) if at_rule.name.name.eq_ignore_ascii_case("scope"))
        });
        for (sel, resolved) in rule
            .selector
            .selectors
            .iter()
            .zip(resolve_nested_selectors(rule, ctx))
        {
            let s = ctx.slice(sel.span).trim();
            if specificity(s) > self.max {
                let selector = if nested {
                    format!("\"{}\" (\"{}\")", s, resolved.trim())
                } else {
                    format!("\"{s}\"")
                };
                ctx.report(
                    self.name(),
                    sel.span,
                    format!(
                        "Too high specificity in {selector}, maximum \"{}\"",
                        self.raw
                    ),
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
        Tester::new(
            "selector-max-specificity",
            json!(["0,0,2"]),
            ["a {}", "a::before {}"],
            [],
        )
        .test();
        let fail = vec![
            reject_message(
                "#a {}",
                None,
                "Too high specificity in \"#a\", maximum \"0,0,2\"",
            ),
            reject_message(
                "article { & #a {} }",
                None,
                "Too high specificity in \"& #a\" (\"article #a\"), maximum \"0,0,2\"",
            ),
        ];
        Tester::new("selector-max-specificity", json!(["0,0,2"]), [], &fail).test();
        let comma_fail = [reject_message(
            ".a:not(.b, .c) {}",
            None,
            "Too high specificity in \".a:not(.b, .c)\", maximum \"0,1,0\"",
        )];
        Tester::new(
            "selector-max-specificity",
            json!(["0,1,0"]),
            [],
            &comma_fail,
        )
        .test();
    }
}
