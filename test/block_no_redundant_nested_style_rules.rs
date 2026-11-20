const REJECTED: &str = "Redundant nested style rule";

test_rule! {
    name: default_options,
    ruleName: "block-no-redundant-nested-style-rules",
    config: [true],
    accept: [
        {
            code: "",
        },
        {
            code: "a {}",
        },
        {
            code: "& {}",
        },
        {
            code: "a { &:hover {} }",
        },
        {
            code: "a { &.foo {} }",
        },
        {
            code: "a { &#foo {} }",
        },
        {
            code: "a { &[foo] {} }",
        },
        {
            code: "a { & .foo {} }",
        },
        {
            code: "a { & > .foo {} }",
        },
        {
            code: "a { &::before { content: \"\"; } }",
        },
        {
            code: "a & {}",
        },
        {
            code: "@media all { a {} }",
        },
        {
            code: "a { @media all { color: red; } }",
        },
        {
            code: "a { /* comment */ &.foo {} }",
            description: "with comment",
        },
        {
            code: "a { & /* comment */ .foo {} }",
            description: "nesting selector with comment",
        },
    ],
    reject: [
        {
            code: "a { & {} }",
            message: REJECTED,
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: "a { @media all { & {} } }",
            message: REJECTED,
            line: 1,
            column: 18,
            endLine: 1,
            endColumn: 22,
        },
        {
            code: "@scope (a) { & {} }",
            message: REJECTED,
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: "a { @media all {} & {} }",
            message: REJECTED,
            line: 1,
            column: 19,
            endLine: 1,
            endColumn: 23,
        },
        {
            code: "a { & {} & {} }",
            warnings: [
                {
                    message: REJECTED,
                    line: 1,
                    column: 5,
                    endLine: 1,
                    endColumn: 9,
                },
                {
                    message: REJECTED,
                    line: 1,
                    column: 10,
                    endLine: 1,
                    endColumn: 14,
                },
            ],
        },
        {
            code: "a { & /* foo */ {} }",
            message: REJECTED,
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 19,
            description: "with comment",
        },
        {
            code: "a { /* foo */ & {} }",
            message: REJECTED,
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 19,
        },
    ],
}
