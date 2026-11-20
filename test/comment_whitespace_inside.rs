const EXPECTED_OPENING: &str = "Expected whitespace after \"/*\"";
const REJECTED_OPENING: &str = "Expected no whitespace after \"/*\"";
const EXPECTED_CLOSING: &str = "Expected whitespace before \"*/\"";
const REJECTED_CLOSING: &str = "Expected no whitespace before \"*/\"";

test_rule! {
    name: always,
    ruleName: "comment-whitespace-inside",
    config: ["always"],
    fix: true,
    computeEditInfo: true,

    accept: [
        {
            code: "/* comment */",
        },
        {
            code: "/* comment comment */",
        },
        {
            code: "/* comment\ncomment */",
        },
        {
            code: "/* comment\n\ncomment */",
        },
        {
            code: "/** comment */",
        },
        {
            code: "/**** comment ***/",
        },
        {
            code: "/*\ncomment\n*/",
        },
        {
            code: "/*\tcomment   */",
        },
        {
            code: "/*! copyright */",
        },
        {
            code: "/*# sourcemap */",
        },
        {
            code: "/*!\ncopyright */",
        },
        {
            code: "/*#\nsourcemap */",
        },
    ],

    reject: [
        {
            code: "/*comment */",
            fixed: "/* comment */",
            message: EXPECTED_OPENING,
            line: 1,
            column: 3,
            fix: { range: [1, 2], text: "* " },
        },
        {
            code: "/**comment **/",
            fixed: "/** comment **/",
            message: EXPECTED_OPENING,
            line: 1,
            column: 4,
            fix: { range: [2, 3], text: "* " },
        },
        {
            code: "/* comment*/",
            fixed: "/* comment */",
            message: EXPECTED_CLOSING,
            line: 1,
            column: 10,
            fix: { range: [9, 10], text: "t " },
        },
        {
            code: "/*comment comment */",
            fixed: "/* comment comment */",
            message: EXPECTED_OPENING,
            line: 1,
            column: 3,
            fix: { range: [1, 2], text: "* " },
        },
        {
            code: "/* comment comment*/",
            fixed: "/* comment comment */",
            message: EXPECTED_CLOSING,
            line: 1,
            column: 18,
            fix: { range: [17, 18], text: "t " },
        },
        {
            code: "/*comment\n\ncomment */",
            fixed: "/* comment\n\ncomment */",
            message: EXPECTED_OPENING,
            line: 1,
            column: 3,
            fix: { range: [1, 2], text: "* " },
        },
        {
            code: "/* comment\n\ncomment*/",
            fixed: "/* comment\n\ncomment */",
            message: EXPECTED_CLOSING,
            line: 3,
            column: 7,
            fix: { range: [18, 19], text: "t " },
        },
        {
            code: "/*!copyright */",
            fixed: "/* !copyright */",
            message: EXPECTED_OPENING,
            line: 1,
            column: 3,
            fix: { range: [1, 2], text: "* " },
        },
        {
            code: "/*comment\n *comment\n *comment\n */",
            fixed: "/* comment\n *comment\n *comment\n */",
            message: EXPECTED_OPENING,
            line: 1,
            column: 3,
            fix: { range: [1, 2], text: "* " },
        },
        {
            code: "/*comment*/",
            fixed: "/* comment */",
            warnings: [
                {
                    message: EXPECTED_OPENING,
                    line: 1,
                    column: 3,
                    endLine: 1,
                    endColumn: 4,
                    fix: { range: [1, 2], text: "* " },
                },
                {
                    message: EXPECTED_CLOSING,
                    line: 1,
                    column: 9,
                    endLine: 1,
                    endColumn: 10,
                },
            ],
        },
    ],
}

