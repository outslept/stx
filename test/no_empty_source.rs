const RULE_NAME: &str = "no-empty-source";
const REJECTED: &str = "Empty source";

test_rule! {
    name: default_config,
    ruleName: RULE_NAME,
    config: [true],
    accept: [
        {
            code: ".class {}",
        },
        {
            code: "   .class {}   ",
        },
        {
            code: "/* comment */",
        },
        {
            code: "\n.class {}",
        },
        {
            code: "\r\n.class {}",
        },
        {
            code: "/* stylelint-disable */",
        },
        {
            code: "/* stylelint-disable no-empty-source */",
        },
    ],
    reject: [
        {
            code: "",
            description: "empty source",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 2,
        },
        {
            code: "   ",
            description: "source with spaces",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 5,
        },
        {
            code: "\t",
            description: "source with tab",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 3,
        },
        {
            code: "\n",
            description: "source with newline",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 2,
            endColumn: 2,
        },
        {
            code: "\r\n",
            description: "source with newline",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 2,
            endColumn: 2,
        },
        {
            code: "\n\n\n",
            description: "source with multiple newline",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 4,
            endColumn: 2,
        },
        {
            code: "\r\n\r\n\r\n",
            description: "source with multiple newline",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 4,
            endColumn: 2,
        },
        {
            code: "  \n  ",
            description: "source with spaces and newline",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 2,
            endColumn: 4,
        },
        {
            code: "  \r\n  ",
            description: "source with spaces and newline",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 2,
            endColumn: 4,
        },
        {
            code: "/* stylelint-disable foo */",
            description: "unrelated configuration comment",
            message: REJECTED,
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 29,
        },
    ],
}

test_rule! {
    name: html_syntax,
    ruleName: RULE_NAME,
    config: [true],
    customSyntax: "postcss-html",
    skip: true,
    accept: [
        {
            description: "HTML without CSS",
            code: "<html></html>",
        },
    ],
    reject: [
        {
            code: "<style>\n</style>",
            description: "CSS block in HTML",
            message: REJECTED,
            line: 2,
            column: 1,
            endLine: 2,
            endColumn: 2,
        },
    ],
}

test_rule! {
    name: css_in_js_syntax,
    ruleName: RULE_NAME,
    config: [true],
    customSyntax: "naiveCssInJs",
    skip: true,
    accept: [
        {
            code: "css` $var `;",
            description: "contains only non-standard syntax",
        },
    ],
}
