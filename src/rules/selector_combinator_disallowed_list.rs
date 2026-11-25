use oxc_css_parser::{Span, ast::ComplexSelector};

use crate::{context::LintContext, rules::Rule, utils::selector_combinators};

fn strings_from(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

pub fn rejected_message(combinator: &str) -> String {
    format!("Disallowed combinator \"{combinator}\"")
}

pub struct SelectorCombinatorDisallowedList {
    disallowed: Vec<String>,
}

impl SelectorCombinatorDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            disallowed: strings_from(primary),
        }
    }
}

impl Rule for SelectorCombinatorDisallowedList {
    fn name(&self) -> &'static str {
        "selector-combinator-disallowed-list"
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(sel.span);
        for (start, end, value) in selector_combinators(raw) {
            if !self.disallowed.iter().any(|c| *c == value) {
                continue;
            }
            ctx.report(
                self.name(),
                Span {
                    start: sel.span.start + start as u32,
                    end: sel.span.start + end as u32,
                },
                rejected_message(&value),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "a {}",
            "a, b {}",
            "a /for/ b {}",
            "a + b {}",
            "a:not(b ~ c) {}",
        ];
        let fail = vec![
            reject("a b {}", None, rejected_message(" "), (1, 2, 1, 3)),
            reject("a\tb {}", None, rejected_message(" "), (1, 2, 1, 3)),
            reject("a\n\tb {}", None, rejected_message(" "), (1, 2, 2, 2)),
            reject("a,\nb c {}", None, rejected_message(" "), (2, 2, 2, 3)),
            reject(
                "a:not(b > c) {}",
                None,
                rejected_message(">"),
                (1, 9, 1, 10),
            ),
            reject("a > b {}", None, rejected_message(">"), (1, 3, 1, 4)),
        ];
        Tester::new(
            "selector-combinator-disallowed-list",
            json!([">", " "]),
            &pass,
            &fail,
        )
        .test();
        Tester::new(
            "selector-combinator-disallowed-list",
            json!(">"),
            ["a {}"],
            [reject(
                "a > b {}",
                None,
                rejected_message(">"),
                (1, 3, 1, 4),
            )],
        )
        .test();
    }
}
