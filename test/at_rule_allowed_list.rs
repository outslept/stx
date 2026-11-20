const RULE_NAME: &str = "at-rule-allowed-list";

fn rejected(at_rule: &str) -> String {
    format!("Disallowed at-rule \"{at_rule}\"")
}

test_rule! {
    name: array_config,
    ruleName: RULE_NAME,
    config: ["extend", "import", "/^(-webkit-|-moz-)?keyframes$/i"],
    accept: [
        {
            code: "a { color: pink; }",
            description: "Some random code.",
        },
        {
            code: "a { @extend %placeholder; }",
            description: "@rule from an allowed list, is a Sass directive.",
        },
        {
            code: "\n      a {\n        @extend\n        %placeholder;\n      }\n    ",
            description: "@rule from an allowed list; newline after its name.",
        },
        {
            code: "\n      @keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            description: "@rule from an allowed list; independent rule.",
        },
        {
            code: "\n      @Keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            description: "@rule from an allowed list; independent rule; messed case.",
        },
        {
            code: "\n      @-moz-keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            description: "@rule from an allowed list; independent rule; has vendor prefix.",
        },
        {
            code: "\n\t@-WEBKIT-KEYFRAMES name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            description: "@rule from an allowed list; independent rule; has vendor prefix.",
        },
    ],
    reject: [
        {
            code: "\n      @mixin name () {}\n    ",
            line: 2,
            column: 7,
            endLine: 2,
            endColumn: 13,
            message: rejected("@mixin"),
            description: "@rule not from an allowed list; independent rule.",
        },
    ],
}

test_rule! {
    name: string_regex_config,
    ruleName: RULE_NAME,
    config: "/^(-webkit-|-moz-)?keyframes$/i",
    accept: [
        {
            code: "\n      @keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            description: "@rule from an allowed list; independent rule.",
        },
        {
            code: "\n      @Keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            description: "@rule from an allowed list; independent rule; messed case.",
        },
        {
            code: "\n      @-moz-keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            description: "@rule from an allowed list; independent rule; has vendor prefix.",
        },
        {
            code: "\n\t@-WEBKIT-KEYFRAMES name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            description: "@rule from an allowed list; independent rule; has vendor prefix.",
        },
    ],
    reject: [
        {
            code: "\n      @mixin name ($p) {}\n    ",
            message: rejected("@mixin"),
            line: 2,
            column: 7,
            endLine: 2,
            endColumn: 13,
            description: "@rule not from an allowed list.",
        },
        {
            code: "@import 'path/to/file.css';",
            message: rejected("@import"),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 8,
            description: "@rule not from an allowed list.",
        },
        {
            code: "@media screen and (max-witdh: 1000px) {}",
            message: rejected("@media"),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 7,
            description: "@rule not from an allowed list.",
        },
    ],
}

test_rule! {
    name: less_syntax,
    ruleName: RULE_NAME,
    config: ["keyframes"],
    customSyntax: "postcss-less",
    accept: [
        {
            code: "\n        .mixin() { margin: 0; }\n        span { .mixin(); }\n      ",
            description: "ignore Less mixin which are treated as at-rule",
        },
    ],
}
