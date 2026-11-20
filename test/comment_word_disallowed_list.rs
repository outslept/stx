fn rejected(pattern: &str) -> String {
    format!("Disallowed word matching pattern \"{pattern}\"")
}

test_rule! {
    name: single_string,
    ruleName: "comment-word-disallowed-list",
    config: "bad-word",

    accept: [
        {
            code: "/* comment */",
        },
        {
            code: "/*# bad-word */",
        },
    ],

    reject: [
        {
            code: "/* Comment with bad-word  */",
            message: rejected("bad-word"),
            line: 1,
            column: 17,
            endLine: 1,
            endColumn: 25,
        },
        {
            code: "/* bad-word */",
            message: rejected("bad-word"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 12,
        },
        {
            code: "/*** bad-word ***/",
            message: rejected("bad-word"),
            line: 1,
            column: 6,
            endLine: 1,
            endColumn: 14,
        },
        {
            code: "/*! bad-word */",
            message: rejected("bad-word"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 13,
        },
        {
            code: "/** bad-word **/",
            message: rejected("bad-word"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 13,
        },
    ],
}

test_rule! {
    name: array_patterns,
    ruleName: "comment-word-disallowed-list",
    config: ["/^TODO:/", "bad-word"],

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
            code: "/*# sourcemap bad-word */",
        },
        {
            code: "a { color: pink; /* comment */\ntop: 0; }",
        },
        {
            code: "a {} /* comment */",
        },
        {
            code: "/* todo */",
        },
        {
            code: "/* todo: */",
        },
        {
            code: "/* todo: comment */",
        },
        {
            code: "/* tOdO: comment */",
        },
        {
            code: "/* Todo: comment */",
        },
        {
            code: "/*! Todo: comment */",
        },
        {
            code: "/*# Todo: comment */",
        },
        {
            code: "/** TODO: comment **/",
        },
        {
            code: "/*** TODO: comment ***/",
        },
    ],

    reject: [
        {
            code: "/* TODO: */",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "/* TODO: comment */",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "/* TODO: comment\n next line */",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "/* TODO: comment\r\n next line */",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "/* TODO: comment\n\n next line */",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "/*\n TODO: comment */",
            message: rejected("/^TODO:/"),
            line: 2,
            column: 2,
            endLine: 2,
            endColumn: 7,
        },
        {
            code: "/*\r\n TODO: comment */",
            message: rejected("/^TODO:/"),
            line: 2,
            column: 2,
            endLine: 2,
            endColumn: 7,
        },
        {
            code: "/*\n\n TODO: comment */",
            message: rejected("/^TODO:/"),
            line: 3,
            column: 2,
            endLine: 3,
            endColumn: 7,
        },
        {
            code: "/*\r\n\r\n TODO: comment */",
            message: rejected("/^TODO:/"),
            line: 3,
            column: 2,
            endLine: 3,
            endColumn: 7,
        },
        {
            code: "/* bad-word */",
            message: rejected("bad-word"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 12,
        },
        {
            code: "/* Comment with bad-word */",
            message: rejected("bad-word"),
            line: 1,
            column: 17,
            endLine: 1,
            endColumn: 25,
        },
        {
            code: "/*! copyright bad-word */",
            message: rejected("bad-word"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 23,
        },
        {
            code: "/** bad-word **/",
            message: rejected("bad-word"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 13,
        },
        {
            code: "/*** bad-word ***/",
            message: rejected("bad-word"),
            line: 1,
            column: 6,
            endLine: 1,
            endColumn: 14,
        },
    ],
}

test_rule! {
    name: scss,
    ruleName: "comment-word-disallowed-list",
    config: ["/^TODO:/", "bad-word"],
    customSyntax: "postcss-scss",

    accept: [
        {
            code: "// comment",
        },
        {
            code: "// todo",
        },
        {
            code: "// todo:",
        },
        {
            code: "// Todo:",
        },
        {
            code: "// tOdO:",
        },
    ],

    reject: [
        {
            code: "// TODO:",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "// TODO: comment",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "// bad-word",
            message: rejected("bad-word"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 12,
        },
    ],
}

test_rule! {
    name: less,
    ruleName: "comment-word-disallowed-list",
    config: ["/^TODO:/", "bad-word"],
    customSyntax: "postcss-less",

    accept: [
        {
            code: "// comment",
        },
        {
            code: "// todo:",
        },
        {
            code: "// Todo:",
        },
        {
            code: "// tOdO:",
        },
    ],

    reject: [
        {
            code: "// TODO:",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "// TODO: comment",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "// bad-word",
            message: rejected("bad-word"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 12,
        },
    ],
}

test_rule! {
    name: regex_literal_array,
    ruleName: "comment-word-disallowed-list",
    config: ["/^TODO:/", "bad-word"],

    accept: [
        {
            code: "/* comment */",
        },
    ],

    reject: [
        {
            code: "/* TODO: */",
            message: rejected("/^TODO:/"),
            line: 1,
            column: 4,
            endLine: 1,
            endColumn: 9,
        },
    ],
}
