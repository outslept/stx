const RULE_NAME: &str = "no-irregular-whitespace";
const REJECTED: &str = "Irregular whitespace";

test_rule! {
    name: default_config,
    ruleName: RULE_NAME,
    config: [true],
    accept: [
        {
            code: " ",
            description: "regular whitespace",
        },
        {
            code: "\n",
            description: "new line",
        },
        {
            code: "/* Irregular\u{00A0}whitespace\nin\u{00A0}multi line comments\nare\u{00A0}allowed */",
            description: "irregular whitespace in multi line comments",
        },
        {
            code: "/* stylelint-disable-next-line no-irregular-whitespace */\n\u{00A0}b, /* stylelint-disable-next-line no-irregular-whitespace */\na\u{00A0}b\n/* stylelint-disable-next-line no-irregular-whitespace */\n\u{00A0}\n{}",
        },
        {
            code: "/* stylelint-disable-next-line no-irregular-whitespace */\n\u{00A0}@media\n/* stylelint-disable-next-line no-irregular-whitespace */\n\u{00A0} /* stylelint-disable-next-line no-irregular-whitespace */\nonly\u{00A0}screen\n{}",
        },
        {
            code: ".foo {\n\t/* stylelint-disable-next-line no-irregular-whitespace */\n\tpadding\u{00A0}: /* stylelint-disable-next-line no-irregular-whitespace */\n\t\u{00A0} /* stylelint-disable-next-line no-irregular-whitespace */\n\t1px\u{00A0}2px\n\t/* stylelint-disable-next-line no-irregular-whitespace */\n\tcalc(5px\u{00A0}+ 6px);\n}",
        },
        {
            code: "a { content: '\u{00A0}'; }",
            description: "non-breaking space as content value inside declaration",
        },
        {
            code: "a { content: \"foo \t \n  bar\"; }",
            description: "non-breaking space inside a quoted string value in declaration",
        },
        {
            code: "@charset \"\u{00A0}\"",
            description: "non-breaking space within at-rule",
        },
        {
            code: "a[id=\"\u{00A0}\"] {}",
            description: "non-breaking space as attribute in selector",
        },
        {
            code: "a { animation-name: \"foo\u{00A0}\" }",
        },
        {
            code: "a { animation-name: 'ba\u{00A0}r' }",
        },
        {
            code: "a { font-family: 'Times\u{0085}New\u{00A0}\u{00A0}Roman', serif; }",
        },
        {
            code: "@font-feature-values \"foo\u{00A0}bar\"",
        },
        {
            code: "a[title=\"irregular\u{000B}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u000B",
        },
        {
            code: "a[title=\"irregular\u{000C}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u000C",
        },
        {
            code: "a[title=\"irregular\u{00A0}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u00A0",
        },
        {
            code: "a[title=\"irregular\u{0085}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u0085",
        },
        {
            code: "a[title=\"irregular\u{1680}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u1680",
        },
        {
            code: "a[title=\"irregular\u{180E}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u180E",
        },
        {
            code: "a[title=\"irregular\u{FEFF}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\uFEFF",
        },
        {
            code: "a[title=\"irregular\u{2000}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2000",
        },
        {
            code: "a[title=\"irregular\u{2001}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2001",
        },
        {
            code: "a[title=\"irregular\u{2002}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2002",
        },
        {
            code: "a[title=\"irregular\u{2003}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2003",
        },
        {
            code: "a[title=\"irregular\u{2004}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2004",
        },
        {
            code: "a[title=\"irregular\u{2005}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2005",
        },
        {
            code: "a[title=\"irregular\u{2006}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2006",
        },
        {
            code: "a[title=\"irregular\u{2007}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2007",
        },
        {
            code: "a[title=\"irregular\u{2008}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2008",
        },
        {
            code: "a[title=\"irregular\u{2009}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2009",
        },
        {
            code: "a[title=\"irregular\u{200A}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u200A",
        },
        {
            code: "a[title=\"irregular\u{200B}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u200B",
        },
        {
            code: "a[title=\"irregular\u{2028}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2028",
        },
        {
            code: "a[title=\"irregular\u{2029}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u2029",
        },
        {
            code: "a[title=\"irregular\u{202F}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u202F",
        },
        {
            code: "a[title=\"irregular\u{205F}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u205F",
        },
        {
            code: "a[title=\"irregular\u{3000}whitespace\"] { color: pink; }",
            description: "irregular whitespace in attribute selector: \\u3000",
        },
        {
            code: ":lang(\"en\u{00A0}us \")",
        },
        {
            code: ":not([data-id=\"a\u{00A0}b\"])",
        },
        {
            code: "a[data-x='foo \u{00A0}  bar'] {}",
        },
        {
            code: ":is(.class[attr=\"\u{00A0}a\"])",
        },
        {
            code: ":is(input[type=\"checkbox\u{00A0}\"])",
        },
        {
            code: "a[class='\u{00A0}foo'] {}",
        },
        {
            code: "a[id='f\u{00A0}oo'] {}",
        },
        {
            code: "[data-y=\"foo\tbar\"] {}",
        },
        {
            code: "input[placeholder=\"foo\u{2009}bar\"] {}",
        },
        {
            code: ":nth-child(\" 2n+ 1\u{00A0}\")",
        },
        {
            code: "a/*foo\u{00A0}bar*/[attr=\"test\"] {}",
        },
        {
            code: "a { /* comment\u{00A0}*/ color: red; }",
        },
        {
            code: "a { grid-template-areas: /* \"comment\u{00A0}\" */ none /* \"\u{00A0}comment \" */; }",
        },
        {
            code: "a { grid-template-areas: /* comment\u{00A0} */ none /* \u{00A0}comment */; }",
        },
        {
            code: "a { cursor: url(\"pointer.cur\u{00A0}\"), pointer; }",
        },
        {
            code: "@counter-style custom-list { symbols: \"-\u{00A0}\" \"◦\" \"‣\"; }",
        },
    ],
    reject: [
        {
            code: "@\u{00A0}charset \"utf-8\"",
            description: "irregular whitespace in at-charset css rule",
            message: REJECTED,
            line: 1,
            column: 2,
            endLine: 1,
            endColumn: 3,
        },
        {
            code: "@charset\u{00A0} \"utf-8\"",
            description: "irregular whitespace after at-charset css rule",
            message: REJECTED,
            line: 1,
            column: 9,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: ".firstClass\u{00A0}.secondClass { color: pink; }",
            description: "irregular whitespace in selector",
            message: REJECTED,
            line: 1,
            column: 12,
            endLine: 1,
            endColumn: 13,
        },
        {
            code: ".firstClass .secondClass \u{00A0}{ color: pink; }",
            description: "irregular whitespace after selector",
            message: REJECTED,
            line: 1,
            column: 26,
            endLine: 1,
            endColumn: 27,
        },
        {
            code: "margin: 1rem\u{00A0}2rem;",
            description: "irregular whitespace in declaration value",
            message: REJECTED,
            line: 1,
            column: 13,
            endLine: 1,
            endColumn: 14,
        },
        {
            code: "margin: 1rem 2rem\u{00A0};",
            description: "irregular whitespace after value",
            message: REJECTED,
            line: 1,
            column: 18,
            endLine: 1,
            endColumn: 19,
        },
        {
            code: "$variable\u{00A0}: 5rem;",
            description: "irregular whitespace in variable name",
            message: REJECTED,
            line: 1,
            column: 10,
            endLine: 1,
            endColumn: 11,
        },
        {
            code: "@media screen {\u{000C}}",
            description: "irregular whitespace in the after of an at rule",
            message: REJECTED,
            line: 1,
            column: 16,
            endLine: 1,
            endColumn: 17,
        },
        {
            code: ".a {\u{000C}}",
            description: "irregular whitespace in the after of a rule",
            message: REJECTED,
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 6,
        },
        {
            code: ".a {\u{000C}\u{000C}}",
            description: "irregular whitespace in the after of a rule",
            message: REJECTED,
            line: 1,
            column: 5,
            endLine: 1,
            endColumn: 7,
        },
        {
            code: "/* a comment */\n\u{00A0}b, /* a comment */\na\u{00A0}b\n/* a comment */\n\u{00A0}\n{}",
            warnings: [
                {
                    message: REJECTED,
                    line: 2,
                    column: 1,
                    endLine: 2,
                    endColumn: 2,
                },
                {
                    message: REJECTED,
                    line: 3,
                    column: 2,
                    endLine: 3,
                    endColumn: 3,
                },
                {
                    message: REJECTED,
                    line: 5,
                    column: 1,
                    endLine: 5,
                    endColumn: 2,
                },
            ],
        },
        {
            code: "/* a comment */\n\u{00A0}@media\n/* a comment */\n\u{00A0} /* a comment */\nonly\u{00A0}screen\n{}",
            warnings: [
                {
                    message: REJECTED,
                    line: 2,
                    column: 1,
                    endLine: 2,
                    endColumn: 2,
                },
                {
                    message: REJECTED,
                    line: 4,
                    column: 1,
                    endLine: 4,
                    endColumn: 2,
                },
                {
                    message: REJECTED,
                    line: 5,
                    column: 5,
                    endLine: 5,
                    endColumn: 6,
                },
            ],
        },
        {
            code: ".foo {\n\t/* a comment */\n\tpadding\u{00A0}: /* a comment */\n\t\u{00A0} /* a comment */\n\t1px\u{00A0}2px\n\t/* a comment */\n\tcalc(5px\u{00A0}+ 6px);\n}",
            warnings: [
                {
                    message: REJECTED,
                    line: 3,
                    column: 9,
                    endLine: 3,
                    endColumn: 10,
                },
                {
                    message: REJECTED,
                    line: 4,
                    column: 2,
                    endLine: 4,
                    endColumn: 3,
                },
                {
                    message: REJECTED,
                    line: 5,
                    column: 5,
                    endLine: 5,
                    endColumn: 6,
                },
                {
                    message: REJECTED,
                    line: 7,
                    column: 10,
                    endLine: 7,
                    endColumn: 11,
                },
            ],
        },
        {
            code: "a {\n\tgrid-template:\n\t\t\"a a\u{00A0}a\" 40px\n\t\t\"b c c\" 40px\n\t\t\"b\u{00A0}c c\" 40px / 1fr 1fr 1fr;\n}",
            warnings: [
                {
                    message: REJECTED,
                    line: 3,
                    column: 7,
                    endLine: 3,
                    endColumn: 8,
                },
                {
                    message: REJECTED,
                    line: 5,
                    column: 5,
                    endLine: 5,
                    endColumn: 6,
                },
            ],
        },
        {
            code: "a { grid-template-areas: \"a\u{00A0}a\" \"b b\"; }",
            message: REJECTED,
            line: 1,
            column: 28,
            endLine: 1,
            endColumn: 29,
        },
        {
            code: "a { grid: \"foo\u{00A0}\" 1fr /* \u{00A0}comment */ \"bar\" 1fr; }",
            message: REJECTED,
            line: 1,
            column: 15,
            endLine: 1,
            endColumn: 16,
        },
    ],
}
