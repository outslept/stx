macro_rules! expected {
    ($primary:expr) => {
        format!("Expected {} color-function notation", $primary)
    };
}

test_rule! {
    name: modern,
    ruleName: "color-function-notation",
    config: ["modern"],
    fix: true,
    computeEditInfo: true,
    accept: [
        {
            code: "a { color: rgb(0 0 0) }",
        },
        {
            code: "a { color: rgb(12 122 231 / 0.2) }",
        },
        {
            code: "a { color: rgb(51 170 51 / 20%) }",
        },
        {
            code: "a { color: hsl(4.71239rad 60% 70%) }",
        },
        {
            code: "a { color: hsl(.75turn 60% 70%) }",
        },
        {
            code: "a { color: hsl(270 60% 50% / .15) }",
        },
        {
            code: "a { color: hsl(270 60% 50% / 15%) }",
        },
        {
            code: "a { color: HSL(270 60% 70%) }",
        },
        {
            code: "a { color: RgB(240 0 0 / 50%) }",
        },
        {
            code: "a { color: hsl(var(--hue) var(--sat) var(--sat) / var(--alpha)) }",
        },
        {
            code: "a {\n\tcolor: hsl(270 60% 50%\n\t\t/ 15%)\n}",
        },
        {
            code: "a {\n\tcolor: hsl(\n\t\t270\n\t\t60%\n\t\t50%\n\t\t/ 15%\n\t)\n}",
        },
        {
            code: "a {\n\tcolor: hsl(\n\t\t270 /* comment */\n\t\t60%\n\t\t50%\n\t\t/* comment */ / 15%\n\t)\n}",
        },
    ],
    reject: [
        {
            code: "a { color: rgb(0, 0, 0) }",
            fixed: "a { color: rgb(0 0 0) }",
            message: expected!("modern"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 24,
            fix: {
                range: [16, 20],
                text: " 0",
            },
        },
        {
            code: "a { color: rgba(0, 0, 0, 0) }",
            fixed: "a { color: rgb(0 0 0 / 0) }",
            message: expected!("modern"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 28,
            fix: {
                range: [14, 24],
                text: "(0 0 0 /",
            },
        },
        {
            code: "a { color: rgba(0, 0, 0, 50%) }",
            fixed: "a { color: rgb(0 0 0 / 50%) }",
            message: expected!("modern"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 30,
            fix: {
                range: [14, 24],
                text: "(0 0 0 /",
            },
        },
        {
            code: "a { color: rgba(0 , 0 , 0 , 50%) }",
            fixed: "a { color: rgb(0 0 0 / 50%) }",
            message: expected!("modern"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 33,
            fix: {
                range: [14, 27],
                text: "(0 0 0 /",
            },
        },
        {
            code: "a { color: rgba(0,0,0,50%) }",
            fixed: "a { color: rgb(0 0 0 / 50%) }",
            message: expected!("modern"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 27,
            fix: {
                range: [14, 22],
                text: "(0 0 0 / ",
            },
        },
        {
            code: "a { color: rgba(var(--r, 20), var(--g), var(--b), 0.6); }",
            fixed: "a { color: rgb(var(--r, 20) var(--g) var(--b) / 0.6); }",
            message: expected!("modern"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 55,
            fix: {
                range: [14, 49],
                text: "(var(--r, 20) var(--g) var(--b) /",
            },
        },
        {
            code: "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30),\n\t\t28%, 50%, 0.2\n\t);\n}",
            fixed: "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% / 0.2\n\t);\n}",
            message: expected!("modern"),
            line: 2,
            column: 9,
            endLine: 5,
            endColumn: 3,
            fix: {
                range: [15, 54],
                text: "(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /",
            },
        },
        {
            code: "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30),\n\t\t28%, 50%,\n\t\t0.2\n\t);\n}",
            fixed: "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /\n\t\t0.2\n\t);\n}",
            message: expected!("modern"),
            line: 2,
            column: 9,
            endLine: 6,
            endColumn: 3,
            fix: {
                range: [15, 54],
                text: "(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% /",
            },
        },
        {
            code: "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30),\n\t\t28%,\n\t\t50%, 0.2\n\t);\n}",
            fixed: "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28%\n\t\t50% / 0.2\n\t);\n}",
            message: expected!("modern"),
            line: 2,
            column: 9,
            endLine: 6,
            endColumn: 3,
            fix: {
                range: [15, 56],
                text: "(\n\t\tcalc(var(--hue) + 30)\n\t\t28%\n\t\t50% /",
            },
        },
        {
            code: "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30) /* comment */,\n\t\t28%, 50% /* comment */, 0.2\n\t);\n}",
            fixed: "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30) /* comment */\n\t\t28% 50% /* comment */ / 0.2\n\t);\n}",
            message: expected!("modern"),
            line: 2,
            column: 9,
            endLine: 5,
            endColumn: 3,
            fix: {
                range: [15, 82],
                text: "(\n\t\tcalc(var(--hue) + 30) /* comment */\n\t\t28% 50% /* comment */ /",
            },
        },
        {
            code: "a {\n\tcolor: HSLA(\n\t\tcalc(var(--hue) + 30) /* comment */,\n\t\t28%, 50% /* comment */\n\t\t, 0.2\n\t);\n}",
            fixed: "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30) /* comment */\n\t\t28% 50% /* comment */\n\t\t/ 0.2\n\t);\n}",
            message: expected!("modern"),
            line: 2,
            column: 9,
            endLine: 6,
            endColumn: 3,
            fix: {
                range: [15, 85],
                text: "(\n\t\tcalc(var(--hue) + 30) /* comment */\n\t\t28% 50% /* comment */\n\t\t/",
            },
        },
        {
            code: "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\trgba(30, 75, 115, 1),\n\t\thsl(255, 255, 255)\n\t);\n}",
            fixed: "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\trgb(30 75 115 / 1),\n\t\thsl(255 255 255)\n\t);\n}",
            warnings: [
                {
                    message: expected!("modern"),
                    line: 4,
                    column: 3,
                    endLine: 4,
                    endColumn: 23,
                    fix: {
                        range: [57, 71],
                        text: "(30 75 115 /",
                    },
                },
                {
                    message: expected!("modern"),
                    line: 5,
                    column: 3,
                    endLine: 5,
                    endColumn: 21,
                },
            ],
        },
    ],
}

test_rule! {
    name: legacy,
    ruleName: "color-function-notation",
    config: ["legacy"],
    accept: [
        {
            code: "a { color: rgb(0, 0, 0) }",
        },
        {
            code: "a { color: rgba(12, 122, 231, 0.2) }",
        },
        {
            code: "a { color: rgba(51,170,51,20%) }",
        },
        {
            code: "a { color: hsl(4.71239rad, 60%, 70%) }",
        },
        {
            code: "a { color: hsl(.75turn, 60%, 70%) }",
        },
        {
            code: "a { color: hsla(270, 60%, 50%, .15) }",
        },
        {
            code: "a { color: hsla(270, 60%, 50%, 15%) }",
        },
        {
            code: "a { color: HSL(270, 60%, 70%) }",
        },
        {
            code: "a { color: RgBA(240, 0, 0, 50%) }",
        },
        {
            code: "a { color: hsla(var(--hue), var(--sat), var(--sat), var(--alpha)) }",
        },
        {
            code: "a {\n\tcolor: hsla(270, 60%, 50%\n\t\t, 15%)\n}",
        },
        {
            code: "a {\n\tcolor: hsla(\n\t\t270,\n\t\t60%,\n\t\t50%,\n\t\t15%\n\t)\n}",
        },
        {
            code: "a {\n\tcolor: hsla(\n\t\t270, /* comment */\n\t\t60%,\n\t\t50%,\n\t\t/* comment */ 15%\n\t)\n}",
        },
    ],
    reject: [
        {
            code: "a { color: rgb(0 0 0) }",
            message: expected!("legacy"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 22,
        },
        {
            code: "a { color: rgb(0 0 0 0) }",
            message: expected!("legacy"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 24,
        },
        {
            code: "a { color: rgb(0 0 0 / 50%) }",
            message: expected!("legacy"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 28,
        },
        {
            code: "a {\n\tcolor: HSL(\n\t\tcalc(var(--hue) + 30)\n\t\t28% 50% / 0.2\n\t);\n}",
            message: expected!("legacy"),
            line: 2,
            column: 9,
            endLine: 5,
            endColumn: 3,
        },
        {
            code: "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\trgb(30 75 115 / 1),\n\t\thsl(255 255 255)\n\t);\n}",
            warnings: [
                {
                    message: expected!("legacy"),
                    line: 4,
                    column: 3,
                    endLine: 4,
                    endColumn: 21,
                },
                {
                    message: expected!("legacy"),
                    line: 5,
                    column: 3,
                    endLine: 5,
                    endColumn: 19,
                },
            ],
        },
    ],
}

test_rule! {
    name: modern_scss,
    ruleName: "color-function-notation",
    config: ["modern"],
    customSyntax: "postcss-scss",
    fix: true,
    computeEditInfo: true,
    accept: [
        {
            code: "a { color: rgb(white, .5); }",
        },
        {
            code: "$a: rgb(0 0 0);",
        },
        {
            code: "$a: rgb(0 0 0 / 0);",
        },
        {
            code: "a { color: rgb($a $b $c / $d) }",
        },
        {
            code: "a { color: rgb($a, $b, $c, $d) }",
        },
        {
            code: "a { color: rgb($color-var, 50%); }",
        },
        {
            code: "a { color: rgba(color.mix(#000, #fff, 35%), 0.6); }",
        },
        {
            code: "a { color: rgba(#fff, 0.85) }",
        },
        {
            code: "a { color: rgba($a, $b, $c, $d) }",
        },
        {
            code: "a { color: rgb(color.$var, 0.1) }",
            description: "namespaced import",
        },
        {
            code: "a { color: rgba(color.$var, 0.1) }",
            description: "namespaced import",
        },
    ],
    reject: [
        {
            code: "$a: rgb(0, 0, 0);",
            fixed: "$a: rgb(0 0 0);",
            message: expected!("modern"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 17,
            fix: {
                range: [9, 13],
                text: " 0",
            },
        },
        {
            code: "$a: rgb(0, 0, 0, 0);",
            fixed: "$a: rgb(0 0 0 / 0);",
            message: expected!("modern"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 20,
            fix: {
                range: [9, 16],
                text: " 0 0 /",
            },
        },
    ],
}

test_rule! {
    name: modern_ignore_with_var_inside,
    ruleName: "color-function-notation",
    config: ["modern", { "ignore": "with-var-inside" }],
    fix: true,
    accept: [
        {
            code: "a { color: rgba(var(--bar), 0.5); }",
        },
        {
            code: "a { color: rgba(VaR(--bar), 0.5); }",
        },
    ],
}

test_rule! {
    name: legacy_ignore_with_var_inside,
    ruleName: "color-function-notation",
    config: ["legacy", { "ignore": "with-var-inside" }],
    fix: true,
    accept: [
        {
            code: "a { color: rgb(var(--foo) / 0.5); }",
        },
        {
            code: "a { color: rgb(VaR(--foo) / 0.5); }",
        },
    ],
}

test_rule! {
    name: legacy_scss,
    ruleName: "color-function-notation",
    config: ["legacy"],
    customSyntax: "postcss-scss",
    accept: [
        {
            code: "$a: rgb(0, 0, 0);",
        },
        {
            code: "$a: rgba(0, 0, 0, 0);",
        },
        {
            code: "a { color: rgba($a, $b, $c, $d) }",
        },
        {
            code: "a { color: rgb($a $b $c / $d) }",
        },
        {
            code: "a { color: rgb(color.$a color.$b color.$c / color.$d) }",
            description: "namespaced import",
        },
    ],
    reject: [
        {
            code: "$a: rgb(0 0 0 / 0);",
            message: expected!("legacy"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 19,
        },
    ],
}
