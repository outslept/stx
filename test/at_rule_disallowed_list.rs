use crate::test_rule;

fn rejected(at_rule: &str) -> String {
    format!("Disallowed at-rule \"{at_rule}\"")
}

test_rule! {
    name: array_config,
    ruleName: "at-rule-disallowed-list",
    config: ["extend", "supports", "/^(-webkit-|-moz-)?keyframes$/i"],
    accept: [
        {
            code: "a { color: pink; }",
            description: "Some random code.",
        },
        {
            code: "@mixin name ($p) {}",
            description: "@rule not from a disallowed list.",
        },
    ],
    reject: [
        {
            code: "a { @extend %placeholder; }",
            message: rejected("@extend"),
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 12,
            description: "@rule from a disallowed list, is a Sass directive.",
        },
        {
            code: "\n      a {\n        @extend\n        %placeholder;\n      }\n    ",
            message: rejected("@extend"),
            line: 3,
            column: 9,
            endLine: 3,
            endColumn: 16,
            description: "@rule from a disallowed list; newline after its name.",
        },
        {
            code: "\n      @keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            message: rejected("@keyframes"),
            line: 2,
            column: 7,
            endLine: 2,
            endColumn: 17,
            description: "@rule from a disallowed list; independent rule.",
        },
        {
            code: "\n      @Keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            message: rejected("@Keyframes"),
            line: 2,
            column: 7,
            description: "@rule from a disallowed list; independent rule; messed case.",
        },
        {
            code: "\n      @-moz-keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            message: rejected("@-moz-keyframes"),
            line: 2,
            column: 7,
            description: "@rule from a disallowed list; independent rule; has vendor prefix.",
        },
        {
            code: "\n      @-WEBKIT-KEYFRAMES name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            message: rejected("@-WEBKIT-KEYFRAMES"),
            line: 2,
            column: 7,
            description: "@rule from a disallowed list; independent rule; has vendor prefix.",
        },
    ],
}

test_rule! {
    name: string_regex_config,
    ruleName: "at-rule-disallowed-list",
    config: "/^(-webkit-|-moz-)?keyframes$/i",
    accept: [
        {
            code: "a { color: pink; }",
            description: "Some random code.",
        },
        {
            code: "@mixin name ($p) {}",
            description: "@rule not from a disallowed list.",
        },
    ],
    reject: [
        {
            code: "\n      @keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            message: rejected("@keyframes"),
            line: 2,
            column: 7,
            description: "@rule from a disallowed list; independent rule.",
        },
        {
            code: "\n      @Keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            message: rejected("@Keyframes"),
            line: 2,
            column: 7,
            description: "@rule from a disallowed list; independent rule; messed case.",
        },
        {
            code: "\n      @-moz-keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            message: rejected("@-moz-keyframes"),
            line: 2,
            column: 7,
            description: "@rule from a disallowed list; independent rule; has vendor prefix.",
        },
        {
            code: "\n      @-WEBKIT-KEYFRAMES name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            message: rejected("@-WEBKIT-KEYFRAMES"),
            line: 2,
            column: 7,
            description: "@rule from a disallowed list; independent rule; has vendor prefix.",
        },
    ],
}

test_rule! {
    name: less_syntax,
    ruleName: "at-rule-disallowed-list",
    config: ["keyframes"],
    customSyntax: "postcss-less",
    accept: [
        {
            code: "\n        .keyframes() { margin: 0; }\n        span { .keyframes(); }\n      ",
            description: "ignore Less mixin which are treated as at-rule",
        },
    ],
}
