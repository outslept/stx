use crate::{
    context::LintContext,
    rules::Rule,
    utils::{options_matches, parse_option_matchers},
};
use oxc_css_parser::{Span, Syntax, ast::Stylesheet};
use serde_json::{Map, Value};

pub struct StringNoNewline {
    ignore_preludes: bool,
    ignore_values: bool,
}

impl StringNoNewline {
    pub fn from_config(secondary: Option<&Map<String, Value>>) -> Self {
        let ignore = parse_option_matchers(secondary, "ignore");
        Self {
            ignore_preludes: options_matches(&ignore, "at-rule-preludes"),
            ignore_values: options_matches(&ignore, "declaration-values"),
        }
    }
}

impl Rule for StringNoNewline {
    fn name(&self) -> &'static str {
        "string-no-newline"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let bytes = ctx.source.as_bytes();
        if !bytes.contains(&b'\n') {
            return;
        }
        let len = bytes.len();
        let mut i = 0usize;
        let mut head = None;
        let mut colon = false;
        let mut brackets = 0usize;
        let mut value_braces = 0usize;
        let mut pending = Vec::new();

        while i <= len {
            let byte = bytes.get(i).copied().unwrap_or(b';');
            if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
                continue;
            }

            if ctx.syntax != Syntax::Css
                && brackets == 0
                && byte == b'/'
                && bytes.get(i + 1) == Some(&b'/')
            {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }

            if brackets == 0 && colon {
                if byte == b'{'
                    && (value_braces > 0
                        || head.is_some_and(|start| bytes[start..].starts_with(b"--")))
                {
                    value_braces += 1;
                    i += 1;
                    continue;
                }
                if byte == b'}' && value_braces > 0 {
                    value_braces -= 1;
                    i += 1;
                    continue;
                }
            }
            if brackets == 0
                && value_braces == 0
                && (matches!(byte, b'{' | b'}' | b';')
                    || (ctx.syntax == Syntax::Sass && byte == b'\n'))
            {
                if !colon || byte == b'{' {
                    for span in pending.drain(..) {
                        ctx.report(self.name(), span, "Newline in string");
                    }
                } else {
                    pending.clear();
                }
                head = None;
                colon = false;
                i += 1;
                continue;
            }
            if !byte.is_ascii_whitespace() {
                head.get_or_insert(i);
            }
            match byte {
                b'(' | b'[' => brackets += 1,
                b')' | b']' => brackets = brackets.saturating_sub(1),
                b':' if brackets == 0 => colon = true,
                b'\\' => {
                    i += 2;
                    continue;
                }
                _ => {}
            }

            if matches!(byte, b'"' | b'\'') {
                let quote = byte;
                let start = i;
                let mut reported = false;
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
                    if !reported && matches!(bytes[i], b'\n' | b'\r') {
                        let span = Span {
                            start: start as u32,
                            end: i as u32,
                        };
                        if head.is_some_and(|start| bytes[start] == b'@') {
                            if !self.ignore_preludes {
                                ctx.report(self.name(), span, "Newline in string");
                            }
                        } else if self.ignore_values && colon {
                            pending.push(span);
                        } else {
                            ctx.report(self.name(), span, "Newline in string");
                        }
                        reported = true;
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
    use crate::{
        config::Config,
        lint_once,
        rules::build_rules,
        tester::{Tester, reject},
    };
    use oxc_css_parser::Syntax;
    use serde_json::json;

    #[test]
    fn closing_quote_does_not_start_another_warning() {
        let config = Config::from_json_str(r#"{"rules":{"string-no-newline":true}}"#).unwrap();
        let rules = build_rules(&config);
        let pass = lint_once(
            "a::before {\n\tcontent: 'foo\n\tbar';\n\tcontent: 'escaped\\\r\nline';\n\tcontent: 'another\r\nbreak';\n}",
            Syntax::Css,
            &rules,
            false,
        );
        let spans: Vec<_> = pass
            .warnings
            .iter()
            .map(|w| (w.line, w.column, w.end_line, w.end_column))
            .collect();
        assert_eq!(spans, [(2, 11, 2, 15), (6, 11, 6, 19)]);
    }

    #[test]
    fn ignore_is_scoped_to_the_requested_field() {
        let code = "@import 'foo\r\n.css';\na:is([href='one\ntwo']) { content: 'foo\nbar'; }";
        for (ignore, expected) in [
            ("at-rule-preludes", [(3, 12, 3, 16), (4, 19, 4, 23)]),
            ("declaration-values", [(1, 9, 1, 13), (3, 12, 3, 16)]),
        ] {
            let config = Config::from_json_str(
                &json!({"rules": {"string-no-newline": [true, {"ignore": [ignore]}]}}).to_string(),
            )
            .unwrap();
            let pass = lint_once(code, Syntax::Css, &build_rules(&config), false);
            let spans: Vec<_> = pass
                .warnings
                .iter()
                .map(|w| (w.line, w.column, w.end_line, w.end_column))
                .collect();
            assert_eq!(spans, expected);
        }
        Tester::new(
            "string-no-newline",
            json!([true, {"ignore": ["declaration-values"]}]),
            ["a { --foo: { 'one\ntwo' }; content: 'foo\nbar'; }"],
            [],
        )
        .test();
    }

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
