use oxc_css_parser::{Span, ast::ComplexSelector};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, options_matches, parse_primary_option_matchers, selector_attribute_blocks,
    },
};

fn attribute_name(selector: &str, (start, end): (usize, usize)) -> Option<(usize, usize)> {
    let bytes = selector.as_bytes();
    let mut i = start;
    while i < end && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let name_start = i;
    while i < end {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        if b.is_ascii_whitespace() || matches!(b, b'~' | b'|' | b'^' | b'$' | b'*' | b'=') {
            break;
        }
        i += 1;
    }
    if i == name_start {
        None
    } else {
        Some((name_start, i.min(end)))
    }
}

pub fn rejected_message(name: &str) -> String {
    format!("Disallowed name \"{name}\"")
}

crate::define_stx_lint! {
    /// Disallows attribute selectors by attribute name.
    ///
    /// Set to a name or a list of names and `/regex/` patterns. Matches the name, not
    /// the operator or value. Does not fix violations.
    ///
    /// Example: `a[data-auto] {}` is disallowed by `/^data-/`.
    "selector-attribute-name-disallowed-list", restriction;
    pub struct SelectorAttributeNameDisallowedList {
        disallowed: Vec<OptionMatcher>,
    }
}

impl SelectorAttributeNameDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            disallowed: parse_primary_option_matchers(primary),
        }
    }
}

impl Rule for SelectorAttributeNameDisallowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(sel.span);
        for block in selector_attribute_blocks(raw) {
            let Some((start, end)) = attribute_name(raw, block) else {
                continue;
            };
            let name = &raw[start..end];
            if !options_matches(&self.disallowed, name) {
                continue;
            }
            ctx.report(
                self.name(),
                Span {
                    start: sel.span.start + start as u32,
                    end: sel.span.start + end as u32,
                },
                rejected_message(name),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = [
            "a[title] { }",
            "a[target=\"_blank\"] { }",
            "a[href$=\".pdf\"] { }",
            "div[lang~=\"en-us\"] { }",
        ];
        let fail = [
            reject(
                "[class*=\"foo\"] { }",
                None,
                "Disallowed name \"class\"",
                (1, 2, 1, 7),
            ),
            reject(
                "[ class*=\"foo\" ] { }",
                None,
                "Disallowed name \"class\"",
                (1, 3, 1, 8),
            ),
            reject(
                "[class *= \"foo\"] { }",
                None,
                "Disallowed name \"class\"",
                (1, 2, 1, 7),
            ),
            reject(
                "[disabled] { }",
                None,
                "Disallowed name \"disabled\"",
                (1, 2, 1, 10),
            ),
            reject(
                "[id~=\"bar\"] { }",
                None,
                "Disallowed name \"id\"",
                (1, 2, 1, 4),
            ),
            reject(
                "[data-foo*=\"bar\"] { }",
                None,
                "Disallowed name \"data-foo\"",
                (1, 2, 1, 10),
            ),
            reject(
                "[ data-foo *= \"bar\" ] { }",
                None,
                "Disallowed name \"data-foo\"",
                (1, 3, 1, 11),
            ),
            reject_warnings(
                "/* a comment */\ndiv[class*=\"foo\"], /* a comment */\ndiv[id*=\"foo\"],\n/* a comment */\ndiv[data-foo*=\"foo\"]\n{ color: red; }",
                None,
                vec![
                    ("Disallowed name \"class\"".into(), (2, 5, 2, 10)),
                    ("Disallowed name \"id\"".into(), (3, 5, 3, 7)),
                    ("Disallowed name \"data-foo\"".into(), (5, 5, 5, 13)),
                ],
            ),
        ];
        Tester::new(
            "selector-attribute-name-disallowed-list",
            json!(["class", "disabled", "id", "/^data-/"]),
            pass,
            fail,
        )
        .test();
        Tester::new(
            "selector-attribute-name-disallowed-list",
            json!("/^data-/"),
            ["a[title] { }"],
            [reject(
                "a[data-foo=\"bar\"] { }",
                None,
                "Disallowed name \"data-foo\"",
                (1, 3, 1, 11),
            )],
        )
        .test();
    }
}
