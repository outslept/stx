use oxc_css_parser::{Span, ast::Declaration};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, get_scheme_from_url, is_standard_syntax_url, options_matches,
        parse_primary_option_matchers, url_arguments,
    },
};

#[derive(Debug, Default)]
pub struct FunctionUrlSchemeAllowedList {
    allowed: Vec<OptionMatcher>,
}

impl FunctionUrlSchemeAllowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            allowed: parse_primary_option_matchers(primary),
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

impl Rule for FunctionUrlSchemeAllowedList {
    fn name(&self) -> &'static str {
        "function-url-scheme-allowed-list"
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
            if options_matches(&self.allowed, &scheme) {
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
            "a { background: url('https://www.example.com/file.jpg'); }",
            "a { background-image: url('data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs='); }",
            "a { background-image: url('https://example.com:3000'); }",
            "a { background-image: url('//example.com:3000'); }",
            "@font-face { font-family: 'foo'; src: url('/path/to/foo.ttf'); }",
            "a { background: url(HTTPS://example.com/file.jpg); }",
            "a { background: some-url(); }",
            "a { background: url($image); }",
            "a { background: url(@image); }",
            "a { background: url(http://#{$host}/path); }",
            "a { background: url('http://@{host}/path'); }",
            "a { background: url(http://$(host)/path); }",
            "a { background: url(var(--image)); }",
            "a { background: url(example.com); }",
            "a { background: url(example.com:3000); }",
            "a { background: url(https://example.com:3000); }",
        ];

        let fail = vec![
            reject(
                "a { background: url(http://www.example.com/file.jpg); }",
                None,
                rejected_message("http"),
                (1, 21, 1, 52),
            ),
            reject(
                "a { background: url('http://www.example.com/file.jpg'); }",
                None,
                rejected_message("http"),
                (1, 21, 1, 54),
            ),
            reject(
                "a { background: url(\"http://www.example.com/file.jpg\"); }",
                None,
                rejected_message("http"),
                (1, 21, 1, 54),
            ),
            reject(
                "a { background: url('http://example.com:3000'); }",
                None,
                rejected_message("http"),
                (1, 21, 1, 46),
            ),
            reject(
                "@font-face { font-family: 'foo'; src: url('http://www.example.com/file.jpg'); }",
                None,
                rejected_message("http"),
                (1, 43, 1, 76),
            ),
            reject(
                "a { background: no-repeat center/80% url('http://www.example.com/file.jpg'); }",
                None,
                rejected_message("http"),
                (1, 42, 1, 75),
            ),
        ];

        Tester::new(
            "function-url-scheme-allowed-list",
            json!(["https", "data"]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn empty_config() {
        let pass = vec![
            "a { background: url('/path/to/file.jpg'); }",
            "a { background: url(//www.example.com/file.jpg); }",
            "a { background: url(\"//www.example.com/file.jpg\"); }",
            "a { background: url(example.com:3000); }",
        ];
        let fail = vec![
            reject(
                "a { background: url('https://www.example.com/file.jpg'); }",
                None,
                rejected_message("https"),
                (1, 21, 1, 55),
            ),
            reject(
                "a { background-image: url('data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs='); }",
                None,
                rejected_message("data"),
                (1, 27, 1, 99),
            ),
        ];
        Tester::new("function-url-scheme-allowed-list", json!([]), &pass, &fail).test();
        Tester::new("function-url-scheme-allowed-list", json!(""), &pass, &fail).test();
    }

    #[test]
    fn regex_and_extra_schemes() {
        let fail = vec![reject(
            "a { background: url(http://example.com/file.jpg); }",
            None,
            rejected_message("http"),
            (1, 21, 1, 48),
        )];
        Tester::new(
            "function-url-scheme-allowed-list",
            json!(["uri", "file", "https"]),
            ["a { background: url(https://example.com/file.jpg); }"],
            &fail,
        )
        .test();

        let pass = vec![
            "a { background: url(http://example.com/file.jpg); }",
            "a { background: url(HTTP://example.com/file.jpg); }",
            "a { background: url(https://example.com/file.jpg); }",
        ];
        let fail = vec![
            reject(
                "a { background: url(ftp://example.com/file.jpg); }",
                None,
                rejected_message("ftp"),
                (1, 21, 1, 47),
            ),
            reject(
                "a { background-image: url('data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs='); }",
                None,
                rejected_message("data"),
                (1, 27, 1, 99),
            ),
        ];
        Tester::new(
            "function-url-scheme-allowed-list",
            json!(["/^http/"]),
            &pass,
            &fail,
        )
        .test();
    }
}
