const RULE_NAME: &str = "selector-max-id";

fn expected(selector: &str, max: usize) -> String {
    format!("Too many ID selectors in \"{}\", maximum {}", selector, max)
}

test_rule! {
    name: sanity_checks,
    ruleName: RULE_NAME,
    config: [0],
    accept: [
        {
            code: "foo {}",
        },
        {
            code: ".bar {}",
        },
        {
            code: "foo .bar {}",
        },
        {
            code: "[foo] {}",
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
            code: "/* stylelint-disable-next-line selector-max-id */\n#foo, /* stylelint-disable-next-line selector-max-id */\n#bar,\n/* stylelint-disable-next-line selector-max-id */\n#baz\n{}",
        },
    ],
    reject: [
        {
            code: "#foo {}",
            message: expected("#foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 5,
        },
        {
            code: ".bar #foo {}",
            message: expected(".bar #foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: ".bar > #foo {}",
            message: expected(".bar > #foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 12,
        },
        {
            code: "#foo.bar {}",
            message: expected("#foo.bar", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: ".foo, .bar, #foo.baz {}",
            message: expected("#foo.baz", 0),
            line: 1,
            column: 13,
            endLine: 1,
            endColumn: 21,
        },
        {
            code: "#foo [lang^=en] {}",
            message: expected("#foo [lang^=en]", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 16,
        },
        {
            code: "#foo[lang^=en] {}",
            message: expected("#foo[lang^=en]", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 15,
        },
        {
            code: "* #foo {}",
            message: expected("* #foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 7,
        },
        {
            code: "#foo * {}",
            message: expected("#foo *", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 7,
        },
        {
            code: "*#foo {}",
            message: expected("*#foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 6,
        },
        {
            code: "#foo:hover {}",
            message: expected("#foo:hover", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 11,
        },
        {
            code: ":not(#foo) {}",
            message: expected(":not(#foo)", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 11,
        },
        {
            code: "/* a comment */\n#foo, /* a comment */\n#bar,\n/* a comment */\n#foo\n{}",
            warnings: [
                {
                    message: expected("#foo", 0),
                    line: 2,
                    column: 1,
                    endLine: 2,
                    endColumn: 5,
                },
                {
                    message: expected("#bar", 0),
                    line: 3,
                    column: 1,
                    endLine: 3,
                    endColumn: 5,
                },
                {
                    message: expected("#foo", 0),
                    line: 5,
                    column: 1,
                    endLine: 5,
                    endColumn: 5,
                },
            ],
        },
    ],
}

test_rule! {
    name: standard_tests,
    ruleName: RULE_NAME,
    config: [2],
    accept: [
        {
            code: "#foo {}",
            description: "fewer than max ID selectors",
        },
        {
            code: "#foo:hover {}",
            description: "pseudo selectors",
        },
        {
            code: "#foo #bar {}",
            description: "compound selector",
        },
        {
            code: "#foo, \n#bar {}",
            description: "multiple selectors: fewer than max ID selectors",
        },
        {
            code: "#foo #bar, \n#bar #foo {}",
            description: "multiple selectors: exactly max ID selectors",
        },
        {
            code: "#foo { #bar {} }",
            description: "nested selectors",
        },
        {
            code: "#foo { #bar > & {} }",
            description: "nested selectors: parent selector",
        },
        {
            code: "#foo, #bar { & > #foo {} }",
            description: "nested selectors: superfluous parent selector",
        },
        {
            code: "@media print { #foo #bar {} }",
            description: "media query: parent",
        },
        {
            code: "#foo { @media print { #bar {} } }",
            description: "media query: nested",
        },
        {
            code: ".foo { @media print { #bar #baz {} } }",
            description: "@media query: nested compound selector",
        },
        {
            code: "#foo { #baz { .quux {} } }",
            description: "nested compound selector",
        },
        {
            code: "#foo { .baz { #quux {} } }",
            description: "nested compound selector",
        },
    ],
    reject: [
        {
            code: "#foo #bar #baz {}",
            description: "compound selector: greater than max ID selectors",
            message: expected("#foo #bar #baz", 2),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 15,
        },
        {
            code: "#foo #bar:not(#baz) {}",
            description: ":not(): outside and inside",
            message: expected("#foo #bar:not(#baz)", 2),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 20,
        },
        {
            code: ":--foo(#foo) #bar #baz {}",
            description: "custom functional pseudo",
            message: expected(":--foo(#foo) #bar #baz", 2),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 23,
        },
        {
            code: "#foo, \n#bar #baz #foo {}",
            description: "multiple selectors: greater than max ID selectors",
            message: expected("#bar #baz #foo", 2),
            line: 2,
            column: 1,
            endLine: 2,
            endColumn: 15,
        },
        {
            code: "#foo #bar #baz:not(#quux) {}",
            description: ":not(): greater than max ID selectors, outside",
            message: expected("#foo #bar #baz:not(#quux)", 2),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 26,
        },
        {
            code: ".foo { @media print { #bar #baz #foo {} } }",
            description: "nested compound selector: greater than max ID selectors, nested @media query",
            message: expected("#bar #baz #foo", 2),
            line: 1,
            column: 23,
            endLine: 1,
            endColumn: 37,
        },
    ],
}

test_rule! {
    name: scss_max_0,
    ruleName: RULE_NAME,
    config: [0],
    customSyntax: "postcss-scss",
    accept: [
        {
            code: "@keyframes spin { #{50% - $n} {} }",
        },
        {
            code: "@for $n from 1 through 10 { .n-#{$n} { content: \"n: #{1 + 1}\"; } }",
            description: "ignore sass interpolation inside @for",
        },
        {
            code: "@for $n from 1 through 10 { .n#{$n}-#{$n} { content: \"n: #{1 + 1}\"; } }",
            description: "ignore multiple sass interpolations in a selector inside @for",
        },
        {
            code: "@for $n from 1 through 10 { .n#{$n}n#{$n} { content: \"n: #{1 + 1}\"; } }",
            description: "ignore multiple sass interpolations in a selector inside @for",
        },
        {
            code: "@each $n in $vals { .n-#{$n} { content: \"n: #{1 + 1}\"; } }",
            description: "ignore sass interpolation inside @each",
        },
        {
            code: "@while $n < 10 { .n-#{$n} { content: \"n: #{1 + 1}\"; } }",
            description: "ignore sass interpolation inside @while",
        },
        {
            code: "div:nth-child(#{map-get($foo, bar)}) {}",
            description: "ignore sass map-get interpolation",
        },
        {
            code: "@for $n from 1 through 10 { .n-#{$n} #foo {} }",
            description: "ignore sass interpolation + ID inside @for",
        },
    ],
    reject: [
        {
            code: "#foo { @include test {} }",
            message: expected("#foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 5,
        },
        {
            code: ".bar #foo { @include test {} }",
            message: expected(".bar #foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 10,
        },
        {
            code: ".bar > #foo { @include test {} }",
            message: expected(".bar > #foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 12,
        },
        {
            code: "#foo.bar { @include test {} }",
            message: expected("#foo.bar", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 9,
        },
        {
            code: ".foo, .bar, #foo.baz { @include test {} }",
            message: expected("#foo.baz", 0),
            line: 1,
            column: 13,
            endLine: 1,
            endColumn: 21,
        },
        {
            code: "#foo [lang^=en] { @include test {} }",
            message: expected("#foo [lang^=en]", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 16,
        },
        {
            code: "#foo[lang^=en] { @include test {} }",
            message: expected("#foo[lang^=en]", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 15,
        },
        {
            code: "* #foo { @include test {} }",
            message: expected("* #foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 7,
        },
        {
            code: "#foo * { @include test {} }",
            message: expected("#foo *", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 7,
        },
        {
            code: "*#foo { @include test {} }",
            message: expected("*#foo", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 6,
        },
        {
            code: "#foo:hover { @include test {} }",
            message: expected("#foo:hover", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 11,
        },
        {
            code: ":not(#foo) { @include test {} }",
            message: expected(":not(#foo)", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 11,
        },
        {
            code: "@include test { #foo {} }",
            message: expected("#foo", 0),
            line: 1,
            column: 17,
            endLine: 1,
            endColumn: 21,
        },
    ],
}

test_rule! {
    name: less_max_0,
    ruleName: RULE_NAME,
    config: [0],
    customSyntax: "postcss-less",
    accept: [
        {
            code: ".for(@n: 1) when (@n <= 10) { .n-@{n} { content: %(\"n: %d\", 1 + 1); } .for(@n + 1); }",
            description: "ignore Less interpolation inside .for",
        },
        {
            code: ".for(@n: 1) when (@n <= 10) { .n-@{n}-@{n} { content: %(\"n: %d\", 1 + 1); } .for(@n + 1); }",
            description: "ignore multiple Less interpolations in a selector inside .for",
        },
        {
            code: ".for(@n: 1) when (@n <= 10) { .n-@{n}n@{n} { content: %(\"n: %d\", 1 + 1); } .for(@n + 1); }",
            description: "ignore multiple Less interpolations in a selector inside .for",
        },
        {
            code: ".each(@vals, @n: 1) when (@n <= length(@vals)) { @val: extract(@vals, @n); .n-@{val} { content: %(\"n: %d\", 1 + 1); } .each(@vals, @n + 1); }",
            description: "ignore Less interpolation inside .each",
        },
        {
            code: ".while(@n: 0) when (@n < 10) { .n-@{n} { content: %(\"n: %d\", 1 + 1); } .while(@n + 1) }",
            description: "ignore Less interpolation inside .while",
        },
        {
            code: ".for(@n: 1) when (@n <= 10) { .n-@{n} #foo {} .for(@n + 1) }",
            description: "ignore Less interpolation + ID inside .for",
        },
    ],
}

test_rule! {
    name: scss_max_2,
    ruleName: RULE_NAME,
    config: [2],
    customSyntax: "postcss-scss",
    accept: [
        {
            code: "#foo { @if ($p == 1) { #bar {} } }",
            description: "@if statement: nested",
        },
        {
            code: ".foo { @if ($p == 1) { #bar #baz {} } }",
            description: "@if statement: nested compound selector",
        },
    ],
    reject: [
        {
            code: "#foo #bar #baz { @if ($p == 1) { #bar {} } }",
            description: "compound selector: greater than max ID selectors, nested @if statement",
            message: expected("#foo #bar #baz", 2),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 15,
        },
        {
            code: ".foo { @if ($p == 1) { #bar #baz #foo {} } }",
            description: "nested compound selector: greater than max ID selectors, nested @if statement",
            message: expected("#bar #baz #foo", 2),
            line: 1,
            column: 24,
            endLine: 1,
            endColumn: 38,
        },
    ],
}

test_rule! {
    name: ignore_context_functional_pseudo_classes,
    ruleName: RULE_NAME,
    config: [0, { "ignoreContextFunctionalPseudoClasses": [":not", "/^:(h|H)as$/"] }],
    accept: [
        {
            code: "a:not(#foo) {}",
            description: "selector within ignored pseudo-class (string input)",
        },
        {
            code: "a:has(#foo) {}",
            description: "selector within ignored pseudo-class (regex input)",
        },
        {
            code: "a:not(:is(#foo)) {}",
            description: "selector within ignored nested pseudo-class (regex input)",
        },
    ],
    reject: [
        {
            code: "a:matches(#foo) {}",
            description: "selector within non-ignored pseudo class",
            message: expected("a:matches(#foo)", 0),
            line: 1,
            column: 1,
            endLine: 1,
            endColumn: 16,
        },
    ],
}
