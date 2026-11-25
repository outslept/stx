use oxc_css_parser::{
    Span,
    ast::{AtRule, Declaration},
};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{is_standard_syntax_url, url_arguments},
};

pub fn rejected_message() -> String {
    "Disallowed scheme-relative url".to_string()
}

fn strip_quotes(value: &str) -> &str {
    value
        .trim_start_matches(['\'', '"'])
        .trim_end_matches(['\'', '"'])
}

fn check<'a>(raw: &'a str, base: u32, ctx: &mut LintContext<'a>) {
    for (start, end) in url_arguments(raw) {
        let url = strip_quotes(raw[start..end].trim());
        if !is_standard_syntax_url(url) || !url.starts_with("//") {
            continue;
        }
        ctx.report(
            "function-url-no-scheme-relative",
            Span {
                start: base + start as u32,
                end: base + end as u32,
            },
            rejected_message(),
        );
    }
}

crate::define_stx_lint! {
    /// Disallows scheme-relative `url()` arguments in declarations and `@import`.
    ///
    /// Set the primary option to `true`. This checks for a leading `//` with
    /// or without quotes. Path-relative URLs, URLs with an explicit scheme,
    /// and bare string `@import` targets are not checked.
    ///
    /// ```css
    /// @import url('//www.example.org/foo.css'); /* reported */
    /// @import url('../foo.css'); /* passes */
    /// ```
    "function-url-no-scheme-relative", restriction;
    pub struct FunctionUrlNoSchemeRelative;
}

impl Rule for FunctionUrlNoSchemeRelative {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(decl.span);
        check(raw, decl.span.start, ctx);
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("import") {
            return;
        }
        let raw = ctx.slice(at_rule.span);
        check(raw, at_rule.span.start, ctx);
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
            "a { background: url(); }",
            "a { background: url(''); }",
            "a { background: url(\"\"); }",
            "a { background: url(/); }",
            "a { background: url(./); }",
            "a { background: url(./file.jpg); }",
            "a { background: url(../file.jpg); }",
            "a { background: URL(../file.jpg); }",
            "a { background: url('../file.jpg'); }",
            "a { background: url(\"../file.jpg\"); }",
            "a { background: url('/path/to/file.jpg'); }",
            "a { background: url('https://www.example.org/file.jpg'); }",
            "a { background: url('http://www.example.org/file.jpg'); }",
            "a { background-image: url('data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs='); }",
            "@font-face { font-family: 'foo'; src: url('/path/to/foo.ttf'); }",
            "a { background: some-url(); }",
            "a { background: url($image); }",
            "a { background: url(@image); }",
            "a { background: url(var(--image)); }",
            "@import url('../foo.css');",
            "@import url(\"../foo.css\");",
            "@import url('/path/to/foo.css');",
            "@import url('https://www.example.org/foo.css');",
            "@import url('http://www.example.org/foo.css');",
        ];

        let fail = vec![
            reject(
                "a { background: url(//); }",
                None,
                rejected_message(),
                (1, 21, 1, 23),
            ),
            reject(
                "a { background: url(//www.example.org/file.jpg); }",
                None,
                rejected_message(),
                (1, 21, 1, 47),
            ),
            reject(
                "a { background: url('//www.example.org/file.jpg'); }",
                None,
                rejected_message(),
                (1, 21, 1, 49),
            ),
            reject(
                "a { background: url(\"//www.example.org/file.jpg\"); }",
                None,
                rejected_message(),
                (1, 21, 1, 49),
            ),
            reject(
                "a { background: url( \"//www.example.org/file.jpg\" ); }",
                None,
                rejected_message(),
                (1, 21, 1, 51),
            ),
            reject(
                "@font-face { font-family: 'foo'; src: url('//www.example.org/file.jpg'); }",
                None,
                rejected_message(),
                (1, 43, 1, 71),
            ),
            reject(
                "a { background: no-repeat center/80% url('//www.example.org/file.jpg'); }",
                None,
                rejected_message(),
                (1, 42, 1, 70),
            ),
            reject(
                "@import url('//www.example.org/foo.css')",
                None,
                rejected_message(),
                (1, 13, 1, 40),
            ),
            reject(
                "@import url(\"//www.example.org/foo.css\") screen and (min-width: 768px);",
                None,
                rejected_message(),
                (1, 13, 1, 40),
            ),
        ];

        Tester::new("function-url-no-scheme-relative", json!(true), &pass, &fail).test();
    }
}
