fn rejected(property: &str) -> String {
    format!("Duplicate property \"{property}\"")
}

test_rule! {
    name: declaration_block_no_duplicate_properties_default,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true],
    fix: true,
    accept: [
        {
            code: "a { color: pink; }",
        },
        {
            code: "a { color: pink; background: orange; }",
        },
        {
            code: "a { color: pink; { &:hover { color: orange; } } }",
            description: "spec nested",
        },
        {
            code: "a { color: pink; @media { color: orange; } }",
            description: "nested",
        },
        {
            code: "a { color: pink; @media { color: orange; &::before { color: black; } } }",
            description: "double nested",
        },
        {
            code: "a { $scss: 0; $scss: $scss + 1; }",
        },
        {
            code: "a { @less: 0; @less: @less + 1; }",
        },
        {
            code: "a { --custom-property: 0; --custom-property: 1; }",
        },
        {
            code: "@fontface { src: url(font.eof); src: url(font.woff) }",
        },
        {
            code: "@fontface { sRc: url(font.eof); sRc: url(font.woff) }",
        },
        {
            code: "@fontface { SRC: url(font.eof); SRC: url(font.woff) }",
        },
    ],
    reject: [
        {
            code: "a { color: pink; color: orange }",
            fixed: "a { color: orange }",
            message: rejected("color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "a { cOlOr: pink; color: orange }",
            fixed: "a { color: orange }",
            message: rejected("cOlOr"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "a { color: pink; cOlOr: orange }",
            fixed: "a { cOlOr: orange }",
            message: rejected("color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "a { cOlOr: pink; cOlOr: orange }",
            fixed: "a { cOlOr: orange }",
            message: rejected("cOlOr"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "a { COLOR: pink; color: orange }",
            fixed: "a { color: orange }",
            message: rejected("COLOR"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "a { color: pink; COLOR: orange }",
            fixed: "a { COLOR: orange }",
            message: rejected("color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "a { COlOr: pink; coLOR: pink; color: pink }",
            fixed: "a { color: pink }",
            description: "multiple duplicates - same",
            warnings: [
                {
                    message: rejected("COlOr"),
                    line: 1,
                    column: 5,
                    endLine: 1,
                    endColumn: 10,
                },
                {
                    message: rejected("coLOR"),
                    line: 1,
                    column: 18,
                    endLine: 1,
                    endColumn: 23,
                },
            ],
        },
        {
            code: "a { color: pink; color: pink; color: orange }",
            fixed: "a { color: orange }",
            description: "multiple duplicates - diff",
            warnings: [
                {
                    message: rejected("color"),
                    line: 1,
                    column: 5,
                    endLine: 1,
                    endColumn: 10,
                },
                {
                    message: rejected("color"),
                    line: 1,
                    column: 18,
                    endLine: 1,
                    endColumn: 23,
                },
            ],
        },
        {
            code: "a { color: pink; background: orange; color: orange }",
            fixed: "a { background: orange; color: orange }",
            message: rejected("color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "a { color: pink; background: orange; background: pink; }",
            fixed: "a { color: pink; background: pink; }",
            message: rejected("background"),
            line: 1,
            column: 18,
            endLine: 1,
            endColumn: 28,
        },
        {
            code: "a { color: pink; { &:hover { color: orange; color: black; } } }",
            fixed: "a { color: pink; { &:hover { color: black; } } }",
            description: "spec nested",
            message: rejected("color"),
            line: 1,
            column: 30,
            endLine: 1,
            endColumn: 35,
        },
        {
            code: "a { color: pink; @media { color: orange; color: black; } }",
            fixed: "a { color: pink; @media { color: black; } }",
            description: "nested",
            message: rejected("color"),
            line: 1,
            column: 27,
            endLine: 1,
            endColumn: 32,
        },
        {
            code: "@media { color: orange; .foo { color: black; color: white; } }",
            fixed: "@media { color: orange; .foo { color: white; } }",
            description: "nested",
            message: rejected("color"),
            line: 1,
            column: 32,
            endLine: 1,
            endColumn: 37,
        },
        {
            code: "a { color: pink; @media { color: orange; &::before { color: black; color: white; } } }",
            fixed: "a { color: pink; @media { color: orange; &::before { color: white; } } }",
            description: "double nested",
            message: rejected("color"),
            line: 1,
            column: 54,
            endLine: 1,
            endColumn: 59,
        },
        {
            code: "a { color: pink; @media { color: orange; .foo { color: black; color: white; } } }",
            fixed: "a { color: pink; @media { color: orange; .foo { color: white; } } }",
            description: "double nested again",
            message: rejected("color"),
            line: 1,
            column: 49,
            endLine: 1,
            endColumn: 54,
        },
        {
            code: "a { -webkit-border-radius: 12px; -webkit-border-radius: 10px; }",
            fixed: "a { -webkit-border-radius: 10px; }",
            message: rejected("-webkit-border-radius"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 26,
        },
        {
            code: "a { -WEBKIT-border-radius: 12px; -webkit-BORDER-radius: 10px; }",
            fixed: "a { -webkit-BORDER-radius: 10px; }",
            message: rejected("-WEBKIT-border-radius"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 26,
        },
        {
            code: "a { color: red !important; color: blue; }",
            fixed: "a { color: red !important; }",
            message: rejected("color"),
            line: 1,
            column: 28,
            endLine: 1,
            endColumn: 33,
        },
        {
            code: "a { color: red !important; color: blue !important; }",
            fixed: "a { color: blue !important; }",
            message: rejected("color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "a { scrollbar-color: rebeccapurple green; scrollbar-color: rebeccapurple transparent; }",
            fixed: "a { scrollbar-color: rebeccapurple transparent; }",
            message: rejected("scrollbar-color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 20,
        },
        {
            code: "p { border-color: yellow green #000 red; border-color: black; }",
            fixed: "p { border-color: black; }",
            message: rejected("border-color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 17,
        },
        {
            code: "p { animation-name: red; animation-name: black; }",
            fixed: "p { animation-name: black; }",
            message: rejected("animation-name"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 19,
        },
        {
            code: "a { border-color: red yellow; border-color: green; }",
            fixed: "a { border-color: green; }",
            message: rejected("border-color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 17,
            description: "different number of values should be different syntaxes",
        },
        {
            code: "a { scrollbar-color: red #000; scrollbar-color: blue; }",
            fixed: "a { scrollbar-color: blue; }",
            message: rejected("scrollbar-color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 20,
            description: "different number of values should be different syntaxes",
        },
        {
            code: "a { border-color: red #000; border-color: blue green; }",
            fixed: "a { border-color: blue green; }",
            message: rejected("border-color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 17,
            description: "mixed named color and hex should be different syntaxes",
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_edit_info,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true],
    computeEditInfo: true,
    accept: [],
    reject: [
        {
            code: "a { color: pink; color: orange }",
            message: rejected("color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
            fix: { range: [11, 24], text: "" },
        },
        {
            code: "a { COlOr: pink; coLOR: pink; color: pink }",
            description: "multiple duplicates - same",
            warnings: [
                {
                    message: rejected("COlOr"),
                    line: 1,
                    column: 5,
                    endLine: 1,
                    endColumn: 10,
                    fix: { range: [4, 17], text: "" },
                },
                {
                    message: rejected("coLOR"),
                    line: 1,
                    column: 18,
                    endLine: 1,
                    endColumn: 23,
                },
            ],
        },
        {
            code: "a { color: pink; background: orange; color: orange }",
            message: rejected("color"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
            fix: { range: [4, 17], text: "" },
        },
        {
            code: "a { color: pink; background: orange; background: pink; }",
            message: rejected("background"),
            line: 1,
            column: 18,
            endLine: 1,
            endColumn: 28,
            fix: { range: [29, 49], text: "" },
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_ignore_consecutive,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true, { "ignore": ["consecutive-duplicates"] }],
    fix: true,
    accept: [
        {
            code: "p { font-size: 16px; font-size: 1rem; }",
        },
        {
            code: "p { display: inline-block; font-size: 16px; font-size: 1rem; }",
        },
        {
            code: "p { font-size: 16px; font-size: 1rem; color: red; }",
        },
        {
            code: "p { display: inline-block; font-size: 16px; font-size: 1rem; color: red; }",
        },
    ],
    reject: [
        {
            code: "p { font-size: 16px; font-weight: 400; font-size: 1rem; }",
            fixed: "p { font-weight: 400; font-size: 1rem; }",
            message: rejected("font-size"),
        },
        {
            code: "p { display: inline-block; font-size: 16px; font-weight: 400; font-size: 1rem; }",
            fixed: "p { display: inline-block; font-weight: 400; font-size: 1rem; }",
            message: rejected("font-size"),
        },
        {
            code: "p { font-size: 16px; font-weight: 400; font-size: 1rem; color: red; }",
            fixed: "p { font-weight: 400; font-size: 1rem; color: red; }",
            message: rejected("font-size"),
        },
        {
            code: "p { display: inline-block; font-size: 16px; font-weight: 400; font-size: 1rem; color: red; }",
            fixed: "p { display: inline-block; font-weight: 400; font-size: 1rem; color: red; }",
            message: rejected("font-size"),
        },
        {
            code: "p { font-size: 16px !important; font-weight: 400; font-size: 1rem; }",
            fixed: "p { font-size: 16px !important; font-weight: 400; }",
            message: rejected("font-size"),
        },
        {
            code: "a { font-size: 16px; color: red; font-size: 1rem; color: blue; }",
            fixed: "a { font-size: 1rem; color: blue; }",
            description: "two interleaved properties, each with non-consecutive duplicates",
            warnings: [
                {
                    message: rejected("font-size"),
                    line: 1,
                    column: 5,
                    endLine: 1,
                    endColumn: 14,
                },
                {
                    message: rejected("color"),
                    line: 1,
                    column: 22,
                    endLine: 1,
                    endColumn: 27,
                },
            ],
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_ignore_consecutive_edit_info,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true, { "ignore": ["consecutive-duplicates"] }],
    computeEditInfo: true,
    accept: [],
    reject: [
        {
            code: "p { font-size: 16px; font-weight: 400; font-size: 1rem; }",
            message: rejected("font-size"),
            fix: { range: [9, 26], text: "" },
        },
        {
            code: "p { font-size: 16px !important; font-weight: 400; font-size: 1rem; }",
            message: rejected("font-size"),
            fix: { range: [50, 67], text: "" },
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_ignore_diff_values,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true, { "ignore": ["consecutive-duplicates-with-different-values"] }],
    fix: true,
    accept: [
        { code: "p { font-size: 16px; font-size: 1rem; font-weight: 400; }" },
        { code: "p { font-size: 16px; font-size: 18px; font-weight: 400; }" },
    ],
    reject: [
        {
            code: "p { font-size: 16px; font-weight: 400; font-size: 1rem; }",
            fixed: "p { font-weight: 400; font-size: 1rem; }",
            message: rejected("font-size"),
        },
        {
            code: "p { font-size: 16px; font-size: 16px; font-weight: 400; }",
            fixed: "p { font-size: 16px; font-weight: 400; }",
            message: rejected("font-size"),
        },
        {
            code: "p { font-size: 16px !important; font-weight: 400; font-size: 1rem; }",
            fixed: "p { font-size: 16px !important; font-weight: 400; }",
            message: rejected("font-size"),
            line: 1,
            column: 51,
            endLine: 1,
            endColumn: 60,
        },
        {
            code: "p { font-size: 16px; font-size: 16px !important; font-weight: 400; }",
            fixed: "p { font-size: 16px !important; font-weight: 400; }",
            message: rejected("font-size"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 14,
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_ignore_diff_syntaxes,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true, { "ignore": ["consecutive-duplicates-with-different-syntaxes"] }],
    fix: true,
    accept: [
        { code: "p { width: 100vw; height: 100vh; }" },
        { code: "p { width: 100vw; width: 100dvw; height: 100vh; }" },
        { code: "p { margin: 10dvw 10dvw; margin: 10vw 10vw; padding: 0; }" },
        { code: "p { margin: 10dvh 10dvw 10dvh; margin: 10vw 10vw; padding: 0; }" },
        { code: "p { width: 100%; width: fit-content; }" },
        { code: "p { width: min-content; width: max-content; }" },
        { code: "p { width: calc(10px + 2px); width: calc(10px + 2rem); }" },
        { code: "p { width: calc(10px + 2px); width: calc(10rem + 2rem); }" },
        { code: "p { width: min(10px, 11px); width: max(10px, 11px); }" },
        { code: "p { width: calc((10px + 2px)); width: calc((10rem + 2rem)); }" },
        { code: "p { width: calc((10px + 2px) + 10px); width: calc((10rem + 2rem) + 10px); }" },
        { code: "p { width: $a; width: calc(1 + $a); }", description: "using SCSS variables" },
        { code: "p { width: _$a; width: _$a2; }", description: "using invalid value" },
        { code: "p { width: env(foo); width: env(--bar); }" },
        { code: "p { width: env(--foo); width: env(bar); }" },
    ],
    reject: [
        {
            code: "p { width: 100vw ; width: 100vw; }",
            fixed: "p { width: 100vw; }",
            message: rejected("width"),
        },
        {
            code: "p { width: calc(100vw /* a comment */  + 10vw); width: calc(100vw + 10vw); }",
            fixed: "p { width: calc(100vw + 10vw); }",
            message: rejected("width"),
        },
        {
            code: "p { width: 100vw; height: 100vh; width: 100dvw; }",
            fixed: "p { height: 100vh; width: 100dvw; }",
            message: rejected("width"),
        },
        {
            code: "p { width: 100vw; width: 100vw; height: 100vh; }",
            fixed: "p { width: 100vw; height: 100vh; }",
            message: rejected("width"),
        },
        {
            code: "p { margin: 10vw 10vw; margin: 10vw 10vw; padding: 0; }",
            fixed: "p { margin: 10vw 10vw; padding: 0; }",
            message: rejected("margin"),
        },
        {
            code: "p { width: 100vw; width: 50vw; height: 100vh; }",
            fixed: "p { width: 50vw; height: 100vh; }",
            message: rejected("width"),
        },
        {
            code: "p { width: 100vw !important; height: 100vh; width: 100dvw; }",
            fixed: "p { width: 100vw !important; height: 100vh; }",
            message: rejected("width"),
        },
        {
            code: "p { width: 100vw; height: 100vh; width: 100dvw !important; }",
            fixed: "p { height: 100vh; width: 100dvw !important; }",
            message: rejected("width"),
        },
        {
            code: "p { width: calc(10px + 4rem); width: calc(10px + 2rem); }",
            fixed: "p { width: calc(10px + 2rem); }",
            message: rejected("width"),
        },
        {
            code: "p { width: CaLC(10px + 4rem); width: calc(10px + 2rem); }",
            fixed: "p { width: calc(10px + 2rem); }",
            message: rejected("width"),
        },
        {
            code: "p { width: var(--foo); width: var(--bar); }",
            fixed: "p { width: var(--bar); }",
            message: rejected("width"),
        },
        {
            code: "p { content: \"foo\"; content: \"bar\"; }",
            fixed: "p { content: \"bar\"; }",
            message: rejected("content"),
        },
        {
            code: "p { image: url(#foo); image: url(#bar); }",
            fixed: "p { image: url(#bar); }",
            message: rejected("image"),
        },
        {
            code: "p { color: red; color: blue; }",
            fixed: "p { color: blue; }",
            message: rejected("color"),
        },
        {
            code: "p { color: red; color: green; }",
            fixed: "p { color: green; }",
            message: rejected("color"),
        },
        {
            code: "p { background-color: red; background-color: blue; }",
            fixed: "p { background-color: blue; }",
            message: rejected("background-color"),
        },
        {
            code: "p { color: red; background: orange; color: blue; }",
            fixed: "p { background: orange; color: blue; }",
            message: rejected("color"),
        },
        {
            code: "a { scrollbar-color: red blue; scrollbar-color: green yellow; }",
            fixed: "a { scrollbar-color: green yellow; }",
            message: rejected("scrollbar-color"),
        },
        {
            code: "a { border-color: red green blue yellow; border-color: black white gray silver; }",
            fixed: "a { border-color: black white gray silver; }",
            message: rejected("border-color"),
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_ignore_prefixless,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true, { "ignore": ["consecutive-duplicates-with-same-prefixless-values"] }],
    fix: true,
    accept: [
        {
            code: "p { width: -moz-fit-content; width: fit-content; }",
        },
        {
            code: "p { width: fit-content; width: -moz-fit-content; }",
        },
        {
            code: "p { width: -MOZ-fit-content; width: fit-content; }",
        },
        {
            code: "p { width: -webkit-fit-content; width: -moz-fit-content; width: fit-content; }",
        },
    ],
    reject: [
        {
            code: "p { width: fit-content; height: 32px; width: -moz-fit-content; }",
            fixed: "p { height: 32px; width: -moz-fit-content; }",
            message: rejected("width"),
        },
        {
            code: "p { width: 100%; width: -moz-fit-content; height: 32px; }",
            fixed: "p { width: -moz-fit-content; height: 32px; }",
            message: rejected("width"),
        },
        {
            code: "p { width: -moz-fit-content; width: -moz-fit-content; }",
            fixed: "p { width: -moz-fit-content; }",
            message: rejected("width"),
        },
        {
            code: "p { width: -moz-fit-content; width: -moz-fit-content !important; }",
            fixed: "p { width: -moz-fit-content !important; }",
            message: rejected("width"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: "p { width: 100% !important; width: -moz-fit-content; }",
            fixed: "p { width: 100% !important; }",
            message: rejected("width"),
            line: 1,
            column: 29,
            endLine: 1,
            endColumn: 34,
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_ignore_diff_values_edit_info,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true, { "ignore": ["consecutive-duplicates-with-different-values"] }],
    computeEditInfo: true,
    accept: [],
    reject: [
        {
            code: "p { font-size: 16px; font-size: 16px !important; font-weight: 400; }",
            message: rejected("font-size"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 14,
            fix: { range: [19, 36], text: "" },
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_ignore_properties_string,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true, { "ignoreProperties": ["color"] }],
    fix: true,
    accept: [
        {
            code: "p { color: pink; color: orange; }",
        },
        {
            code: "p { COLOR: pink; color: orange; }",
        },
        {
            code: "p { color: pink; background: orange; color: orange }",
        },
        {
            code: "p { color: pink; color: orange !important; }",
        },
    ],
    reject: [
        {
            code: "p { color: pink; background: orange; background: white; }",
            fixed: "p { color: pink; background: white; }",
            message: rejected("background"),
        },
        {
            code: "p { background: orange; color: pink; background: white; }",
            fixed: "p { color: pink; background: white; }",
            message: rejected("background"),
        },
        {
            code: "p { background: orange; color: pink; background: white !important; }",
            fixed: "p { color: pink; background: white !important; }",
            message: rejected("background"),
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_ignore_properties_regex,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true, { "ignoreProperties": ["/background-/", "/padding-/"] }],
    fix: true,
    accept: [
        {
            code: "p { background-color: pink; background-color: orange; }",
        },
        {
            code: "p { background-color: pink; color: orange; background-color: orange }",
        },
        {
            code: "p { padding-left: auto; padding-left: 1rem; }",
        },
    ],
    reject: [
        {
            code: "p { color: pink; background: orange; background: white; }",
            fixed: "p { color: pink; background: white; }",
            message: rejected("background"),
        },
        {
            code: "p { background: orange; color: pink; background: white; }",
            fixed: "p { color: pink; background: white; }",
            message: rejected("background"),
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_html,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true],
    customSyntax: "postcss-html",
    fix: true,
    skip: true,
    accept: [
        {
            code: "<style>a { color: pink; }</style>",
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_properties_css_in_js,
    ruleName: "declaration-block-no-duplicate-properties",
    config: [true],
    customSyntax: "naive-css-in-js",
    skip: true,
    accept: [
        {
            code: "css` color: pink; `;",
        },
    ],
}
