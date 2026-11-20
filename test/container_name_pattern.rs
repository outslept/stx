fn expected(container_name: &str, pattern: &str) -> String {
    format!("Expected \"{container_name}\" to match pattern \"{pattern}\"")
}

test_rule! {
    name: container_name_pattern_default,
    ruleName: "container-name-pattern",
    config: ["foo-.+"],
    accept: [
        { code: "a { color: red; }" },
        { code: "a { container-name: initial; }" },
        { code: "a { container-name: REVERT; }" },
        { code: "a { container: initial / inline-size; }" },
        { code: "@container NONE {}" },
        { code: "@container foo-bar {}" },
        { code: " @CONTAINER foo-bar   {}" },
        { code: " @container foo-bar {}" },
        { code: "@container foo-bar {\n}" },
        { code: "@container foo-bar (width > 400px) {}" },
        { code: "@container foo-bar () {}" },
        { code: "@container foo-bar style(--cards small) {}" },
        { code: "@container foo-bar scroll-state(stuck: top) {}" },
        { code: "@container foo-bar (inline-size > 30em), foo-baz style(--large: true) {}" },
        { code: "@container foo-bar (inline-size > 30em) and foo-baz style(--large: true) {}" },
        {
            code: "@container () {}",
            description: "nameless container condition",
        },
        {
            code: "a { container-name: foo-bar; }",
        },
        {
            code: "a { container: foo-bar / inline-size; }",
        },
        {
            code: "a { container: foo-bar foo-baz / inline-size }",
            description: "multiple container names",
        },
        {
            code: "a { container: foo-bar foo-baz / size scroll-state }",
            description: "multiple container names and multiple container types",
        },
    ],
    reject: [
        {
            code: "@container foo {}",
            message: expected("foo", "foo-.+"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 15,
        },
        {
            code: "@container bar {}",
            message: expected("bar", "foo-.+"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 15,
        },
        {
            code: "@container FOO-bar {}",
            message: expected("FOO-bar", "foo-.+"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 19,
        },
        {
            code: "@container foo-bar (), FOO-bar () {}",
            message: expected("FOO-bar", "foo-.+"),
            line: 1,
            column: 24,
            endLine: 1,
            endColumn: 31,
        },
        {
            code: "a { container-name: FOO-bar; }",
            message: expected("FOO-bar", "foo-.+"),
            line: 1,
            column: 21,
            endLine: 1,
            endColumn: 28,
        },
        {
            code: "@container bar (inline-size > 30em), baz style(--large: true) {}",
            warnings: [
                {
                    message: expected("bar", "foo-.+"),
                    line: 1,
                    column: 12,
                    endLine: 1,
                    endColumn: 15,
                },
                {
                    message: expected("baz", "foo-.+"),
                    line: 1,
                    column: 38,
                    endLine: 1,
                    endColumn: 41,
                },
            ],
        },
        {
            code: "a { container-name: bar baz; }",
            warnings: [
                {
                    message: expected("bar", "foo-.+"),
                    line: 1,
                    column: 21,
                    endLine: 1,
                    endColumn: 24,
                },
                {
                    message: expected("baz", "foo-.+"),
                    line: 1,
                    column: 25,
                    endLine: 1,
                    endColumn: 28,
                },
            ],
        },
        {
            code: "a { container: bar baz / inline-size; }",
            warnings: [
                {
                    message: expected("bar", "foo-.+"),
                    line: 1,
                    column: 16,
                    endLine: 1,
                    endColumn: 19,
                },
                {
                    message: expected("baz", "foo-.+"),
                    line: 1,
                    column: 20,
                    endLine: 1,
                    endColumn: 23,
                },
            ],
        },
    ],
}

test_rule! {
    name: container_name_pattern_regex,
    ruleName: "container-name-pattern",
    config: ["/^foo-bar$/"],
    accept: [
        {
            code: "@container foo-bar {}",
            description: "Accepts pattern in RegExp notation",
        },
    ],
    reject: [
        {
            code: "@container foo-baz {}",
            message: expected("foo-baz", "/^foo-bar$/"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 19,
        },
    ],
}

test_rule! {
    name: container_name_pattern_language_options,
    ruleName: "container-name-pattern",
    config: ["foo-.+"],
    accept: [
        {
            code: "a { container-name: bar; }",
            skip: true,
        },
    ],
    reject: [
        {
            code: "a { container-name: foo; }",
            message: expected("foo", "foo-.+"),
            line: 1,
            column: 21,
            endLine: 1,
            endColumn: 24,
        },
    ],
}
