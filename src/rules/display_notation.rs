use oxc_css_parser::ast::Declaration;

use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        DISPLAY_INSIDE_KEYWORDS, DISPLAY_LIST_ITEM_KEYWORD, DISPLAY_OUTSIDE_KEYWORDS,
        ValueTokenKind, decl_value_span, ident_name, is_standard_syntax_value, tokenize_value,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Short,
    Full,
}

const SHORT_TO_LONG: &[(&str, &[&str])] = &[
    ("block list-item", &["block", "flow", "list-item"]),
    ("block", &["block", "flow"]),
    ("flex", &["block", "flex"]),
    ("flow list-item", &["block", "flow", "list-item"]),
    ("flow", &["block", "flow"]),
    ("flow-root", &["block", "flow-root"]),
    ("grid", &["block", "grid"]),
    ("inline list-item", &["inline", "flow", "list-item"]),
    ("inline", &["inline", "flow"]),
    ("inline-block", &["inline", "flow-root"]),
    ("inline-flex", &["inline", "flex"]),
    ("inline-grid", &["inline", "grid"]),
    ("inline-table", &["inline", "table"]),
    ("list-item", &["block", "flow", "list-item"]),
    ("ruby", &["inline", "ruby"]),
    ("run-in", &["run-in", "flow"]),
    ("table", &["block", "table"]),
];

const LONG_TO_SHORT: &[(&str, &[&str])] = &[
    ("block flex", &["flex"]),
    ("block flow list-item", &["list-item"]),
    ("block flow", &["block"]),
    ("block flow-root", &["flow-root"]),
    ("block grid", &["grid"]),
    ("block list-item", &["list-item"]),
    ("block table", &["table"]),
    ("flow list-item", &["list-item"]),
    ("inline flex", &["inline-flex"]),
    ("inline flow list-item", &["inline", "list-item"]),
    ("inline flow", &["inline"]),
    ("inline flow-root", &["inline-block"]),
    ("inline grid", &["inline-grid"]),
    ("inline ruby", &["ruby"]),
    ("inline table", &["inline-table"]),
    ("run-in flow", &["run-in"]),
];

pub fn expected_message(unexpected: &str, expected: &str) -> String {
    format!("Expected \"{unexpected}\" to be \"{expected}\"")
}

fn keyword_order(keyword: &str) -> u8 {
    if DISPLAY_OUTSIDE_KEYWORDS
        .iter()
        .any(|k| k.eq_ignore_ascii_case(keyword))
    {
        1
    } else if DISPLAY_INSIDE_KEYWORDS
        .iter()
        .any(|k| k.eq_ignore_ascii_case(keyword))
    {
        2
    } else if DISPLAY_LIST_ITEM_KEYWORD
        .iter()
        .any(|k| k.eq_ignore_ascii_case(keyword))
    {
        3
    } else {
        0
    }
}

crate::define_stx_lint! {
    /// Enforces short or expanded keyword notation for `display` values.
    ///
    /// Set the primary option to `"short"` or `"full"`. Auto-fix rewrites
    /// recognized equivalents and leaves other values, such as `none`, alone.
    ///
    /// ```css
    /// a { display: inline flex; } /* fixes to inline-flex with "short" */
    /// a { display: inline-flex; } /* fixes to inline flex with "full" */
    /// ```
    "display-notation", style, fix = fix;
    pub struct DisplayNotation {
        mode: DisplayMode,
    }
}

impl DisplayNotation {
    pub fn from_primary(primary: &serde_json::Value) -> Option<Self> {
        let mode = match primary.as_str()? {
            "short" => DisplayMode::Short,
            "full" => DisplayMode::Full,
            _ => return None,
        };
        Some(Self { mode })
    }
}

impl Rule for DisplayNotation {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if !prop.eq_ignore_ascii_case("display") {
            return;
        }
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        if !is_standard_syntax_value(ctx.slice(value_span)) {
            return;
        }

        let tokens = tokenize_value(ctx.source, value_span);
        let mut keywords = Vec::new();
        for token in &tokens {
            if token.kind != ValueTokenKind::Ident || !is_standard_syntax_value(token.raw) {
                return;
            }
            keywords.push(token);
        }
        if keywords.is_empty() {
            return;
        }

