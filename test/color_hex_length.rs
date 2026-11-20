fn expected(actual: &str, expected: &str) -> String {
    format!("Expected \"{actual}\" to be \"{expected}\"")
}

test_rule! {
    name: short,
    ruleName: "color-hex-length",
    config: ["short"],
    fix: true,
    computeEditInfo: true,

    accept: [
        {
            code: "a { color: pink; }",
        },
        {
            code: "a { color: #000; }",
        },
        {
            code: "a { color: #FFF; }",
        },
        {
            code: "a { color: #f00; }",
            description: "mixed case",
        },
        {
            code: "a { color: #0000; }",
        },
        {
            code: "a { color: #FFFF; }",
        },
        {
            code: "a { color: #f000; }",
            description: "mixed case",
        },
        {
            code: "a { color: #00ff00; }",
        },
        {
            code: "a { color: #00ff00ff; }",
        },
        {
            code: "a { something: #000, #fff, #aba; }",
        },
        {
            code: "a { color: #ff; }",
            description: "invalid short",
        },
        {
            code: "a { color: #ffffffa; }",
            description: "invalid long",
        },
        {
            code: "a { color: #fffffffffff; }",
            description: "invalid extra long",
        },
        {
            code: "a { padding: 000000; }",
        },
        {
            code: "a::before { content: \"#ABABAB\"; }",
        },
        {
            code: "a { color: white /* #FFFFFF */; }",
        },
        {
            code: "a { background: url(somefile.swvg#abcdef)}",
        },
    ],

    reject: [
        {
            code: "a { color: #FFFFFF; }",
            fixed: "a { color: #FFF; }",
            message: expected("#FFFFFF", "#FFF"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 19,
            fix: {
                range: [14, 18],
                text: "F",
            },
        },
        {
            code: "a { color: #FfaAFF; }",
            fixed: "a { color: #FaF; }",
            message: expected("#FfaAFF", "#FaF"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 19,
            fix: {
                range: [13, 17],
                text: "a",
            },
        },
        {
            code: "a { color: #00aa00aa; }",
            fixed: "a { color: #0a0a; }",
            message: expected("#00aa00aa", "#0a0a"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 21,
            fix: {
                range: [13, 19],
                text: "a0",
            },
        },
        {
            code: "a { something: #000, #fff, #ababab; }",
            fixed: "a { something: #000, #fff, #aba; }",
            message: expected("#ababab", "#aba"),
            line: 1,
            column: 28,
            endLine: 1,
            endColumn: 35,
            fix: {
                range: [30, 34],
                text: "a",
            },
        },
    ],
}

test_rule! {
    name: long,
    ruleName: "color-hex-length",
    config: ["long"],
    fix: true,
    computeEditInfo: true,

    accept: [
        {
            code: "a { color: pink; }",
        },
        {
            code: "a { color: #000000; }",
        },
        {
            code: "a { color: #FFFFFF; }",
        },
        {
            code: "a { color: #f00000; }",
            description: "mixed case",
        },
        {
            code: "a { color: #00000000; }",
        },
        {
            code: "a { color: #FFFFFFFF; }",
        },
        {
            code: "a { color: #f0000000; }",
            description: "mixed case",
        },
        {
            code: "a { something: #000000, #ffffff, #ababab; }",
        },
        {
            code: "a { color: #ff; }",
            description: "invalid short",
        },
        {
            code: "a { color: #ffffffa; }",
            description: "invalid long",
        },
        {
            code: "a { color: #fffffffffff; }",
            description: "invalid extra long",
        },
        {
            code: "a { padding: 000; }",
        },
        {
            code: "a::before { content: \"#ABA\"; }",
        },
        {
            code: "a { color: white /* #FFF */; }",
        },
        {
            code: "a { background: url(somefile.swvg#abc)}",
        },
        {
            code: "a { background: url('data:image/svg+xml;utf8,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><path fill=\"#000\" d=\"M32 2a24 24 0 0 0-24 24c0 16.2 21.6 34.8 22.5 35.5a2.4 2.4 0 0 0 3 0C34.4 60.8 56 42.2 56 26A24 24 0 0 0 32 2zm0 32a8 8 0 1 1 8-8 8 8 0 0 1-8 8z\"/></svg>'); }",
            description: "ignores data URI containing `#`",
        },
    ],

    reject: [
        {
            code: "a { color: #FFF; }",
            fixed: "a { color: #FFFFFF; }",
            message: expected("#FFF", "#FFFFFF"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 16,
            fix: {
                range: [15, 16],
                text: "FFFF",
            },
        },
        {
            code: "a { color: #Ffa; }",
            fixed: "a { color: #FFffaa; }",
            message: expected("#Ffa", "#FFffaa"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 16,
            fix: {
                range: [13, 16],
                text: "FFffaa",
            },
        },
        {
            code: "a { color: #0a0a; }",
            fixed: "a { color: #00aa00aa; }",
            message: expected("#0a0a", "#00aa00aa"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 17,
            fix: {
                range: [13, 17],
                text: "00aa00aa",
            },
        },
        {
            code: "a { something: #000000, #ffffff, #aba; }",
            fixed: "a { something: #000000, #ffffff, #aabbaa; }",
            message: expected("#aba", "#aabbaa"),
            line: 1,
            column: 34,
            endLine: 1,
            endColumn: 38,
            fix: {
                range: [35, 38],
                text: "aabbaa",
            },
        },
        {
            code: "a { background: url('data:image/svg+xml;utf8,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><path fill=\"#000\" d=\"M32 2a24 24 0 0 0-24 24c0 16.2 21.6 34.8 22.5 35.5a2.4 2.4 0 0 0 3 0C34.4 60.8 56 42.2 56 26A24 24 0 0 0 32 2zm0 32a8 8 0 1 1 8-8 8 8 0 0 1-8 8z\"/></svg>') #fff; }",
            fixed: "a { background: url('data:image/svg+xml;utf8,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><path fill=\"#000\" d=\"M32 2a24 24 0 0 0-24 24c0 16.2 21.6 34.8 22.5 35.5a2.4 2.4 0 0 0 3 0C34.4 60.8 56 42.2 56 26A24 24 0 0 0 32 2zm0 32a8 8 0 1 1 8-8 8 8 0 0 1-8 8z\"/></svg>') #ffffff; }",
            description: "ignores data URI containing `#`, but checks subsequent hex",
            message: expected("#fff", "#ffffff"),
            line: 1,
            column: 289,
            endLine: 1,
            endColumn: 293,
            fix: {
                range: [291, 292],
                text: "ffff",
            },
        },
    ],
}

test_rule! {
    name: short_css_in_js,
    ruleName: "color-hex-length",
    config: ["short"],
    customSyntax: "naive-css-in-js",
    fix: true,
    skip: true,

    accept: [
        {
            code: "export default <svg style={{ color: \"#fff\" }} />;",
        },
    ],

    reject: [
        {
            code: "export default <svg style={{ color: \"#ffffff\" }} />;",
            fixed: "export default <svg style={{ color: \"#fff\" }} />;",
            message: expected("#ffffff", "#fff"),
        },
    ],
}
