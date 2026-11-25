use crate::{
    context::LintContext,
    rules::Rule,
    utils::{ValueTokenKind, tokenize_value},
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

crate::define_stx_lint! {
    /// Require named grid areas to form filled rectangles.
    ///
    /// Enable with `true`; there are no rule-specific secondary options. In
    /// `grid-template-areas`, `grid-template`, and `grid`, each quoted row must
    /// contain at least one cell and have the same number of cells as every
    /// other row. Dot-only tokens are empty cells. Values with arrow-function
    /// syntax are skipped.
    ///
    /// Example: `a { grid-template-areas: "x x" "x y"; }` is reported because
    /// `x` forms an L shape rather than a rectangle.
    "named-grid-areas-no-invalid", correctness, recommended = true;
    pub struct NamedGridAreasNoInvalid;
}

impl Rule for NamedGridAreasNoInvalid {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let prop_end = if let InterpolableIdent::Literal(ident) = &decl.name {
            let name: &str = ident.name;
            if !name.eq_ignore_ascii_case("grid-template-areas")
                && !name.eq_ignore_ascii_case("grid-template")
                && !name.eq_ignore_ascii_case("grid")
            {
                return;
            }
            ident.span.end
        } else {
            return;
        };

        let value_tokens = tokenize_value(
            ctx.source,
            Span {
                start: prop_end,
                end: decl
                    .important
                    .as_ref()
                    .map_or(decl.span.end, |important| important.span.start),
            },
        );
        if value_tokens.windows(3).any(|tokens| {
            tokens[0].raw == ")"
                && tokens[1].raw == "="
                && tokens[2].raw == ">"
                && tokens[1].span.end == tokens[2].span.start
        }) {
            return;
        }

        let mut rows: Vec<(Vec<&str>, Span)> = Vec::new();
        let mut empty_row = false;
        for token in value_tokens {
            if token.kind != ValueTokenKind::String {
                continue;
            }
            let quote = token.raw.as_bytes()[0] as char;
            let content = &token.raw[1..];
            let cells: Vec<_> = content
                .strip_suffix(quote)
                .unwrap_or(content)
                .split_whitespace()
                .collect();
            if cells.is_empty() {
                ctx.report(self.name(), token.span, "Expected cell token within string");
                empty_row = true;
            } else {
                rows.push((cells, token.span));
            }
        }
        if empty_row {
            return;
        }

        let mut expected_cols: Option<usize> = None;
        for (tokens, span) in &rows {
            if let Some(cols) = expected_cols {
                if tokens.len() != cols {
                    ctx.report(
                        self.name(),
                        *span,
                        "Expected same number of cell tokens in each string".to_string(),
                    );
                    return;
                }
            } else {
                expected_cols = Some(tokens.len());
            }
        }

        let mut areas: Vec<(&str, Vec<(usize, usize)>)> = Vec::new();
        for (r_idx, (tokens, _)) in rows.iter().enumerate() {
            for (c_idx, &token) in tokens.iter().enumerate() {
                if token.chars().all(|c| c == '.') {
                    continue;
                }
                match areas.iter_mut().find(|(name, _)| *name == token) {
                    Some((_, coords)) => coords.push((r_idx, c_idx)),
                    None => areas.push((token, vec![(r_idx, c_idx)])),
                }
            }
        }

        areas.sort_by_key(|(name, _)| *name);
        for (name, coords) in areas {
            let min_r = coords.iter().map(|c| c.0).min().unwrap_or(0);
            let max_r = coords.iter().map(|c| c.0).max().unwrap_or(0);
            let min_c = coords.iter().map(|c| c.1).min().unwrap_or(0);
            let max_c = coords.iter().map(|c| c.1).max().unwrap_or(0);
            let expected_count = (max_r - min_r + 1) * (max_c - min_c + 1);

            if coords.len() != expected_count {
                ctx.report(
                    self.name(),
                    rows[coords[0].0].1,
                    format!("Expected single filled-in rectangle for \"{name}\""),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    fn rect(name: &str) -> String {
        format!("Expected single filled-in rectangle for \"{name}\"")
    }

    #[test]
    fn checks_grid_strings_but_not_comments_or_arrow_functions() {
        Tester::new(
            "named-grid-areas-no-invalid",
            json!(true),
            [
                r#"a { grid-template-areas: /* "" */ "head head" /* "a b c" */ "nav main"; }"#,
                r#"a { grid-template-areas /* : "" */: /* "" */ none; }"#,
                r#"a { grid-template-areas: ({ $foo }) => ($foo ? 'a' : 'b b'); }"#,
            ],
            [reject_warnings(
                r#"a { grid-template-areas: "a" "b b" "" ''; }"#,
                None,
                vec![
                    ("Expected cell token within string".into(), (1, 36, 1, 38)),
                    ("Expected cell token within string".into(), (1, 39, 1, 41)),
                ],
            )],
        )
        .test();
    }

    #[test]
    fn reports_on_first_string_with_area_in_name_order() {
        let fail = [
            reject(
                "a { grid-template-areas: \"a a a\" \"b b a\"; }",
                None,
                rect("a"),
                (1, 26, 1, 33),
            ),
            reject_warnings(
                "a { grid-template-areas: \"c b a\" \"a c a\"; }",
                None,
                vec![(rect("a"), (1, 26, 1, 33)), (rect("c"), (1, 26, 1, 33))],
            ),
            reject(
                "a { grid-template-areas: \"x y\" \"z y\" \"z x\"; }",
                None,
                rect("x"),
                (1, 26, 1, 31),
            ),
        ];
        Tester::new("named-grid-areas-no-invalid", json!(true), [], fail).test();
    }
}
