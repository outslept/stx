fn expected(pattern: &str) -> String {
    format!("Expected comment to match pattern \"{pattern}\"")
}

test_rule! {
    name: regex_pattern,
    ruleName: "comment-pattern",
    config: ["/foo-.+/"],

    accept: [
        {
            code: "/* foo-valid yay */",
        },
        {
            code: "/* foo--\nmulti-line\n\n\r\n\r\n\n\r\n\t\n*/",
        },
    ],

    reject: [
        {
            code: "/* not foo- */",
            message: expected("/foo-.+/"),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 15,
        },
        {
            code: "/**/",
            message: expected("/foo-.+/"),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 5,
        },
    ],
}

test_rule! {
    name: string_pattern,
    ruleName: "comment-pattern",
    config: ["foo-.+"],

    accept: [
        {
            code: "/* foo-valid yay */",
        },
        {
            code: "/* foo--\nmulti-line\n\n\r\n\r\n\n\r\n\t\n*/",
        },
    ],

    reject: [
        {
            code: "/* not foo- */",
            message: expected("foo-.+"),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 15,
        },
        {
            code: "/**/",
            message: expected("foo-.+"),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 5,
        },
    ],
}

test_rule! {
    name: scss,
    ruleName: "comment-pattern",
    config: ["/foo-.+/"],
    customSyntax: "postcss-scss",

    accept: [
        {
            code: "a {} // foo-ok",
        },
        {
            code: "// foo-ok",
        },
    ],

    reject: [
        {
            code: "a {} // not-foo",
            description: "checks inline scss comments",
            message: expected("/foo-.+/"),
            line: 1,
            column: 6,
            endLine: 1,
            endColumn: 16,
        },
    ],
}