        let mut ordered = keywords.clone();
        ordered.sort_by_key(|token| keyword_order(token.raw));
        let normalized = ordered
            .iter()
            .map(|token| token.raw)
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();

        let table = match self.mode {
            DisplayMode::Short => LONG_TO_SHORT,
            DisplayMode::Full => SHORT_TO_LONG,
        };
        let Some(&(_, replacement)) = table.iter().find(|(key, _)| *key == normalized) else {
            return;
        };

        let first = keywords.first().unwrap();
        let last = keywords.last().unwrap();
        let span = oxc_css_parser::Span {
            start: first.span.start,
            end: last.span.end,
        };

        let mut fix_value = replacement.join(" ");
        fix_value.push_str(&comments_between(
            ctx.source,
            first.span.end,
            last.span.start,
        ));

        ctx.report_with_fix(
            self.name(),
            span,
            expected_message(&normalized, &replacement.join(" ")),
            Fix::single(span, fix_value),
        );
    }
}

fn comments_between(source: &str, start: u32, end: u32) -> String {
    let bytes = source.as_bytes();
    let mut i = start as usize;
    let end = end as usize;
    let mut out = String::new();
    while i + 1 < end {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let comment_start = i;
            i += 2;
            while i + 1 < end && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(end);
            out.push_str(&source[comment_start..i]);
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn short() {
        let pass = vec![
            "a { display: block; }",
            "a { display: run-in flow-root; }",
            "a { display: flow-root list-item; }",
            "a { display: ; }",
            "a { display: /* block */; }",
            "a { display: block var(--foo, flow); }",
            "a { display: none; }",
            "a { display: inline-block; }",
            "a { display: list-item; }",
        ];

        let fail = vec![
            reject(
                "a { display: block flow; }",
                "a { display: block; }",
                expected_message("block flow", "block"),
                (1, 14, 1, 24),
            ),
            reject(
                "a { display: inline flow-root; }",
                "a { display: inline-block; }",
                expected_message("inline flow-root", "inline-block"),
                (1, 14, 1, 30),
            ),
            reject(
                "a { display: block list-item; }",
                "a { display: list-item; }",
                expected_message("block list-item", "list-item"),
                (1, 14, 1, 29),
            ),
            reject(
                "a { display: flow list-item; }",
                "a { display: list-item; }",
                expected_message("flow list-item", "list-item"),
                (1, 14, 1, 28),
            ),
            reject(
                "a { display: /* foo */block/* bar */flow/* baz */; }",
                "a { display: /* foo */block/* bar *//* baz */; }",
                expected_message("block flow", "block"),
                (1, 23, 1, 41),
            ),
        ];

        Tester::new("display-notation", json!(["short"]), &pass, &fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn full() {
        let pass = vec![
            "a { display: block flow; }",
            "a { display: run-in flow-root; }",
            "a { display: block flow-root list-item; }",
            "a { display: none; }",
            "a { display: inline flow root; }",
            "a { display: block flow list-item; }",
        ];

        let fail = vec![
            reject(
                "a { display: block; }",
                "a { display: block flow; }",
                expected_message("block", "block flow"),
                (1, 14, 1, 19),
            ),
            reject(
                "a { display: inline-block; }",
                "a { display: inline flow-root; }",
                expected_message("inline-block", "inline flow-root"),
                (1, 14, 1, 26),
            ),
            reject(
                "a { display: list-item; }",
                "a { display: block flow list-item; }",
                expected_message("list-item", "block flow list-item"),
                (1, 14, 1, 23),
            ),
            reject(
                "a { display: flow list-item; }",
                "a { display: block flow list-item; }",
                expected_message("flow list-item", "block flow list-item"),
                (1, 14, 1, 28),
            ),
            reject(
                "a { display: /* foo */block/* bar */; }",
                "a { display: /* foo */block flow/* bar */; }",
                expected_message("block", "block flow"),
                (1, 23, 1, 28),
            ),
        ];

        Tester::new("display-notation", json!(["full"]), &pass, &fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn scss_syntax() {
        Tester::new(
            "display-notation",
            json!(["short"]),
            ["a { display: $block; }"],
            [],
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
