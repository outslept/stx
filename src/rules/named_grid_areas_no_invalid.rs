use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

pub struct NamedGridAreasNoInvalid;

impl Rule for NamedGridAreasNoInvalid {
    fn name(&self) -> &'static str {
        "named-grid-areas-no-invalid"
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
            ident.span.end as usize
        } else {
            return;
        };

        let end = decl.span.end as usize;
        let src = ctx.source;
        if prop_end >= end || end > src.len() {
            return;
        }

        let bytes = src.as_bytes();
        let mut i = prop_end;
        let mut rows: Vec<(Vec<String>, Span)> = Vec::new();

        while i < end {
            if bytes[i] == b'"' || bytes[i] == b'\'' {
                let quote = bytes[i];
                let start = i;
                i += 1;
                while i < end && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    i += 1;
                }
                let content = &src[start + 1..i.min(end)];
                if i < end {
                    i += 1;
                }
                let span = Span {
                    start: start as u32,
                    end: i as u32,
                };
                let tokens: Vec<String> =
                    content.split_whitespace().map(|s| s.to_string()).collect();
                rows.push((tokens, span));
                continue;
            }
            i += 1;
        }

        if rows.is_empty() {
            return;
        }

        let mut expected_cols: Option<usize> = None;
        for (tokens, span) in &rows {
            if tokens.is_empty() {
                ctx.report(
                    self.name(),
                    *span,
                    "Expected cell token within string".to_string(),
                );
                return;
            }
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
            for (c_idx, token) in tokens.iter().enumerate() {
                if token.chars().all(|c| c == '.') {
                    continue;
                }
                match areas.iter_mut().find(|(name, _)| *name == token) {
                    Some((_, coords)) => coords.push((r_idx, c_idx)),
                    None => areas.push((token.as_str(), vec![(r_idx, c_idx)])),
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
