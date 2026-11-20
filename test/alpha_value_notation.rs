const RULE_NAME: &str = "alpha-value-notation";

fn expected(unfixed: &str, fixed: &str) -> String {
    format!("Expected \"{}\" to be \"{}\"", unfixed, fixed)
}

test_rule! {
    name: number_config,
    ruleName: RULE_NAME,
    config: ["number"],
    fix: true,
    accept: [
        {
            code: "a { opacity: 0.1 }",
        },
        {
            code: "a { shape-image-threshold: .5 }",
        },
        {
            code: "a { OPACITY: /*comment*/0.1 }",
        },
        {
            code: "a { opacity: var(--alpha) }",
        },
        {
            code: "a { color: rgb(0, 0, 0) }",
        },
        {
            code: "a { color: rgb(0 0 0) }",
        },
        {
            code: "a { color: rgba(0, 0, 0, 0.5) }",
        },
        {
            code: "a { color: rgb(0 0 0 / 0.5) }",
        },
        {
            code: "a { color: hsl(198deg 28% 50% / .50) }",
        },
        {
            code: "a { color: hsla(30, 100%, 50%, .6) }",
        },
        {
            code: "a { color: RGB(0 0 0 / 0.5) }",
        },
        {
            code: "a { color: hsla(var(--hue), var(--sat), var(--sat), var(--alpha)) }",
        },
        {
            code: "a { color: hsl(var(--hsl) / 0.5) }",
        },
        {
            code: "a { color: lch(56.29% 19.86 10 / var(--alpha)) }",
        },
        {
            code: "a { color: oklch(56.29% 19.86 10 / var(--alpha)) }",
        },
        {
            code: "a { color: rgba(0, 0, 0, 0.5/*comment*/) }",
        },
        {
            code: "a { color: rgb(0 0 0 / /*comment*/0.5) }",
        },
        {
            code: "a { color: color(display-p3 0 0 0 / 0.5) }",
        },
        {
            code: "a { color: color(display-p3 0 0 0) }",
        },
        {
            code: "a { color: rgb(from blue 0 0 0 / 0.5) }",
        },
        {
            code: "a { color: hsl(from blue 198deg 28% 50% / .50) }",
        },
        {
            code: "a { color: RGB(from blue 0 0 0 / 0.5) }",
        },
        {
            code: "a { color: hsl(from blue var(--hsl) / 0.5) }",
        },
        {
            code: "a { color: lch(from rgb(0 0 0) 56.29% 19.86 10 / var(--alpha)) }",
        },
        {
            code: "a { color: oklch(from #fff 56.29% 19.86 10 / var(--alpha)) }",
        },
        {
            code: "a { color: rgb(from blue 0 0 0 / /*comment*/0.5) }",
        },
        {
            code: "a { color: color(from blue display-p3 0 0 0 / 0.5) }",
        },
    ],
    reject: [
        {
            code: "a { opacity: 10% }",
            fixed: "a { opacity: 0.1 }",
            message: expected("10%", "0.1"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 17,
        },
        {
            code: "a { shape-image-threshold: 50% }",
            fixed: "a { shape-image-threshold: 0.5 }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 28,
            endLine: 1,
            endColumn: 31,
        },
        {
            code: "feDropShadow { flood-opacity: 50% }",
            fixed: "feDropShadow { flood-opacity: 0.5 }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 31,
            endLine: 1,
            endColumn: 34,
        },
        {
            code: "a { color: rgba(0, 0, 0, 50%) }",
            fixed: "a { color: rgba(0, 0, 0, 0.5) }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 29,
        },
        {
            code: "a { color: rgb(0 0 0 / 50%) }",
            fixed: "a { color: rgb(0 0 0 / 0.5) }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 24,
            endLine: 1,
            endColumn: 27,
        },
        {
            code: "a { color: HSL(198DEG 28% 50% / 10%) }",
            fixed: "a { color: HSL(198DEG 28% 50% / 0.1) }",
            message: expected("10%", "0.1"),
            line: 1,
            column: 33,
            endLine: 1,
            endColumn: 36,
        },
        {
            code: "a { color: rgba(0, 0, 0, 50%/*comment*/) }",
            fixed: "a { color: rgba(0, 0, 0, 0.5/*comment*/) }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 29,
        },
        {
            code: "a { color: hsl(var(--hsl) / 50%) }",
            fixed: "a { color: hsl(var(--hsl) / 0.5) }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 29,
            endLine: 1,
            endColumn: 32,
        },
        {
            code: "a { color: hsl(var(--hsl) / /* comment*/ 50%) }",
            fixed: "a { color: hsl(var(--hsl) / /* comment*/ 0.5) }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 42,
            endLine: 1,
            endColumn: 45,
        },
        {
            code: "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\thwb(270 60% 70% / 10%)\n\t\tlch(56.29% 19.86 236.62 / 40%)\n\t);\n}",
            fixed: "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\thwb(270 60% 70% / 0.1)\n\t\tlch(56.29% 19.86 236.62 / 0.4)\n\t);\n}",
            warnings: [
                {
                    message: expected("10%", "0.1"),
                    line: 4,
                    column: 21,
                    endLine: 4,
                    endColumn: 24,
                },
                {
                    message: expected("40%", "0.4"),
                    line: 5,
                    column: 29,
                    endLine: 5,
                    endColumn: 32,
                },
            ],
        },
        {
            code: "a { opacity: 14% }",
            fixed: "a { opacity: 0.14 }",
            message: expected("14%", "0.14"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 17,
            description: "properly deals with floating-point conversions",
        },
        {
            code: "a { opacity: 0.3% }",
            fixed: "a { opacity: 0.003 }",
            message: expected("0.3%", "0.003"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 18,
            description: "properly deals with floating-point conversions",
        },
        {
            code: "a { color: rgb(from rgb(127 127 127 / 25%) 0 0 0 / 50%) }",
            fixed: "a { color: rgb(from rgb(127 127 127 / 0.25) 0 0 0 / 0.5) }",
            warnings: [
                {
                    message: expected("50%", "0.5"),
                    line: 1,
                    column: 52,
                    endLine: 1,
                    endColumn: 55,
                },
                {
                    message: expected("25%", "0.25"),
                    line: 1,
                    column: 39,
                    endLine: 1,
                    endColumn: 42,
                },
            ],
        },
        {
            code: "a { color: color(display-p3 0 0 0 / 50%) }",
            fixed: "a { color: color(display-p3 0 0 0 / 0.5) }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 37,
            endLine: 1,
            endColumn: 40,
        },
    ],
}

test_rule! {
    name: percentage_config,
    ruleName: RULE_NAME,
    config: ["percentage"],
    fix: true,
    accept: [
        {
            code: "a { opacity: 10% }",
        },
        {
            code: "a { shape-image-threshold: 50% }",
        },
        {
            code: "a { OPACITY: /*comment*/10% }",
        },
        {
            code: "a { color: rgba(0, 0, 0, 50%) }",
        },
        {
            code: "a { color: RGB(0 0 0 / 50%) }",
        },
        {
            code: "a { color: lch(56.29% 19.86 10 / var(--alpha)) }",
        },
        {
            code: "a { color: color(display-p3 0 0 0) }",
        },
        {
            code: "a { color: color(display-p3 0 0 0 / 50%) }",
        },
    ],
    reject: [
        {
            code: "a { opacity: 0.1 }",
            fixed: "a { opacity: 10% }",
            message: expected("0.1", "10%"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 17,
        },
        {
            code: "a { color: rgba(0, 0, 0, .5) }",
            fixed: "a { color: rgba(0, 0, 0, 50%) }",
            message: expected(".5", "50%"),
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 28,
        },
        {
            code: "a { color: HSL(198DEG 28% 50% / 0.1) }",
            fixed: "a { color: HSL(198DEG 28% 50% / 10%) }",
            message: expected("0.1", "10%"),
            line: 1,
            column: 33,
            endLine: 1,
            endColumn: 36,
        },
        {
            code: "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\thwb(270 60% 70% / 0.1)\n\t\tlch(56.29% 19.86 236.62 / 0.40)\n\t);\n}",
            fixed: "a {\n\tbackground-image: linear-gradient(\n\t\tto right,\n\t\thwb(270 60% 70% / 10%)\n\t\tlch(56.29% 19.86 236.62 / 40%)\n\t);\n}",
            warnings: [
                {
                    message: expected("0.1", "10%"),
                    line: 4,
                    column: 21,
                    endLine: 4,
                    endColumn: 24,
                },
                {
                    message: expected("0.40", "40%"),
                    line: 5,
                    column: 29,
                    endLine: 5,
                    endColumn: 33,
                },
            ],
        },
        {
            code: "a { opacity: 0.14 }",
            fixed: "a { opacity: 14% }",
            message: expected("0.14", "14%"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 18,
            description: "properly deals with floating-point conversions",
        },
        {
            code: "a { opacity: 0.003 }",
            fixed: "a { opacity: 0.3% }",
            message: expected("0.003", "0.3%"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 19,
            description: "properly deals with floating-point conversions",
        },
    ],
}

test_rule! {
    name: number_except_opacity,
    ruleName: RULE_NAME,
    config: ["number", { "exceptProperties": ["opacity"] }],
    fix: true,
    accept: [
        {
            code: "a { opacity: 50% }",
        },
        {
            code: "a { shape-image-threshold: .5 }",
        },
        {
            code: "a { color: rgb(0 0 0 / 0.5) }",
        },
    ],
    reject: [
        {
            code: "a { opacity: 0.1 }",
            fixed: "a { opacity: 10% }",
            message: expected("0.1", "10%"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 17,
        },
        {
            code: "a { color: rgba(0, 0, 0, 50%) }",
            fixed: "a { color: rgba(0, 0, 0, 0.5) }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 29,
        },
        {
            code: "a { color: rgba(0, 0, 0, 50%); background-color: rgba(0, 0, 0, 70%); }",
            fixed: "a { color: rgba(0, 0, 0, 0.5); background-color: rgba(0, 0, 0, 0.7); }",
            warnings: [
                {
                    message: expected("50%", "0.5"),
                    line: 1,
                    column: 26,
                    endLine: 1,
                    endColumn: 29,
                },
                {
                    message: expected("70%", "0.7"),
                    line: 1,
                    column: 64,
                    endLine: 1,
                    endColumn: 67,
                },
            ],
        },
    ],
}

test_rule! {
    name: percentage_except_opacity,
    ruleName: RULE_NAME,
    config: ["percentage", { "exceptProperties": ["opacity"] }],
    fix: true,
    accept: [
        {
            code: "a { opacity: 0.5 }",
        },
        {
            code: "a { shape-image-threshold: 50% }",
        },
        {
            code: "a { color: rgb(0 0 0 / 50%) }",
        },
    ],
    reject: [
        {
            code: "a { opacity: 10%; }",
            fixed: "a { opacity: 0.1; }",
            message: expected("10%", "0.1"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 17,
        },
        {
            code: "a { color: rgba(0, 0, 0, 0.5) }",
            fixed: "a { color: rgba(0, 0, 0, 50%) }",
            message: expected("0.5", "50%"),
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 29,
        },
        {
            code: "a { color: rgba(0, 0, 0, 0.5); background-color: rgba(0, 0, 0, 0.7); }",
            fixed: "a { color: rgba(0, 0, 0, 50%); background-color: rgba(0, 0, 0, 70%); }",
            warnings: [
                {
                    message: expected("0.5", "50%"),
                    line: 1,
                    column: 26,
                    endLine: 1,
                    endColumn: 29,
                },
                {
                    message: expected("0.7", "70%"),
                    line: 1,
                    column: 64,
                    endLine: 1,
                    endColumn: 67,
                },
            ],
        },
    ],
}

test_rule! {
    name: scss_number,
    ruleName: RULE_NAME,
    config: ["number"],
    customSyntax: "postcss-scss",
    fix: true,
    accept: [
        {
            code: "$a: rgb(0, 0, 0);",
        },
        {
            code: "$a: rgb(0 0 0);",
        },
        {
            code: "$a: rgba(0, 0, 0, 0.5);",
        },
        {
            code: "$a: rgb(0 0 0 / 0.5);",
        },
        {
            code: "a { color: rgb(0 0 0 / $a) }",
        },
    ],
    reject: [
        {
            code: "$a: rgb(0 0 0 / 50%);",
            fixed: "$a: rgb(0 0 0 / 0.5);",
            message: expected("50%", "0.5"),
            line: 1,
            column: 17,
            endLine: 1,
            endColumn: 20,
        },
    ],
}

test_rule! {
    name: scss_percentage,
    ruleName: RULE_NAME,
    config: ["percentage"],
    customSyntax: "postcss-scss",
    fix: true,
    accept: [
        {
            code: "$a: rgba(0, 0, 0, 50%);",
        },
        {
            code: "$a: rgb(0 0 0 / 50%);",
        },
        {
            code: "$a: 50%; a { color: rgb(0 0 0 / $a) }",
        },
        {
            code: "$a: 0.5; a { color: rgb(0 0 0 / $a) }",
        },
    ],
    reject: [
        {
            code: "$a: rgb(0 0 0 / 0.5);",
            fixed: "$a: rgb(0 0 0 / 50%);",
            message: expected("0.5", "50%"),
            line: 1,
            column: 17,
            endLine: 1,
            endColumn: 20,
        },
    ],
}

test_rule! {
    name: number_compute_edit_info,
    ruleName: RULE_NAME,
    config: ["number"],
    computeEditInfo: true,
    accept: [],
    reject: [
        {
            code: "a { opacity: 10%; }",
            message: expected("10%", "0.1"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 17,
            fix: { range: [13, 16], text: "0.1" },
        },
        {
            code: "a { shape-image-threshold: 50% };",
            message: expected("50%", "0.5"),
            line: 1,
            column: 28,
            endLine: 1,
            endColumn: 31,
            fix: { range: [27, 30], text: "0.5" },
        },
        {
            code: "feDropShadow { flood-opacity: 50% }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 31,
            endLine: 1,
            endColumn: 34,
            fix: { range: [30, 33], text: "0.5" },
        },
        {
            code: "a { color: rgba(0, 0, 0, 50%) }",
            message: expected("50%", "0.5"),
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 29,
            fix: { range: [25, 28], text: "0.5" },
        },
        {
            code: "a { color: rgba(0, 0, 0, 50%); background-color: rgba(0, 0, 0, 70%); }",
            warnings: [
                {
                    message: expected("50%", "0.5"),
                    line: 1,
                    column: 26,
                    endLine: 1,
                    endColumn: 29,
                    fix: { range: [25, 28], text: "0.5" },
                },
                {
                    message: expected("70%", "0.7"),
                    line: 1,
                    column: 64,
                    endLine: 1,
                    endColumn: 67,
                    fix: { range: [63, 66], text: "0.7" },
                },
            ],
        },
        {
            code: "a { opacity: 10%; shape-image-threshold: 50%; }",
            warnings: [
                {
                    message: expected("10%", "0.1"),
                    line: 1,
                    column: 14,
                    endLine: 1,
                    endColumn: 17,
                    fix: { range: [13, 16], text: "0.1" },
                },
                {
                    message: expected("50%", "0.5"),
                    line: 1,
                    column: 42,
                    endLine: 1,
                    endColumn: 45,
                    fix: { range: [41, 44], text: "0.5" },
                },
            ],
        },
        {
            code: "@keyframes name { color: rgba(0, 0, 0, 45%); }",
            message: expected("45%", "0.45"),
            line: 1,
            column: 40,
            endLine: 1,
            endColumn: 43,
            fix: { range: [39, 42], text: "0.45" },
        },
        {
            code: "a { color: red; opacity: 30%; }",
            message: expected("30%", "0.3"),
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 29,
            fix: { range: [25, 28], text: "0.3" },
        },
    ],
}

test_rule! {
    name: percentage_compute_edit_info,
    ruleName: RULE_NAME,
    config: ["percentage"],
    computeEditInfo: true,
    accept: [],
    reject: [
        {
            code: "a { opacity: 0.1; }",
            message: expected("0.1", "10%"),
            line: 1,
            column: 14,
            endLine: 1,
            endColumn: 17,
            fix: { range: [13, 16], text: "10%" },
        },
        {
            code: "a { shape-image-threshold: 0.5 }",
            message: expected("0.5", "50%"),
            line: 1,
            column: 28,
            endLine: 1,
            endColumn: 31,
            fix: { range: [27, 30], text: "50%" },
        },
        {
            code: "feDropShadow { flood-opacity: 0.5 }",
            message: expected("0.5", "50%"),
            line: 1,
            column: 31,
            endLine: 1,
            endColumn: 34,
            fix: { range: [30, 33], text: "50%" },
        },
        {
            code: "a { color: rgba(0, 0, 0, 0.5) }",
            message: expected("0.5", "50%"),
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 29,
            fix: { range: [25, 28], text: "50%" },
        },
        {
            code: "a { color: rgba(0, 0, 0, 0.5); background-color: rgba(0, 0, 0, 0.7); }",
            warnings: [
                {
                    message: expected("0.5", "50%"),
                    line: 1,
                    column: 26,
                    endLine: 1,
                    endColumn: 29,
                    fix: { range: [25, 28], text: "50%" },
                },
                {
                    message: expected("0.7", "70%"),
                    line: 1,
                    column: 64,
                    endLine: 1,
                    endColumn: 67,
                    fix: { range: [63, 66], text: "70%" },
                },
            ],
        },
        {
            code: "a { opacity: 0.1; shape-image-threshold: 0.5; }",
            warnings: [
                {
                    message: expected("0.1", "10%"),
                    line: 1,
                    column: 14,
                    endLine: 1,
                    endColumn: 17,
                    fix: { range: [13, 16], text: "10%" },
                },
                {
                    message: expected("0.5", "50%"),
                    line: 1,
                    column: 42,
                    endLine: 1,
                    endColumn: 45,
                    fix: { range: [41, 44], text: "50%" },
                },
            ],
        },
        {
            code: "a { color: red; opacity: 0.3; }",
            message: expected("0.3", "30%"),
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 29,
            fix: { range: [25, 28], text: "30%" },
        },
    ],
}