test_rule! {
    name: never,
    ruleName: "comment-whitespace-inside",
    config: ["never"],
    fix: true,
    computeEditInfo: true,

    accept: [
        {
            code: "/*comment*/",
        },
        {
            code: "/*comment comment*/",
        },
        {
            code: "/*comment\ncomment*/",
        },
        {
            code: "/*comment\n\ncomment*/",
        },
        {
            code: "/**comment*/",
        },
        {
            code: "/****comment***/",
        },
        {
            code: "/*! copyright */",
        },
        {
            code: "/*# sourcemap */",
        },
    ],

    reject: [
        {
            code: "/* comment*/",
            fixed: "/*comment*/",
            message: REJECTED_OPENING,
            line: 1,
            column: 3,
            fix: { range: [2, 3], text: "" },
        },
        {
            code: "/** comment*/",
            fixed: "/**comment*/",
            message: REJECTED_OPENING,
            line: 1,
            column: 4,
            fix: { range: [3, 4], text: "" },
        },
        {
            code: "/*comment */",
            fixed: "/*comment*/",
            message: REJECTED_CLOSING,
            line: 1,
            column: 10,
            fix: { range: [9, 10], text: "" },
        },
        {
            code: "/*  comment*/",
            fixed: "/*comment*/",
            message: REJECTED_OPENING,
            line: 1,
            column: 3,
            endLine: 1,
            endColumn: 5,
            fix: { range: [2, 4], text: "" },
        },
        {
            code: "/*comment  */",
            fixed: "/*comment*/",
            message: REJECTED_CLOSING,
            line: 1,
            column: 10,
            endLine: 1,
            endColumn: 12,
            fix: { range: [9, 11], text: "" },
        },
        {
            code: "/*\ncomment*/",
            fixed: "/*comment*/",
            message: REJECTED_OPENING,
            line: 1,
            column: 3,
            fix: { range: [2, 3], text: "" },
        },
        {
            code: "/*comment\n*/",
            fixed: "/*comment*/",
            message: REJECTED_CLOSING,
            line: 1,
            column: 10,
            fix: { range: [9, 10], text: "" },
        },
        {
            code: "/* comment comment*/",
            fixed: "/*comment comment*/",
            message: REJECTED_OPENING,
            line: 1,
            column: 3,
            fix: { range: [2, 3], text: "" },
        },
        {
            code: "/*comment comment */",
            fixed: "/*comment comment*/",
            message: REJECTED_CLOSING,
            line: 1,
            column: 18,
            fix: { range: [17, 18], text: "" },
        },
        {
            code: "/* comment\n\ncomment*/",
            fixed: "/*comment\n\ncomment*/",
            message: REJECTED_OPENING,
            line: 1,
            column: 3,
            fix: { range: [2, 3], text: "" },
        },
        {
            code: "/*comment\n\ncomment */",
            fixed: "/*comment\n\ncomment*/",
            message: REJECTED_CLOSING,
            line: 3,
            column: 8,
            fix: { range: [18, 19], text: "" },
            endLine: 3,
            endColumn: 9,
        },
        {
            code: "/*comment\n *comment\n *comment\n */",
            fixed: "/*comment\n *comment\n *comment*/",
            message: REJECTED_CLOSING,
            line: 3,
            column: 10,
            endLine: 4,
            endColumn: 2,
            fix: { range: [29, 31], text: "" },
        },
        {
            code: "/*comment\n \t*/",
            fixed: "/*comment*/",
            message: REJECTED_CLOSING,
            line: 1,
            column: 10,
            endLine: 2,
            endColumn: 3,
            fix: { range: [9, 12], text: "" },
        },
    ],
}

test_rule! {
    name: scss_always,
    ruleName: "comment-whitespace-inside",
    config: ["always"],
    customSyntax: "postcss-scss",

    accept: [
        {
            code: "//comment",
            description: "single-line comment ignored",
        },
    ],
}

test_rule! {
    name: scss_never,
    ruleName: "comment-whitespace-inside",
    config: ["never"],
    customSyntax: "postcss-scss",

    accept: [
        {
            code: "// comment",
            description: "single-line comment ignored",
        },
    ],
}

test_rule! {
    name: less_always,
    ruleName: "comment-whitespace-inside",
    config: ["always"],
    customSyntax: "postcss-less",

    accept: [
        {
            code: "//comment",
            description: "single-line comment ignored",
        },
    ],
}

test_rule! {
    name: less_never,
    ruleName: "comment-whitespace-inside",
    config: ["never"],
    customSyntax: "postcss-less",

    accept: [
        {
            code: "// comment",
            description: "single-line comment ignored",
        },
    ],
}
