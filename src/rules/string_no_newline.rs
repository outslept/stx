use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::Stylesheet};

pub struct StringNoNewline;

impl Rule for StringNoNewline {
    fn name(&self) -> &'static str {
        "string-no-newline"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let src = ctx.source;
        let bytes = src.as_bytes();
        let len = bytes.len();
        let mut i = 0usize;

        while i < len {
            if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
                continue;
            }

            if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }

            if bytes[i] == b'"' || bytes[i] == b'\'' {
                let quote = bytes[i];
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        if i + 1 < len
                            && bytes[i + 1] == b'\r'
                            && i + 2 < len
                            && bytes[i + 2] == b'\n'
                        {
                            i += 3;
                        } else {
                            i += 2;
                        }
                        continue;
                    }
                    if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    if bytes[i] == b'\n' || bytes[i] == b'\r' {
                        ctx.report(
                            self.name(),
                            Span {
                                start: start as u32,
                                end: i as u32,
                            },
                            "Newline in string".to_string(),
                        );
                        break;
                    }
                    i += 1;
                }
                continue;
            }

            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn newline_span() {
        Tester::new(
            "string-no-newline",
            json!(true),
            ["a { content: \"line\\Aline\"; }"],
            [reject(
                "a { content: \"broken\nvalue\"; }",
                None,
                "Newline in string",
                (1, 14, 1, 21),
            )],
        )
        .test();
    }
}
