const RULE_NAME: &str = "annotation-no-unknown";

fn rejected(annotation: &str) -> String {
    format!("Unknown annotation \"{annotation}\"")
}

test_rule! {
    name: ignore_annotations,
    ruleName: RULE_NAME,
    config: [
        true,
        {
            "ignoreAnnotations": ["/^--foo-/", "--bar"]
        }
    ],
    accept: [
        {
            code: "a { color: green !important; }",
        },
        {
            code: "a { color: green !IMPORTANT; }",
        },
        {
            code: "a { color: green !ImPoRtAnT; }",
        },
        {
            code: "a { color: green !--foo--bar; }",
        },
        {
            code: "a { color: green !--bar; }",
        },
    ],
    reject: [
        {
            code: "a { color: green !imprtant }",
            message: rejected("!imprtant"),
            line: 1,
            column: 18,
            endLine: 1,
            endColumn: 27,
        },
        {
            code: "a { color: green !IMPRTANT }",
            message: rejected("!IMPRTANT"),
            line: 1,
            column: 18,
            endLine: 1,
            endColumn: 27,
        },
        {
            code: "a { color: green !ImPrTaNt }",
            message: rejected("!ImPrTaNt"),
            line: 1,
            column: 18,
            endLine: 1,
            endColumn: 27,
        },
    ],
}

test_rule! {
    name: scss_syntax,
    ruleName: RULE_NAME,
    config: [true],
    customSyntax: "postcss-scss",
    accept: [
        {
            code: "a { color: #{$green} !imprtant; }",
        },
    ],
}
