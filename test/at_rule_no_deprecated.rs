macro_rules! rejected {
    ($at_rule:expr) => {
        format!("Deprecated at-rule \"{}\"", $at_rule)
    };
}

test_rule! {
    name: default_options,
    ruleName: "at-rule-no-deprecated",
    config: [true],
    fix: true,
    computeEditInfo: true,
    accept: [
        {
            code: "@position-try --foo {}",
        },
        {
            code: "@starting-style {}",
        },
        {
            code: "@container (min-width: 700px) {}",
        },
        {
            code: "@CONTAINER (min-width: 500px) {}",
        },
        {
            code: "a { @apply --foo; }",
        },
    ],
    reject: [
        {
            code: "@document url(http://www.w3.org/);",
            unfixable: true,
            message: rejected!("@document"),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "@VIEWPORT { orientation: landscape; }",
            unfixable: true,
            message: rejected!("@VIEWPORT"),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "a { @nest .foo & {} }",
            fixed: "a { .foo & {} }",
            message: rejected!("@nest"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
            fix: {
                range: [4, 10],
                text: "",
            },
        },
        {
            code: "a { @NEST .foo & {} }",
            fixed: "a { .foo & {} }",
            message: rejected!("@NEST"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
            fix: {
                range: [4, 10],
                text: "",
            },
        },
        {
            code: "a { @nest .foo /* comment */ & {} }",
            fixed: "a { .foo /* comment */ & {} }",
            description: "comment within the selector",
            message: rejected!("@nest"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
            fix: {
                range: [4, 10],
                text: "",
            },
        },
        {
            code: "a { @nest .foo {} @nest .bar {} }",
            fixed: "a { .foo {} .bar {} }",
            description: "two nested @nest at-rules",
            warnings: [
                {
                    message: rejected!("@nest"),
                    line: 1,
                    column: 5,
                    endLine: 1,
                    endColumn: 10,
                    fix: {
                        range: [4, 10],
                        text: "",
                    },
                },
                {
                    message: rejected!("@nest"),
                    line: 1,
                    column: 19,
                    endLine: 1,
                    endColumn: 24,
                },
            ],
        },
    ],
}

test_rule! {
    name: ignore_at_rules,
    ruleName: "at-rule-no-deprecated",
    config: [true, { "ignoreAtRules": ["document", "/^view/"] }],
    accept: [
        {
            code: "@document url(http://www.w3.org/);",
        },
        {
            code: "@viewport { orientation: landscape; }",
        },
    ],
    reject: [
        {
            code: "a { @nest .foo & {} }",
            message: rejected!("@nest"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
    ],
}
