use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::Stylesheet, token::CommentKind};

crate::define_stx_lint! {
    /// Report double-slash comments where CSS does not support them.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/no-invalid-double-slash-comments>
    ///
    /// Enable with `true`; there are no rule-specific options. The source scan
    /// looks for `//` at the start of input or after whitespace, `{`, or `;`.
    /// It skips strings, block comments, and parser-recognized line comments
    /// such as those in SCSS or Less.
    ///
    /// ```css
    /// // Use a block comment instead.
    /// a { color: red; }
    /// ```
    "no-invalid-double-slash-comments", correctness, recommended = true;
    pub struct NoInvalidDoubleSlashComments;
}

fn double_slash_spans(source: &str, line_comments: &[Span]) -> Vec<Span> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if line_comments.iter().any(|span| span.start as usize == i) {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes.get(i..i + 2) == Some(b"/*") {
            i += 2;
            while i + 1 < bytes.len() && bytes.get(i..i + 2) != Some(b"*/") {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == quote {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }
        if bytes.get(i..i + 2) == Some(b"//")
            && (i == 0 || bytes[i - 1].is_ascii_whitespace() || matches!(bytes[i - 1], b'{' | b';'))
        {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            spans.push(Span {
                start: start as u32,
                end: i as u32,
            });
            continue;
        }
        i += 1;
    }
    spans
}

impl Rule for NoInvalidDoubleSlashComments {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let line_comments: Vec<_> = ctx
            .comments
            .iter()
            .filter(|comment| comment.kind == CommentKind::Line)
            .map(|comment| comment.span)
            .collect();
        for span in double_slash_spans(ctx.source, &line_comments) {
            ctx.report(
                self.name(),
                span,
                "Invalid double-slash CSS comment".to_string(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::double_slash_spans;
    use crate::tester::Tester;
    use oxc_css_parser::Span;
    use serde_json::json;

    #[test]
    fn comment_placement_and_false_positives() {
        assert_eq!(
            double_slash_spans("// invalid selector", &[]),
            [Span { start: 0, end: 19 }]
        );
        assert_eq!(
            double_slash_spans("@media // invalid prelude", &[]),
            [Span { start: 7, end: 25 }]
        );
        assert!(double_slash_spans("a { background: url(//cdn.example/x); }", &[]).is_empty());
        assert!(double_slash_spans("// valid scss", &[Span { start: 0, end: 13 }]).is_empty());
        for syntax in ["postcss-scss", "postcss-less"] {
            Tester::new(
                "no-invalid-double-slash-comments",
                json!(true),
                ["// valid line comment"],
                [],
            )
            .with_custom_syntax(syntax)
            .test();
        }
    }
}
