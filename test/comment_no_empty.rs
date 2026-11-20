const RULE_NAME: &str = "comment-no-empty";
const REJECTED: &str = "Empty comment";

test_rule! {
    name: default_config,
    ruleName: RULE_NAME,
    config: [true],
    accept: [
        {
            code: "/* comment */",
        },
        {
            code: "/*\n multi-line comment\n */\n",
        },
        {
            code: "a { color: red; /* comment */ }",
        },
    ],
    reject: [
        {
            code: "/**/",
            message: REJECTED,
            line: 1,
            column: 1,
        },
        {
            code: " /* */",
            message: REJECTED,
            line: 1,
            column: 2,
        },
        {
            code: "\n/*\n*/",
            message: REJECTED,
            line: 2,
            column: 1,
        },
        {
            code: "\n /*\r\n*/",
            message: REJECTED,
            line: 2,
            column: 2,
        },
        {
            code: "/*\t\t*/",
            message: REJECTED,
            line: 1,
            column: 1,
        },
        {
            code: "a {/*\r\n*/\n color: red;}",
            message: REJECTED,
            line: 1,
            column: 4,
        },
    ],
}

test_rule! {
    name: scss_syntax,
    ruleName: RULE_NAME,
    config: [true],
    customSyntax: "postcss-scss",
    accept: [
        {
            code: "//",
            description: "ignore inline scss comments",
        },
        {
            code: "a { color: red; } // ",
            description: "ignore inline scss comments",
        },
    ],
}
