use oxc_css_parser::{Span, ast::ComplexSelector};

use crate::{context::LintContext, rules::Rule, utils::selector_attribute_blocks};

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

fn operator(selector: &str, (start, end): (usize, usize)) -> Option<(usize, usize)> {
    let bytes = selector.as_bytes();
    let mut i = start;
    let mut quote = 0u8;
    while i < end {
        let b = bytes[i];
        if quote != 0 {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                quote = 0;
            }
        } else if b == b'"' || b == b'\'' {
            quote = b;
        } else if b == b'=' {
            let op_start = if i > start && matches!(bytes[i - 1], b'~' | b'|' | b'^' | b'$' | b'*')
            {
                i - 1
            } else {
                i
            };
            return Some((op_start, i + 1));
        }
        i += 1;
    }
    None
}

pub fn rejected_message(operator: &str) -> String {
    format!("Disallowed operator \"{operator}\"")
}

crate::define_stx_lint! {
    /// Disallows listed operators in attribute selectors.
    ///
    /// Set to a literal operator or a list of operators, such as `["*=", "~="]`.
    /// Attribute presence selectors have no operator and are not checked. Does not fix violations.
    ///
    /// Example: `[class*="foo"] {}` is disallowed when `*=` is listed.
    "selector-attribute-operator-disallowed-list", restriction;
    pub struct SelectorAttributeOperatorDisallowedList {
        disallowed: Vec<String>,
    }
}

impl SelectorAttributeOperatorDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            disallowed: strings_from(primary),
        }
    }
}

impl Rule for SelectorAttributeOperatorDisallowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(sel.span);
        for block in selector_attribute_blocks(raw) {
            let Some((start, end)) = operator(raw, block) else {
                continue;
            };
            let value = &raw[start..end];
            if !self.disallowed.iter().any(|o| o == value) {
                continue;
            }
            ctx.report(
                self.name(),
                Span {
                    start: sel.span.start + start as u32,
                    end: sel.span.start + end as u32,
                },
                rejected_message(value),
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
            "a[target] { }",
            "a[target=\"_blank\"] { }",
            "[class|=\"top\"] { }",
            "[class^=top] { }",
            "[class$=\"test\"] { }",
        ];
        let fail = vec![
            reject(
                "[title~=\"flower\"] { }",
                None,
                rejected_message("~="),
                (1, 7, 1, 9),
            ),
            reject(
                "[ title~=\"flower\" ] { }",
                None,
                rejected_message("~="),
                (1, 8, 1, 10),
            ),
            reject(
                "[title ~= \"flower\"] { }",
                None,
                rejected_message("~="),
                (1, 8, 1, 10),
            ),
            reject(
                "[class*=test] { }",
                None,
                rejected_message("*="),
                (1, 7, 1, 9),
            ),
        ];
        Tester::new(
            "selector-attribute-operator-disallowed-list",
            json!(["*=", "~="]),
            &pass,
            &fail,
        )
        .test();
        Tester::new(
            "selector-attribute-operator-disallowed-list",
            json!("*="),
            ["a[target=\"_blank\"] { }"],
            [reject(
                "[title*=\"foo\"] { }",
                None,
                rejected_message("*="),
                (1, 7, 1, 9),
            )],
        )
        .test();
    }
}
