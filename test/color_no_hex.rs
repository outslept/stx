fn rejected(hex: &str) -> String {
    format!("Disallowed hex color \"{hex}\"")
}

test_rule! {
    name: default,
    ruleName: "color-no-hex",
    config: [true],

    accept: [
        {
            code: "a { color: pink; }",
        },
        {
            code: "a { color: rgba(0, 0, 0, 0); }",
        },
        {
            code: "a { something: black, white, gray; }",
        },
        {
            code: "a { padding: 000; }",
        },
        {
            code: "a::before { content: \"#ababa\"; }",
        },
        {
            code: "a { border-#$side: 0; }",
            description: "ignore sass-like interpolation",
        },
        {
            code: "a { box-sizing: #$type-box; }",
            description: "ignore sass-like interpolation",
        },
        {
            code: "@font-face {\nfont-family: dashicons;\nsrc: url(data:application/font-woff;charset=utf-8;base64, ABCDEF==) format(\"woff\"),\nurl(../fonts/dashicons.ttf) format(\"truetype\"),\nurl(../fonts/dashicons.svg#dashicons) format(\"svg\");\nfont-weight: normal;\nfont-style: normal;\n}",
        },
    ],

    reject: [
        {
            code: "a { color: #12345; }",
            message: rejected("#12345"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: "a { color: #123456a; }",
            message: rejected("#123456a"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 20,
        },
        {
            code: "a { color: #cccccc; }",
            message: rejected("#cccccc"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 19,
        },
        {
            code: "a { something: #00c, red, white; }",
            message: rejected("#00c"),
            line: 1,
            column: 16,
            endLine: 1,
            endColumn: 20,
        },
        {
            code: "a { something: black, #fff1a1, rgb(250, 250, 0); }",
            message: rejected("#fff1a1"),
            line: 1,
            column: 23,
            endLine: 1,
            endColumn: 30,
        },
        {
            code: "a { something:black,white,#12345a; }",
            message: rejected("#12345a"),
            line: 1,
            column: 27,
            endLine: 1,
            endColumn: 34,
        },
        {
            code: "a { color: #ffff; }",
            message: rejected("#ffff"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 17,
        },
        {
            code: "a { color: #ffffffaa; }",
            message: rejected("#ffffffaa"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 21,
        },
    ],
}

test_rule! {
    name: css_in_js,
    ruleName: "color-no-hex",
    config: [true],
    customSyntax: "naive-css-in-js",
    skip: true,

    accept: [
        {
            code: "css` color: pink; `;",
        },
        {
            code: "css` color: rgba(0, 0, 0, 0); `;",
        },
        {
            code: "css` content: \"#abcdef\"; `;",
        },
        {
            code: "css` background: url(#aabbcc); `;",
        },
        {
            code: "css` color: red /* #ff0000 */; `;",
        },
        {
            code: "css` color: linear-gradient(green, rgb(123, 123, 123), hsl(24, 70%, 80%)); `;",
        },
    ],

    reject: [
        {
            code: "css` color: #abcdef `;",
            message: rejected("#abcdef"),
        },
        {
            code: "css` background-color: linear-gradient(#aaa, #ffff, #01234567); `;",
            warnings: [
                { message: rejected("#aaa") },
                { message: rejected("#ffff") },
                { message: rejected("#01234567") },
            ],
        },
    ],
}

test_rule! {
    name: ignore_functions,
    ruleName: "color-no-hex",
    config: [true, { "ignoreFunctions": ["var", "/^--/"] }],

    accept: [
        {
            code: "a { color: var(--foo, #fff); }",
        },
        {
            code: "a { color: --foo(#fff); }",
        },
    ],

    reject: [
        {
            code: "a { color: #ffffff; }",
            message: rejected("#ffffff"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 19,
        },
    ],
}
