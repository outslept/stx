fn expected(property: &str, pattern: &str) -> String {
    format!("Expected \"{property}\" to match pattern \"{pattern}\"")
}

test_rule! {
    name: custom_property_pattern_regex_foo,
    ruleName: "custom-property-pattern",
    config: ["/foo-.+/"],
    accept: [
        {
            code: ":root { --foo-bar: 0; }",
        },
        {
            code: ":root { --boo-foo-bar: 0; }",
        },
        {
            code: ":root { --foo-color: #f00; } a { color: var(--foo-color); }",
        },
        {
            code: ":root { --foo-sub-color: #f00; } a { color: var(--foo-color, var(--foo-sub-color)); }",
        },
        {
            code: "a { --foo-color: var(); }",
            description: "empty function",
        },
    ],
    reject: [
        {
            code: ":root { --boo-bar: 0; }",
            message: expected("--boo-bar", "/foo-.+/"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: ":root { --foo-: 0; }",
            message: expected("--foo-", "/foo-.+/"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 15,
        },
        {
            code: ":root { --foo-color: #f00; } a { color: var(--boo-color); }",
            message: expected("--boo-color", "/foo-.+/"),
            line: 1,
            column: 45,
            endLine: 1,
            endColumn: 56,
        },
        {
            code: ":root { --foo-sub-color: #f00; } a { color: var(--foo-color, var(--boo-sub-color)); }",
            message: expected("--boo-sub-color", "/foo-.+/"),
            line: 1,
            column: 66,
            endLine: 1,
            endColumn: 81,
        },
        {
            code: ":root { --foo-color: #f00; } a { color: VAR(--boo-color)); }",
            message: expected("--boo-color", "/foo-.+/"),
            line: 1,
            column: 45,
            endLine: 1,
            endColumn: 56,
        },
        {
            code: ":root { --foo-color: #f00; } a { color: var(--boo-color), var(--boo-sub-color)); }",
            warnings: [
                {
                    message: expected("--boo-color", "/foo-.+/"),
                    line: 1,
                    column: 45,
                    endLine: 1,
                    endColumn: 56,
                },
                {
                    message: expected("--boo-sub-color", "/foo-.+/"),
                    line: 1,
                    column: 63,
                    endLine: 1,
                    endColumn: 78,
                },
            ],
        },
    ],
}

test_rule! {
    name: custom_property_pattern_string_foo,
    ruleName: "custom-property-pattern",
    config: ["foo-.+"],
    accept: [
        {
            code: ":root { --foo-bar: 0; }",
        },
        {
            code: ":root { --boo-foo-bar: 0; }",
        },
    ],
    reject: [
        {
            code: ":root { --boo-bar: 0; }",
            message: expected("--boo-bar", "foo-.+"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: ":root { --foo-: 0; }",
            message: expected("--foo-", "foo-.+"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 15,
        },
    ],
}

test_rule! {
    name: custom_property_pattern_case_sensitive,
    ruleName: "custom-property-pattern",
    config: ["/^[A-Z][a-z]+-[a-z][a-zA-Z]+$/"],
    accept: [
        {
            code: ":root { --Foo-bar: 0; }",
        },
        {
            code: ":root { --Foo-barBaz: 0; }",
        },
    ],
    reject: [
        {
            code: ":root { --boo-Foo-bar: 0; }",
            message: expected("--boo-Foo-bar", "/^[A-Z][a-z]+-[a-z][a-zA-Z]+$/"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 22,
        },
        {
            code: ":root { --foo-bar: 0; }",
            message: expected("--foo-bar", "/^[A-Z][a-z]+-[a-z][a-zA-Z]+$/"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 18,
        },
        {
            code: ":root { --Foo-Bar: 0; }",
            message: expected("--Foo-Bar", "/^[A-Z][a-z]+-[a-z][a-zA-Z]+$/"),
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 18,
        },
    ],
}

test_rule! {
    name: custom_property_pattern_scss,
    ruleName: "custom-property-pattern",
    config: ["/foo-.+/"],
    customSyntax: "postcss-scss",
    accept: [
        {
            code: "a { color: var($foo-color); }",
        },
        {
            code: "a { color: var(bar.$baz); }",
        },
        {
            code: "a { --#{$bar}: 0; }",
        },
        {
            code: "a { color: var( #{$bar} ); }",
        },
        {
            code: "a { color: var( --#{$bar} ); }",
        },
    ],
}

test_rule! {
    name: custom_property_pattern_at_property,
    ruleName: "custom-property-pattern",
    config: ["/^[a-z]+(?:-[a-z]+)*$/"],
    accept: [
        {
            code: "@property --foo-bar {}",
            description: "Valid @property rule with lowercase name",
        },
        {
            code: "@property --foo, --bar {}",
            description: "Valid @property rule with a list of names",
        },
    ],
    reject: [
        {
            code: "@property --Foo-bar {}",
            message: expected("--Foo-bar", "/^[a-z]+(?:-[a-z]+)*$/"),
            line: 1,
            column: 11,
            endLine: 1,
            endColumn: 20,
            description: "Invalid @property rule: uppercase letter present",
        },
        {
            code: "@property --foo- {}",
            message: expected("--foo-", "/^[a-z]+(?:-[a-z]+)*$/"),
            line: 1,
            column: 11,
            endLine: 1,
            endColumn: 17,
            description: "Invalid @property rule: incomplete property name",
        },
        {
            code: "@property --foo, --Bar {}",
            message: expected("--Bar", "/^[a-z]+(?:-[a-z]+)*$/"),
            line: 1,
            column: 18,
            endLine: 1,
            endColumn: 23,
            description: "Invalid @property rule in a list of names",
        },
    ],
}
