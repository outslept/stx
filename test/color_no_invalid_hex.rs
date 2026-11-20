fn rejected(hex: &str) -> String {
    format!("Invalid hex color \"{hex}\"")
}

test_rule! {
    name: default,
    ruleName: "color-no-invalid-hex",
    config: [true],

    accept: [
        {
            code: "a { color: pink; }",
        },
        {
            code: "a { color: #000; }",
        },
        {
            code: "a { something: #000, #fff, #ababab; }",
        },
        {
            code: "a { color: #0000ffcc; }",
            description: "eight digits",
        },
        {
            code: "a { color:#00fc; }",
            description: "four digits",
        },
        {
            code: "a { padding: 000; }",
        },
        {
            code: "a::before { content: \"#ababa\"; }",
        },
        {
            code: "a { background-image: svg-load('x.svg', fill=url(#a)); }",
            description: "svg-load url with fill",
        },
        {
            code: "a { background-image: url(#a); }",
            description: "url standalone hash",
        },
        {
            code: "a { background-image: url(x.svg#a); }",
            description: "url with hash",
        },
        {
            code: "@font-face {\nfont-family: dashicons;\nsrc: url(data:application/font-woff;charset=utf-8;base64, ABCDEF==) format(\"woff\"),\nurl(../fonts/dashicons.ttf) format(\"truetype\"),\nurl(../fonts/dashicons.svg#dashicons) format(\"svg\");\nfont-weight: normal;\nfont-style: normal;\n}",
        },
    ],

    reject: [
        {
            code: "a { color: #ababa; }",
            message: rejected("#ababa"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: "a { something: #00, #fff, #ababab; }",
            message: rejected("#00"),
            line: 1,
            column: 16,
            endLine: 1,
            endColumn: 19,
        },
        {
            code: "a { something: #000, #fff1az, #ababab; }",
            message: rejected("#fff1az"),
            line: 1,
            column: 22,
            endLine: 1,
            endColumn: 29,
        },
        {
            code: "a { something:#000,#fff,#12345aa; }",
            message: rejected("#12345aa"),
            line: 1,
            column: 25,
            endLine: 1,
            endColumn: 33,
        },
    ],
}

test_rule! {
    name: less,
    ruleName: "color-no-invalid-hex",
    config: [true],
    customSyntax: "postcss-less",

    accept: [
        {
            code: "a { color: #colors[somecolor]; }",
            description: "Less map usage",
        },
    ],
}

test_rule! {
    name: scss,
    ruleName: "color-no-invalid-hex",
    config: [true],
    customSyntax: "postcss-scss",

    accept: [
        {
            code: "a { border-#$side: 0; }",
            description: "ignore sass-like interpolation",
        },
        {
            code: "a { box-sizing: #$type-box; }",
            description: "ignore sass-like interpolation",
        },
    ],
}

test_rule! {
    name: css_in_js,
    ruleName: "color-no-invalid-hex",
    config: [true],
    customSyntax: "naive-css-in-js",
    skip: true,

    accept: [
        {
            code: "css` color: #ffff; `;",
        },
    ],

    reject: [
        {
            code: "css` color: #fffff; `;",
            message: rejected("#fffff"),
        },
    ],
}
