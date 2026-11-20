fn expected(media_name: &str, pattern: &str) -> String {
    format!("Expected \"{media_name}\" to match pattern \"{pattern}\"")
}

test_rule! {
    name: custom_media_pattern_regex_foo,
    ruleName: "custom-media-pattern",
    config: ["/foo-.+/"],
    accept: [
        {
            code: "@keyframes foofoo {}",
        },
        {
            code: "@custom-media --foo-bar (min-width: 0);",
        },
        {
            code: "@custom-media --foo-foofoo (min-width: 0);",
        },
        {
            code: "@cUsToM-mEdIa --foo-foofoo (min-width: 0);",
        },
        {
            code: "@CUSTOM-MEDIA --foo-foofoo (min-width: 0);",
        },
    ],
    reject: [
        {
            code: "@custom-media --foa-bar (min-width: 0);",
            message: expected("--foa-bar", "/foo-.+/"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 24,
        },
        {
            code: "@cUsToM-mEdIa --foa-bar (min-width: 0);",
            message: expected("--foa-bar", "/foo-.+/"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 24,
        },
        {
            code: "@CUSTOM-MEDIA --foa-bar (min-width: 0);",
            message: expected("--foa-bar", "/foo-.+/"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 24,
        },
        {
            code: "@custom-media --foa (min-width: 0);",
            message: expected("--foa", "/foo-.+/"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 20,
        },
    ],
}

test_rule! {
    name: custom_media_pattern_string_foo,
    ruleName: "custom-media-pattern",
    config: ["foo-.+"],
    accept: [
        {
            code: "@keyframes foofoo {}",
        },
        {
            code: "@custom-media --foo-bar (min-width: 0);",
        },
        {
            code: "@custom-media --foo-foofoo (min-width: 0);",
        },
    ],
    reject: [
        {
            code: "@custom-media --foa-bar (min-width: 0);",
            message: expected("--foa-bar", "foo-.+"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 24,
        },
        {
            code: "@custom-media --foa (min-width: 0);",
            message: expected("--foa", "foo-.+"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 20,
        },
    ],
}

test_rule! {
    name: custom_media_pattern_case_sensitive,
    ruleName: "custom-media-pattern",
    config: ["/^[A-Z][a-z]+-[a-z][a-zA-Z]+$/"],
    accept: [
        {
            code: "@custom-media --Ape-ageLess",
        },
        {
            code: "@custom-media --Purr-piratePlant",
        },
    ],
    reject: [
        {
            code: "@custom-media --ape-ageLess",
            message: expected("--ape-ageLess", "/^[A-Z][a-z]+-[a-z][a-zA-Z]+$/"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 28,
        },
        {
            code: "@custom-media --Ape-AgeLess",
            message: expected("--Ape-AgeLess", "/^[A-Z][a-z]+-[a-z][a-zA-Z]+$/"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 28,
        },
    ],
}
