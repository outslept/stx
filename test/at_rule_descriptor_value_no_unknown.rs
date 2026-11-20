use crate::test_rule;

fn rejected(descriptor: &str, value: &str) -> String {
    format!("Unknown value \"{value}\" for descriptor \"{descriptor}\"")
}

test_rule! {
    ruleName: "at-rule-descriptor-value-no-unknown",
    config: true,
    accept: [
        {
            code: "a { color: 0; }",
            description: "ignore normal values",
        },
        {
            code: "@counter-style foo { system: cyclic; symbols: url(bar.svg); suffix: \" \"; }",
        },
        {
            code: "@property --foo { syntax: \"<integer>\"; inherits: false; initial-value: 0; }",
        },
        {
            code: "@view-transition { navigation: auto; }",
        },
        {
            code: "@foo { bar: 0 }",
        },
        {
            code: "@media (width > 400px) { a { color: red; } }",
        },
        {
            code: "a { @media (width > 400px) { color: red; } }",
        },
        {
            code: "@property --foo { system: bar; }",
            description: "a descriptor (with an unknown value) that is unknown to the at-rule",
        },
        {
            code: "@page { margin: var(--foo); }",
            description: "property within @page",
        },
        {
            code: "@function --foo() { result: var(--bar); }",
            description: "result descriptor within @function",
        },
        {
            code: "@font-face { font-family: \"var(--foo)\"; }",
            description: "var() within a string",
        },
    ],
    reject: [
        {
            code: "@counter-style foo { system: bar; }",
            message: rejected("system", "bar"),
            line: 1,
            column: 30,
            endLine: 1,
            endColumn: 33,
        },
        {
            code: "@property --foo { syntax: bar; }",
            message: rejected("syntax", "bar"),
            line: 1,
            column: 27,
            endLine: 1,
            endColumn: 30,
        },
        {
            code: "@property --foo { syntax: bar; inherits: baz; }",
            warnings: [
                {
                    message: rejected("syntax", "bar"),
                    line: 1,
                    column: 27,
                    endLine: 1,
                    endColumn: 30,
                },
                {
                    message: rejected("inherits", "baz"),
                    line: 1,
                    column: 42,
                    endLine: 1,
                    endColumn: 45,
                },
            ],
            description: "multiple invalid value in @property",
        },
        {
            code: "@property --foo { syntax: cyclic; }",
            message: rejected("syntax", "cyclic"),
            line: 1,
            column: 27,
            endLine: 1,
            endColumn: 33,
            description: "a value known to a descriptor of another at-rule but not @property",
        },
        {
            code: "@font-face { font-weight: var(--foo); }",
            message: rejected("font-weight", "var(--foo)"),
            line: 1,
            column: 27,
            endLine: 1,
            endColumn: 37,
        },
        {
            code: "@page { size: var(--foo); }",
            message: rejected("size", "var(--foo)"),
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 25,
            description: "descriptor within @page",
        },
        {
            code: "@font-face { @supports (foo: 0) { ascent-override: baz } }",
            message: rejected("ascent-override", "baz"),
            line: 1,
            column: 52,
            endLine: 1,
            endColumn: 55,
            description: "an unknown value in a nested at-rule",
        },
        {
            code: "@mixin --qux-baz(@contents) {\n\t@supports (overflow-anchor: auto) {\n\t\t@contents;\n\t}\n}\n\nbody {\n\t@apply --qux-baz {\n\t\t@font-face {\n\t\t  ascent-override: foo;\n\t\t}\n\t}\n}",
            message: rejected("ascent-override", "foo"),
            line: 10,
            column: 22,
            endLine: 10,
            endColumn: 25,
            description: "without indirection",
        },
        {
            code: "@mixin --qux-baz(@contents) {\n\t@supports (overflow-anchor: auto) {\n\t\t@font-face {\n\t\t\t@contents;\n\t\t}\n\t}\n}\n\nbody {\n\t@apply --qux-baz {\n\t\tascent-override: foo;\n\t}\n}",
            message: rejected("ascent-override", "foo"),
            line: 11,
            column: 20,
            endLine: 11,
            endColumn: 23,
            description: "with indirection",
            skip: true,
        },
    ],
}
