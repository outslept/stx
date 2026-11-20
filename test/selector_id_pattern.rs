const RULE_NAME: &str = "selector-id-pattern";

fn expected(selector: &str, pattern: &str) -> String {
    format!("Expected \"{selector}\" to match pattern \"{pattern}\"")
}

test_rule! {
    name: regex_config,
    ruleName: RULE_NAME,
    config: ["/^[A-Z]+$/"],
    accept: [
        {
            code: "a {}",
        },
        {
            code: ".foo {}",
        },
        {
            code: "[foo='bar'] {}",
        },
        {
            code: "#FOO {}",
        },
        {
            code: "a .foo > [foo='bar'], #FOO {}",
        },
        {
            code: "a /* #foo */ {}",
        },
        {
            code: ":root { --foo: 1px; }",
            description: "custom property in root",
        },
        {
            code: "html { --foo: 1px; }",
            description: "custom property in selector",
        },
        {
            code: ":root { --custom-property-set: {} }",
            description: "custom property set in root",
        },
        {
            code: "html { --custom-property-set: {} }",
            description: "custom property set in selector",
        },
        {
            code: "/* stylelint-disable-next-line selector-id-pattern */\na #foo, /* stylelint-disable-next-line selector-id-pattern */\na #bar,\n/* stylelint-disable-next-line selector-id-pattern */\na #foo\n{}",
        },
    ],
    reject: [
        {
            code: "a #foo {}",
            message: expected("#foo", "/^[A-Z]+$/"),
            line: 1,
            column: 3,
            endLine: 1,
            endColumn: 7,
        },
        {
            code: "#ABABA > #bar {}",
            message: expected("#bar", "/^[A-Z]+$/"),
            line: 1,
            column: 10,
            endLine: 1,
            endColumn: 14,
        },
        {
            code: "/* a comment */\na #foo, /* a comment */\na #bar,\n/* a comment */\na #foo\n{}",
            warnings: [
                {
                    line: 2,
                    column: 3,
                    endLine: 2,
                    endColumn: 7,
                    message: expected("#foo", "/^[A-Z]+$/"),
                },
                {
                    line: 3,
                    column: 3,
                    endLine: 3,
                    endColumn: 7,
                    message: expected("#bar", "/^[A-Z]+$/"),
                },
                {
                    line: 5,
                    column: 3,
                    endLine: 5,
                    endColumn: 7,
                    message: expected("#foo", "/^[A-Z]+$/"),
                },
            ],
        },
    ],
}

test_rule! {
    name: string_config,
    ruleName: RULE_NAME,
    config: ["^[A-Z]+$"],
    accept: [
        {
            code: "a {}",
        },
        {
            code: ".foo {}",
        },
        {
            code: "[foo='bar'] {}",
        },
        {
            code: "#FOO {}",
        },
        {
            code: "a .foo > [foo='bar'], #FOO {}",
        },
        {
            code: "a /* #foo */ {}",
        },
        {
            code: ":root { --foo: 1px; }",
            description: "custom property in root",
        },
        {
            code: "html { --foo: 1px; }",
            description: "custom property in selector",
        },
        {
            code: ":root { --custom-property-set: {} }",
            description: "custom property set in root",
        },
        {
            code: "html { --custom-property-set: {} }",
            description: "custom property set in selector",
        },
        {
            code: "/* stylelint-disable-next-line selector-id-pattern */\na #foo, /* stylelint-disable-next-line selector-id-pattern */\na #bar,\n/* stylelint-disable-next-line selector-id-pattern */\na #foo\n{}",
        },
    ],
    reject: [
        {
            code: "a #foo {}",
            message: expected("#foo", "^[A-Z]+$"),
            line: 1,
            column: 3,
        },
        {
            code: "#ABABA > #bar {}",
            message: expected("#bar", "^[A-Z]+$"),
            line: 1,
            column: 10,
        },
    ],
}

test_rule! {
    name: scss_syntax,
    ruleName: RULE_NAME,
    config: ["/^[A-Z]+$/"],
    customSyntax: "postcss-scss",
    accept: [
        {
            code: "@for $n from 1 through 5 { #a#{$n} {} }",
            description: "ignore sass interpolation inside @for",
        },
        {
            code: ".#{$a} {}",
            description: "ignore sass var interpolation",
        },
        {
            code: ".foo { .#{$a} {} }",
            description: "ignore sass var nested interpolation",
        },
        {
            code: "a { #ABABA {} }",
        },
    ],
    reject: [
        {
            code: "a { #bar {} }",
            message: expected("#bar", "/^[A-Z]+$/"),
            line: 1,
            column: 5,
        },
    ],
}
