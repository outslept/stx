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
