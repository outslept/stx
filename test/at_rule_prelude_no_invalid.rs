macro_rules! rejected {
    ($at_rule:expr, $prelude:expr) => {
        format!(
            "Invalid prelude \"{}\" for at-rule \"{}\"",
            $prelude, $at_rule
        )
    };
}

test_rule! {
    name: default_options,
    ruleName: "at-rule-prelude-no-invalid",
    config: [true],
    accept: [
        {
            code: "@layer;",
        },
        {
            code: "@layer foo;",
        },
        {
            code: "@media (min-width: 100px) {}",
        },
        {
            code: "@counter-style foo {}",
        },
        {
            code: "@property --foo {}",
        },
        {
            code: "@property --foo, --bar {}",
        },
        {
            code: "@foo bar;",
            description: "Custom at-rule with an any prelude",
        },
    ],
    reject: [
        {
            code: "@layer \"foo\";",
            message: rejected!("@layer", "\"foo\""),
            line: 1,
            column: 8,
            endLine: 1,
            endColumn: 13,
            description: "Invalid prelude for @layer (string instead of ident)",
        },
        {
            code: "@property  foo {}",
            message: rejected!("@property", "foo"),
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 15,
            description: "Invalid prelude for @property (missing leading --)",
        },
        {
            code: "@scope .foo {}",
            message: rejected!("@scope", ".foo"),
            line: 1,
            column: 8,
            endLine: 1,
            endColumn: 12,
            description: "Invalid prelude for @scope (missing parentheses)",
        },
        {
            code: "@font-palette-values foo {}",
            message: rejected!("@font-palette-values", "foo"),
            line: 1,
            column: 22,
            endLine: 1,
            endColumn: 25,
            description: "Invalid prelude for @font-palette-values (missing leading --)",
        },
        {
            code: "@font-feature-values #foo {}",
            message: rejected!("@font-feature-values", "#foo"),
            line: 1,
            column: 22,
            endLine: 1,
            endColumn: 26,
        },
    ],
}

test_rule! {
    name: ignore_at_rules,
    ruleName: "at-rule-prelude-no-invalid",
    config: [true, { "ignoreAtRules": ["property", "/^font-/"] }],
    accept: [
        {
            code: "@property foo;",
        },
        {
            code: "@font-palette-values foo {}",
        },
        {
            code: "@font-feature-values #foo {}",
        },
    ],
}
