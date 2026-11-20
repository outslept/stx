const RULE_NAME: &str = "declaration-no-important";
const REJECTED: &str = "Disallowed !important";

test_rule! {
    name: default_config,
    ruleName: RULE_NAME,
    config: [true],
    accept: [
        {
            code: "a { color: pink; }",
            description: "without !important",
        },
    ],
    reject: [
        {
            code: "a { color: pink !important; }",
            description: "with !important",
            message: REJECTED,
            line: 1,
            column: 17,
            endLine: 1,
            endColumn: 27,
        },
        {
            code: "a { color: pink ! important; }",
            description: "with ! important",
            message: REJECTED,
            line: 1,
            column: 17,
            endLine: 1,
            endColumn: 28,
        },
        {
            code: "a { color: pink!important; }",
            description: "with value!important",
            message: REJECTED,
            line: 1,
            column: 16,
            endLine: 1,
            endColumn: 26,
        },
    ],
}
