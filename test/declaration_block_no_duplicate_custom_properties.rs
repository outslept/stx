fn rejected(property: &str) -> String {
    format!("Duplicate custom property \"{property}\"")
}

test_rule! {
    name: declaration_block_no_duplicate_custom_properties_default,
    ruleName: "declaration-block-no-duplicate-custom-properties",
    config: [true],
    accept: [
        {
            code: "a { --custom-property: 1 }",
        },
        {
            code: "a { --custom-property: 1; --cUstOm-prOpErtY: 1 }",
        },
        {
            code: "a { --custom-property: 1; color: pink; --cUstOm-prOpErtY: 1 }",
        },
        {
            code: "a { color: var(--custom-property, --custom-property) }",
        },
        {
            code: "a { --custom-property: pink; @media { --custom-property: orange; } }",
            description: "nested",
        },
        {
            code: "a { --custom-property: pink; @media { --custom-property: orange; &::before { --custom-property: black; } } }",
            description: "double nested",
        },
        {
            code: "a { --custom-property: pink; { &:hover { --custom-property: orange; --cUstOm-prOpErtY: black; } } }",
            description: "spec nested",
        },
        {
            code: "a { --cUstOm-prOpErtY: pink; { &:hover { --custom-property: orange; --cUstOm-prOpErtY: black; } } }",
            description: "spec nested",
        },
    ],
    reject: [
        {
            code: "a { --custom-property: 1; --custom-property: 2; }",
            message: rejected("--custom-property"),
            line: 1,
            column: 27,
            endLine: 1,
            endColumn: 44,
        },
        {
            code: "a { --custom-property: 1; color: pink; --custom-property: 1; }",
            message: rejected("--custom-property"),
            line: 1,
            column: 40,
            endLine: 1,
            endColumn: 57,
        },
        {
            code: "a { --custom-property: 1; --cUstOm-prOpErtY: 1; color: pink; --cUstOm-prOpErtY: 1; }",
            message: rejected("--cUstOm-prOpErtY"),
            line: 1,
            column: 62,
            endLine: 1,
            endColumn: 79,
        },
        {
            code: "a { --custom-property: pink; { &:hover { --custom-property: orange; --custom-property: black; } } }",
            description: "spec nested",
            message: rejected("--custom-property"),
            line: 1,
            column: 69,
            endLine: 1,
            endColumn: 86,
        },
        {
            code: "a { --custom-property: pink; @media { --custom-property: orange; --custom-property: black; } }",
            description: "nested",
            message: rejected("--custom-property"),
            line: 1,
            column: 66,
            endLine: 1,
            endColumn: 83,
        },
        {
            code: "@media { --custom-property: orange; .foo { --custom-property: black; --custom-property: white; } }",
            description: "nested",
            message: rejected("--custom-property"),
            line: 1,
            column: 70,
            endLine: 1,
            endColumn: 87,
        },
        {
            code: "a { --custom-property: pink; @media { --custom-property: orange; &::before { --custom-property: black; --custom-property: white; } } }",
            description: "double nested",
            message: rejected("--custom-property"),
            line: 1,
            column: 104,
            endLine: 1,
            endColumn: 121,
        },
        {
            code: "a { --custom-property: pink; @media { --custom-property: orange; .foo { --custom-property: black; --custom-property: white; } } }",
            description: "double nested again",
            message: rejected("--custom-property"),
            line: 1,
            column: 99,
            endLine: 1,
            endColumn: 116,
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_custom_properties_html,
    ruleName: "declaration-block-no-duplicate-custom-properties",
    config: [true],
    customSyntax: "postcss-html",
    skip: true,
    accept: [
        {
            code: "<style>a { --custom-property: pink; }</style>",
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_custom_properties_css_in_js,
    ruleName: "declaration-block-no-duplicate-custom-properties",
    config: [true],
    customSyntax: "naive-css-in-js",
    skip: true,
    accept: [
        {
            code: "css` --custom-property: pink; `;",
        },
    ],
}

test_rule! {
    name: declaration_block_no_duplicate_custom_properties_ignore_properties,
    ruleName: "declaration-block-no-duplicate-custom-properties",
    config: [true, { "ignoreProperties": ["--custom-property", "/ignored/"] }],
    accept: [
        {
            code: "a { --custom-property: 1; --custom-property: 1; }",
        },
        {
            code: "a { --custom-ignored-property: 1; --custom-ignored-property: 1; }",
        },
    ],
    reject: [
        {
            code: "a { --another-custom-property: 1; --another-custom-property: 1; }",
            message: rejected("--another-custom-property"),
        },
    ],
}
