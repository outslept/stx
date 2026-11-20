macro_rules! expected {
    ($unfixed:expr, $fixed:expr) => {
        format!("Expected \"{}\" to be \"{}\"", $unfixed, $fixed)
    };
}

test_rule! {
    name: with_alpha,
    ruleName: "color-function-alias-notation",
    config: ["with-alpha"],
    fix: true,
    computeEditInfo: true,
    accept: [
        {
            code: "a { color: rgba(0 0 0) }",
        },
        {
            code: "a { color: hsla(4.71239rad 60% 70%) }",
        },
        {
            code: "a { color: HSLA(270 60% 70%) }",
        },
        {
            code: "a { color: RGBA(240 0 0 / 50%) }",
        },
        {
            code: "a {\n\tcolor: hsla(\n\t\t270 /* comment */\n\t\t60%\n\t\t50%\n\t\t/* comment */ / 15%\n\t)\n}",
        },
    ],
    reject: [
        {
            code: "a { color: rgb(0 0 0) }",
            fixed: "a { color: rgba(0 0 0) }",
            message: expected!("rgb", "rgba"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 15,
            fix: {
                range: [13, 14],
                text: "ba",
            },
        },
        {
            code: "a { color: hsl(270 60% 70%) }",
            fixed: "a { color: hsla(270 60% 70%) }",
            message: expected!("hsl", "hsla"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 15,
            fix: {
                range: [13, 14],
                text: "la",
            },
        },
        {
            code: "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
            fixed: "a {\n\tcolor: HSLa(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
            message: expected!("HSL", "HSLa"),
            line: 2,
            column: 9,
            endLine: 2,
            endColumn: 12,
            fix: {
                range: [14, 15],
                text: "La",
            },
        },
    ],
}

test_rule! {
    name: without_alpha,
    ruleName: "color-function-alias-notation",
    config: ["without-alpha"],
    fix: true,
    computeEditInfo: true,
    accept: [
        {
            code: "a { color: rgb(0 0 0) }",
        },
        {
            code: "a { color: hsl(4.71239rad 60% 70%) }",
        },
        {
            code: "a { color: HSL(270 60% 70%) }",
        },
        {
            code: "a { color: RGB(240 0 0 / 50%) }",
        },
        {
            code: "a {\n\tcolor: hsl(\n\t\t270 /* comment */\n\t\t60%\n\t\t50%\n\t\t/* comment */ / 15%\n\t)\n}",
        },
    ],
    reject: [
        {
            code: "a { color: rgba(0 0 0) }",
            fixed: "a { color: rgb(0 0 0) }",
            message: expected!("rgba", "rgb"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 16,
            fix: {
                range: [14, 15],
                text: "",
            },
        },
        {
            code: "a { color: hsla(270 60% 70%) }",
            fixed: "a { color: hsl(270 60% 70%) }",
            message: expected!("hsla", "hsl"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 16,
            fix: {
                range: [14, 15],
                text: "",
            },
        },
        {
            code: "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
            fixed: "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
            message: expected!("HSLA", "HSL"),
            line: 2,
            column: 9,
            endLine: 2,
            endColumn: 13,
            fix: {
                range: [15, 16],
                text: "",
            },
        },
    ],
}
