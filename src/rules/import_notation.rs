use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{ValueTokenKind, at_rule_prelude_span, tokenize_value},
};
use oxc_css_parser::{Span, ast::AtRule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportNotationMode {
    Url,
    String,
}

crate::define_stx_lint! {
    /// Enforces `url()` or quoted string notation for the target of `@import`.
    ///
    /// Set the primary option to `"url"` to wrap strings in `url()`, or
    /// `"string"` to unwrap nonempty `url()` arguments. Auto-fix preserves
    /// existing quotes, adds double quotes when unwrapping an unquoted path,
    /// and leaves trailing import conditions in place. Unwrapping uses the
    /// first closing parenthesis, so use care with paths containing `)`.
    ///
    /// ```css
    /// @import url(foo.css) screen; /* fixes to @import "foo.css" screen; with "string" */
    /// ```
    "import-notation", style, fix = fix;
    pub struct ImportNotation {
        pub mode: ImportNotationMode,
    }
}

impl Default for ImportNotation {
    fn default() -> Self {
        Self {
            mode: ImportNotationMode::Url,
        }
    }
}

impl Rule for ImportNotation {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if !name.eq_ignore_ascii_case("import") {
            return;
        }
        let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let tokens = tokenize_value(ctx.source, p_span);
        let Some(first) = tokens.first() else {
            return;
        };
        match self.mode {
            ImportNotationMode::Url => {
                if first.kind == ValueTokenKind::String {
                    let expected = format!("url({})", first.raw);
                    ctx.report_with_fix(
                        self.name(),
                        first.span,
                        format!("Expected \"{}\" to be \"{}\"", first.raw, expected),
                        Some(Fix::single(first.span, expected)),
                    );
                }
            }
            ImportNotationMode::String => {
                if first.kind == ValueTokenKind::Function && first.raw.eq_ignore_ascii_case("url(")
                {
                    let start = first.span.start as usize;
                    let end = (p_span.end as usize).min(ctx.source.len());
                    let slice = &ctx.source[start..end];
                    if let Some(close_rel) = slice.find(')') {
                        let url_span = Span {
                            start: first.span.start,
                            end: (start + close_rel + 1) as u32,
                        };
                        let raw_url = &slice[..close_rel + 1];
                        let inner = slice[4..close_rel].trim();
                        if !inner.is_empty() {
                            let expected = if inner.starts_with('"') || inner.starts_with('\'') {
                                inner.to_string()
                            } else {
                                format!("\"{}\"", inner)
                            };
                            ctx.report_with_fix(
                                self.name(),
                                url_span,
                                format!("Expected \"{}\" to be \"{}\"", raw_url, expected),
                                Some(Fix::single(url_span, expected)),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    const RULE_NAME: &str = "import-notation";

    fn expected(unfixed: &str, fixed: &str) -> String {
        format!("Expected \"{unfixed}\" to be \"{fixed}\"")
    }

    #[test]
    fn url_config() {
        let pass = [
            "@import url(\"foo.css\");",
            "@import uRl(\"foo.css\");",
            "@import URL(\"foo.css\");",
            "@import url( \"foo.css\" );",
            "@import url('foo.css');",
            "@import uRl('foo.css');",
            "@import URL('foo.css');",
            "@import url( 'foo.css' );",
            "@import url('foo.css') print;",
            "@import url('foo.css') print, screen;",
            "@import url(\"foo.css\") supports(display: flex) screen and (max-width: 400px);",
        ];
        let fail = [
            reject(
                "@import \"foo.css\";",
                "@import url(\"foo.css\");",
                expected("\"foo.css\"", "url(\"foo.css\")"),
                (1, 9, 1, 18),
            )
            .with_edit_info(0, (8, 17, "url(\"foo.css\")")),
            reject(
                "@import 'foo.css';",
                "@import url('foo.css');",
                expected("'foo.css'", "url('foo.css')"),
                (1, 9, 1, 18),
            )
            .with_edit_info(0, (8, 17, "url('foo.css')")),
            reject(
                "@import 'foo.css' print;",
                "@import url('foo.css') print;",
                expected("'foo.css'", "url('foo.css')"),
                (1, 9, 1, 18),
            )
            .with_edit_info(0, (8, 17, "url('foo.css')")),
            reject(
                "@import 'foo.css' supports(display: flex) screen and (max-width: 400px);",
                "@import url('foo.css') supports(display: flex) screen and (max-width: 400px);",
                expected("'foo.css'", "url('foo.css')"),
                (1, 9, 1, 18),
            )
            .with_edit_info(0, (8, 17, "url('foo.css')")),
            reject(
                "@import 'foo.css' /* comment */ print;",
                "@import url('foo.css') /* comment */ print;",
                expected("'foo.css'", "url('foo.css')"),
                (1, 9, 1, 18),
            )
            .with_edit_info(0, (8, 17, "url('foo.css')")),
        ];

        Tester::new(RULE_NAME, json!(["url"]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn string_config() {
        let pass = [
            "@import \"foo.css\";",
            "@import 'foo.css';",
            "@import 'foo.css' screen;",
            "@import 'foo.css' supports(display: flex) screen and (max-width: 400px);",
        ];
        let fail = [
            reject(
                "@import url(\"foo.css\");",
                "@import \"foo.css\";",
                expected("url(\"foo.css\")", "\"foo.css\""),
                (1, 9, 1, 23),
            )
            .with_edit_info(0, (8, 22, "\"foo.css\"")),
            reject(
                "@import uRl(\"foo.css\");",
                "@import \"foo.css\";",
                expected("uRl(\"foo.css\")", "\"foo.css\""),
                (1, 9, 1, 23),
            )
            .with_edit_info(0, (8, 22, "\"foo.css\"")),
            reject(
                "@import URL(\"foo.css\");",
                "@import \"foo.css\";",
                expected("URL(\"foo.css\")", "\"foo.css\""),
                (1, 9, 1, 23),
            )
            .with_edit_info(0, (8, 22, "\"foo.css\"")),
            reject(
                "@import url( \"foo.css\" );",
                "@import \"foo.css\";",
                expected("url( \"foo.css\" )", "\"foo.css\""),
                (1, 9, 1, 25),
            )
            .with_edit_info(0, (8, 24, "\"foo.css\"")),
            reject(
                "@import url('foo.css');",
                "@import 'foo.css';",
                expected("url('foo.css')", "'foo.css'"),
                (1, 9, 1, 23),
            )
            .with_edit_info(0, (8, 22, "'foo.css'")),
            reject(
                "@import url( 'foo.css' );",
                "@import 'foo.css';",
                expected("url( 'foo.css' )", "'foo.css'"),
                (1, 9, 1, 25),
            )
            .with_edit_info(0, (8, 24, "'foo.css'")),
            reject(
                "@import url(foo.css);",
                "@import \"foo.css\";",
                expected("url(foo.css)", "\"foo.css\""),
                (1, 9, 1, 21),
            )
            .with_edit_info(0, (8, 20, "\"foo.css\"")),
            reject(
                "@import url(foo.css) print;",
                "@import \"foo.css\" print;",
                expected("url(foo.css)", "\"foo.css\""),
                (1, 9, 1, 21),
            )
            .with_edit_info(0, (8, 20, "\"foo.css\"")),
            reject(
                "@import url('foo.css') supports(display: flex) screen and (max-width: 400px);",
                "@import 'foo.css' supports(display: flex) screen and (max-width: 400px);",
                expected("url('foo.css')", "'foo.css'"),
                (1, 9, 1, 23),
            )
            .with_edit_info(0, (8, 22, "'foo.css'")),
            reject(
                "@import url('foo.css' /* comment */) print;",
                "@import 'foo.css' /* comment */ print;",
                expected("url('foo.css' /* comment */)", "'foo.css' /* comment */"),
                (1, 9, 1, 37),
            )
            .with_edit_info(0, (8, 36, "'foo.css' /* comment */")),
        ];

        Tester::new(RULE_NAME, json!(["string"]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }
}
