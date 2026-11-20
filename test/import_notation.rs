const RULE_NAME: &str = "import-notation";

fn expected(unfixed: &str, fixed: &str) -> String {
    format!("Expected \"{unfixed}\" to be \"{fixed}\"")
}

test_rule! {
    name: url_config,
    ruleName: RULE_NAME,
    config: ["url"],
    fix: true,
    accept: [
        {
            code: "@import url(\"foo.css\");",
        },
        {
            code: "@import uRl(\"foo.css\");",
        },
        {
            code: "@import URL(\"foo.css\");",
        },
        {
            code: "@import url( \"foo.css\" );",
        },
        {
            code: "@import url('foo.css');",
        },
        {
            code: "@import uRl('foo.css');",
        },
        {
            code: "@import URL('foo.css');",
        },
        {
            code: "@import url( 'foo.css' );",
        },
        {
            code: "@import url('foo.css') print;",
        },
        {
            code: "@import url('foo.css') print, screen;",
        },
        {
            code: "@import url(\"foo.css\") supports(display: flex) screen and (max-width: 400px);",
        },
    ],
    reject: [
        {
            code: "@import \"foo.css\";",
            fixed: "@import url(\"foo.css\");",
            fix: {
                range: [8, 17],
                text: "url(\"foo.css\")",
            },
            message: expected("\"foo.css\"", "url(\"foo.css\")"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: "@import 'foo.css';",
            fixed: "@import url('foo.css');",
            fix: {
                range: [8, 17],
                text: "url('foo.css')",
            },
            message: expected("'foo.css'", "url('foo.css')"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: "@import 'foo.css' print;",
            fixed: "@import url('foo.css') print;",
            fix: {
                range: [8, 17],
                text: "url('foo.css')",
            },
            message: expected("'foo.css'", "url('foo.css')"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: "@import 'foo.css' supports(display: flex) screen and (max-width: 400px);",
            fixed: "@import url('foo.css') supports(display: flex) screen and (max-width: 400px);",
            fix: {
                range: [8, 17],
                text: "url('foo.css')",
            },
            message: expected("'foo.css'", "url('foo.css')"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: "@import 'foo.css' /* comment */ print;",
            fixed: "@import url('foo.css') /* comment */ print;",
            fix: {
                range: [8, 17],
                text: "url('foo.css')",
            },
            description: "comment after the string",
            message: expected("'foo.css'", "url('foo.css')"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 18,
        },
    ],
}

test_rule! {
    name: string_config,
    ruleName: RULE_NAME,
    config: ["string"],
    fix: true,
    accept: [
        {
            code: "@import \"foo.css\";",
        },
        {
            code: "@import 'foo.css';",
        },
        {
            code: "@import 'foo.css' screen;",
        },
        {
            code: "@import 'foo.css' supports(display: flex) screen and (max-width: 400px);",
        },
    ],
    reject: [
        {
            code: "@import url(\"foo.css\");",
            fixed: "@import \"foo.css\";",
            message: expected("url(\"foo.css\")", "\"foo.css\""),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 23,
            fix: {
                range: [8, 22],
                text: "\"foo.css\"",
            },
        },
        {
            code: "@import uRl(\"foo.css\");",
            fixed: "@import \"foo.css\";",
            message: expected("uRl(\"foo.css\")", "\"foo.css\""),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 23,
            fix: {
                range: [8, 22],
                text: "\"foo.css\"",
            },
        },
        {
            code: "@import URL(\"foo.css\");",
            fixed: "@import \"foo.css\";",
            message: expected("URL(\"foo.css\")", "\"foo.css\""),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 23,
            fix: {
                range: [8, 22],
                text: "\"foo.css\"",
            },
        },
        {
            code: "@import url( \"foo.css\" );",
            fixed: "@import \"foo.css\";",
            message: expected("url( \"foo.css\" )", "\"foo.css\""),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 25,
            fix: {
                range: [8, 24],
                text: "\"foo.css\"",
            },
        },
        {
            code: "@import url('foo.css');",
            fixed: "@import 'foo.css';",
            message: expected("url('foo.css')", "'foo.css'"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 23,
            fix: {
                range: [8, 22],
                text: "'foo.css'",
            },
        },
        {
            code: "@import url( 'foo.css' );",
            fixed: "@import 'foo.css';",
            message: expected("url( 'foo.css' )", "'foo.css'"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 25,
            fix: {
                range: [8, 24],
                text: "'foo.css'",
            },
        },
        {
            code: "@import url(foo.css);",
            fixed: "@import \"foo.css\";",
            message: expected("url(foo.css)", "\"foo.css\""),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 21,
            fix: {
                range: [8, 20],
                text: "\"foo.css\"",
            },
        },
        {
            code: "@import url(foo.css) print;",
            fixed: "@import \"foo.css\" print;",
            message: expected("url(foo.css)", "\"foo.css\""),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 21,
            fix: {
                range: [8, 20],
                text: "\"foo.css\"",
            },
        },
        {
            code: "@import url('foo.css') supports(display: flex) screen and (max-width: 400px);",
            fixed: "@import 'foo.css' supports(display: flex) screen and (max-width: 400px);",
            message: expected("url('foo.css')", "'foo.css'"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 23,
            fix: {
                range: [8, 22],
                text: "'foo.css'",
            },
        },
        {
            code: "@import url('foo.css' /* comment */) print;",
            fixed: "@import 'foo.css' /* comment */ print;",
            description: "comment within the url function",
            message: expected("url('foo.css' /* comment */)", "'foo.css' /* comment */"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 37,
            fix: {
                range: [8, 36],
                text: "'foo.css' /* comment */",
            },
        },
    ],
}
