macro_rules! expected {
    ($hex:expr) => {
        format!("Expected alpha channel in \"{}\"", $hex)
    };
}

macro_rules! rejected {
    ($hex:expr) => {
        format!("Expected no alpha channel in \"{}\"", $hex)
    };
}

test_rule! {
    name: always,
    ruleName: "color-hex-alpha",
    config: ["always"],
    accept: [
        {
            code: "a { color: #ffff; }",
        },
        {
            code: "a { color: #ffffffff; }",
        },
        {
            code: "a { background: linear-gradient(to left, #fffa, #000000aa 100%); }",
        },
        {
            code: "a { background: url(#fff); }",
        },
    ],
    reject: [
        {
            code: "a { color: #fff; }",
            message: expected!("#fff"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 16,
        },
        {
            code: "a { color: #ffffff; }",
            message: expected!("#ffffff"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 19,
        },
        {
            code: "a { background: linear-gradient(to left, #fff, #000000 100%); }",
            warnings: [
                {
                    message: expected!("#fff"),
                    line: 1,
                    column: 42,
                    endLine: 1,
                    endColumn: 46,
                },
                {
                    message: expected!("#000000"),
                    line: 1,
                    column: 48,
                    endLine: 1,
                    endColumn: 55,
                },
            ],
        },
    ],
}

test_rule! {
    name: never,
    ruleName: "color-hex-alpha",
    config: ["never"],
    accept: [
        {
            code: "a { color: #fff; }",
        },
        {
            code: "a { color: #ffffff; }",
        },
    ],
    reject: [
        {
            code: "a { color: #ffff; }",
            message: rejected!("#ffff"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 17,
        },
    ],
}

test_rule! {
    name: always_scss,
    ruleName: "color-hex-alpha",
    config: ["always"],
    customSyntax: "postcss-scss",
    accept: [
        {
            code: "a { color: #{f}; }",
            description: "scss interpolation of 3 characters",
        },
        {
            code: "$var: #ffff;",
            description: "alpha channel scss variable",
        },
    ],
    reject: [
        {
            code: "$var: #fff",
            message: expected!("#fff"),
            line: 1,
            column: 7,
            endLine: 1,
            endColumn: 11,
            description: "no alpha channel scss variable",
        },
    ],
}
