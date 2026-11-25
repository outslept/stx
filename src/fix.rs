use oxc_css_parser::Span;

#[derive(Debug, Clone)]
pub struct Edit {
    pub span: Span,
    pub replacement: String,
}

#[derive(Debug, Clone)]
pub struct Fix {
    pub edits: Vec<Edit>,
}

impl Fix {
    pub fn single(span: Span, replacement: impl Into<String>) -> Self {
        Self {
            edits: vec![Edit {
                span,
                replacement: replacement.into(),
            }],
        }
    }
}

pub fn apply_fixes(source: &str, fixes: Vec<Fix>) -> (String, bool) {
    let mut edits: Vec<Edit> = fixes.into_iter().flat_map(|f| f.edits).collect();
    if edits.is_empty() {
        return (source.to_string(), false);
    }

    edits.sort_by(|a, b| {
        a.span
            .start
            .cmp(&b.span.start)
            .then_with(|| a.span.end.cmp(&b.span.end))
    });

    let mut out = String::with_capacity(source.len());
    let mut last_pos: u32 = 0;
    let mut applied = false;

    for edit in edits {
        if edit.span.start < last_pos {
            continue;
        }
        out.push_str(&source[last_pos as usize..edit.span.start as usize]);
        out.push_str(&edit.replacement);
        last_pos = edit.span.end;
        applied = true;
    }

    out.push_str(&source[last_pos as usize..]);
    (out, applied)
}

#[cfg(test)]
mod tests {
    use super::{Fix, Span, apply_fixes};
    use crate::{config::Config, lint_and_fix, rules::build_rules};
    use oxc_css_parser::Syntax;

    #[test]
    fn skips_overlapping_edits_but_applies_adjacent_ones() {
        let fixes = vec![
            Fix::single(Span { start: 4, end: 6 }, "Z"),
            Fix::single(Span { start: 2, end: 5 }, "skip"),
            Fix::single(Span { start: 1, end: 4 }, "XY"),
        ];
        assert_eq!(apply_fixes("abcdefg", fixes), ("aXYZg".into(), true));
    }

    #[test]
    fn uses_byte_spans_without_changing_crlf() {
        let source = "é\r\n😀x\r\n";
        let fixes = vec![
            Fix::single(Span { start: 4, end: 8 }, "界"),
            Fix::single(Span { start: 8, end: 9 }, ""),
            Fix::single(Span { start: 11, end: 11 }, "fin"),
        ];
        assert_eq!(apply_fixes(source, fixes), ("é\r\n界\r\nfin".into(), true));
        assert_eq!(apply_fixes(source, vec![]), (source.into(), false));
    }

    #[test]
    fn reports_remaining_warnings_at_positions_in_fixed_code() {
        let config = Config::from_json_str(
            r#"{"rules":{"color-named":"never","rule-empty-line-before":"always"}}"#,
        )
        .unwrap();
        let (fixed, pass) = lint_and_fix(
            "a {}\nb { color: red; }",
            Syntax::Css,
            &build_rules(&config),
            false,
        );
        assert_eq!(fixed, "a {}\n\nb { color: red; }");
        let warnings = pass.into_warnings();
        let [warning] = warnings.as_slice() else {
            panic!("Expected one warning, got {warnings:?}");
        };
        assert_eq!(warning.rule, "color-named");
        assert_eq!((warning.line, warning.column), (3, 12));
        assert_eq!((warning.end_line, warning.end_column), (3, 15));
    }
}
