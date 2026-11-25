use oxc_css_parser::{Span, ast::Declaration};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, get_scheme_from_url, is_standard_syntax_url, options_matches,
        parse_primary_option_matchers, url_arguments,
    },
};

crate::define_stx_lint! {
    /// Disallows specified explicit schemes in declaration `url()` arguments.
    ///
    /// The primary option accepts a scheme, a `/regex/`, or an array of either.
    /// Match against the lowercase scheme without its colon. The rule recognizes
    /// `data:` and schemes followed by `://`; relative and scheme-relative URLs
    /// are not checked. This rule does not check `@import` targets.
    ///
    /// With `"http"`:
    ///
    /// ```css
    /// a { background: url(HTTP://example.com/file.jpg); } /* reported */
    /// ```
    "function-url-scheme-disallowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct FunctionUrlSchemeDisallowedList {
        disallowed: Vec<OptionMatcher>,
    }
}

impl FunctionUrlSchemeDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            disallowed: parse_primary_option_matchers(primary),
        }
    }
}

pub fn rejected_message(scheme: &str) -> String {
    format!("Disallowed URL scheme \"{scheme}:\"")
}

fn strip_quotes(value: &str) -> &str {
    value
        .trim_start_matches(['\'', '"'])
        .trim_end_matches(['\'', '"'])
}

impl Rule for FunctionUrlSchemeDisallowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(decl.span);
        for (start, end) in url_arguments(raw) {
            let unspaced = raw[start..end].trim();
            if !is_standard_syntax_url(unspaced) {
                continue;
            }
            let Some(scheme) = get_scheme_from_url(strip_quotes(unspaced)) else {
                continue;
            };
            if !options_matches(&self.disallowed, &scheme) {
                continue;
            }
            ctx.report(
                self.name(),
                Span {
                    start: decl.span.start + start as u32,
                    end: decl.span.start + end as u32,
                },
                rejected_message(&scheme),
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
    fn empty_config() {
        Tester::new(
            "function-url-scheme-disallowed-list",
            json!([]),
            ["a { background: url('http://www.example.com/file.jpg'); }"],
            [],
        )
        .test();
        Tester::new(
            "function-url-scheme-disallowed-list",
            json!(""),
            ["a { background: url('http://www.example.com/file.jpg'); }"],
            [],
        )
        .test();
    }

    #[test]
    fn default_config() {
        let pass = vec![
            "a { background: url(); }",
            "a { background: url(''); }",
            "a { background: url(\"\"); }",
            "a { background: url(:); }",
            "a { background: url(://); }",
            "a { background: url(//); }",
            "a { background: url(/); }",
            "a { background: url(./); }",
            "a { background: url(./file.jpg); }",
            "a { background: url(../file.jpg); }",
            "a { background: URL(../file.jpg); }",
            "a { background: url('../file.jpg'); }",
            "a { background: url(\"../file.jpg\"); }",
            "a { background: url('/path/to/file.jpg'); }",
            "a { background: url(//www.example.com/file.jpg); }",
            "a { background: url(\"//www.example.com/file.jpg\"); }",
            "a { background: url('http://www.example.com/file.jpg'); }",
            "a { background-image: url('http://example.com:3000'); }",
            "a { background-image: url('//example.com:3000'); }",
            "@font-face { font-family: 'foo'; src: url('/path/to/foo.ttf'); }",
            "a { background: url(HTTP://example.com/file.jpg); }",
            "a { background: some-url(); }",
            "a { background: url($image); }",
            "a { background: url(@image); }",
            "a { background: url(http://#{$host}/path); }",
            "a { background: url('http://@{host}/path'); }",
            "a { background: url(http://$(host)/path); }",
            "a { background: url(var(--image)); }",
            "a { background: url(example.com); }",
            "a { background: url(example.com:3000); }",
            "a { background: url(http://example.com:3000); }",
        ];

        let fail = vec![
            reject(
                "a { background-image: url('data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs='); }",
                None,
                rejected_message("data"),
                (1, 27, 1, 99),
            ),
            reject(
                "a { background: url(HTTPS://www.example.com/file.jpg); }",
                None,
                rejected_message("https"),
                (1, 21, 1, 53),
            ),
            reject(
                "a { background: url('https://www.example.com/file.jpg'); }",
                None,
                rejected_message("https"),
                (1, 21, 1, 55),
            ),
            reject(
                "a { background: url(\"https://www.example.com/file.jpg\"); }",
                None,
                rejected_message("https"),
                (1, 21, 1, 55),
            ),
            reject(
                "a { background: url('https://example.com:3000'); }",
                None,
                rejected_message("https"),
                (1, 21, 1, 47),
            ),
            reject(
                "@font-face { font-family: 'foo'; src: url('https://www.example.com/file.jpg'); }",
                None,
                rejected_message("https"),
                (1, 43, 1, 77),
            ),
            reject(
                "a { background: no-repeat center/80% url('https://www.example.com/file.jpg'); }",
                None,
                rejected_message("https"),
                (1, 42, 1, 76),
            ),
        ];

        Tester::new(
            "function-url-scheme-disallowed-list",
            json!(["https", "data"]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn regex_config() {
        let pass = vec![
            "a { background-image: url('data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs='); }",
            "a { background: url(./file.jpg); }",
        ];
        let fail = vec![
            reject(
                "a { background: url(https://example.com/file.jpg); }",
                None,
                rejected_message("https"),
                (1, 21, 1, 49),
            ),
            reject(
                "a { background: url(HTTPS://example.com/file.jpg); }",
                None,
                rejected_message("https"),
                (1, 21, 1, 49),
            ),
            reject(
                "a { background: url(http://example.com/file.jpg); }",
                None,
                rejected_message("http"),
                (1, 21, 1, 48),
            ),
        ];
        Tester::new(
            "function-url-scheme-disallowed-list",
            json!(["/^http/"]),
            &pass,
            &fail,
        )
        .test();
    }
}
