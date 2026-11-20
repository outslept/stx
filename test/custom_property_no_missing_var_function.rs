fn rejected(custom_property: &str) -> String {
    format!("Missing var function for \"{custom_property}\"")
}

test_rule! {
    name: custom_property_no_missing_var_function_default,
    ruleName: "custom-property-no-missing-var-function",
    config: [true],
    accept: [
        {
            code: "a { color: --foo; }",
            description: "undeclared dashed-ident",
        },
        {
            code: "a { color: var(--foo); }",
            description: "undeclared dashed-ident in var",
        },
        {
            code: "a { color: env(--foo); }",
            description: "undeclared dashed-ident in env",
        },
        {
            code: "a { color: color(--foo 0% 0% 0% 0%); }",
            description: "undeclared dashed-ident in color",
        },
        {
            code: "a { color: calc(var(--foo) + var(--bar)); }",
            description: "undeclared dashed-idents in vars in calc",
        },
        {
            code: "a { color: var(--foo, red); }",
            description: "undeclared dashed-idents in var with fallback",
        },
        {
            code: "a { color: var(--foo, var(--bar)); }",
            description: "nested var() with fallback using declared custom properties",
        },
        {
            code: "a { --foo: var(--bar); }",
            description: "undeclared dashed-idents in vars assigned to custom property",
        },
        {
            code: ":root { --foo: red; } a { color: var(--foo); }",
            description: "declared custom property in var",
        },
        {
            code: "@property --foo {} a { color: var(--foo); }",
            description: "declared via at-property custom property in var",
        },
        {
            code: ":--foo {}",
            description: "custom selector",
        },
        {
            code: "@media(--foo) {}",
            description: "custom media query",
        },
        {
            code: "@property --foo {} a { transition: --foo; }",
            description: "ignore a property that can receive a custom-ident",
        },
        {
            code: "@property --foo {} a { position: running(--foo); }",
            description: "ignore a function that can receive a custom-ident",
        },
        {
            code: "@property --foo {} a { view-transition-name: --foo; }",
        },
        {
            code: "@property --qux {} a { container-name: --qux; }",
        },
        {
            code: "@property --foo {} a { scroll-timeline-name: --foo; }",
        },
        {
            code: "@property --foo {} a { SCROLL-TIMELINE-NAME: --foo; }",
        },
        {
            code: "a { color: calc(var(--foo) + 100px); }",
            description: "valid var() usage in calc with additional value",
        },
        {
            code: "a { color: var(--foo, calc(var(--bar) + 20px)); }",
            description: "nested var() with fallback using calc",
        },
        {
            code: "a { color: calc(100px + var(--foo)); }",
            description: "valid var() usage in calc with leading value",
        },
        {
            code: "a { color: var(--foo, var(--bar, var(--foobar, #000))); }",
            description: "nested fallbacks in var() with recursive fallback values",
        },
        {
            code: "button { --state: normal; background-color: if(style(--state: hover): gray; else: white;); }",
            description: "custom property in style() query within if() function",
        },
        {
            code: "@property --state {} button { background-color: if(style(--state: hover): gray; else: white;); }",
            description: "declared custom property in style() query within if() function",
        },
        {
            code: ":root { --theme: dark; } a { color: if(style(--theme: light): black; else: white;); }",
            description: "declared custom property in style() query condition",
        },
        {
            code: "a { --foo: 1; --bar: 2; color: if(style(--foo: 1): if(style(--bar: 2): red; else: blue;); else: green;); }",
            description: "nested if() with multiple style() queries",
        },
        {
            code: "a { --foo: 1; --bar: 2; color: if(style(--foo: var(--bar)): red; else: green;); }",
            description: "nested if() with multiple style() queries",
        },
        {
            code: "a { --foo: 1; --color: red; color: if(style(--foo: 1): var(--color); else: green;); }",
            description: "nested if() with multiple style() queries",
        },
        {
            code: "a { animation-timeline: --foo; }",
            description: "specifies the timeline used to control the progress of a CSS animation",
        },
        {
            code: "a { timeline-scope: --foo; }",
            description: "modifies the scope of a named animation timeline",
        },
        {
            code: "@property --foo {} a { position-try: most-width --foo; }",
            description: "refers to a position option by name in the shorthand",
        },
        {
            code: "@property --foo {} a { top: anchor(--foo bottom); }",
            description: "refers to an anchor element by name in anchor()",
        },
    ],
    reject: [
        {
            code: "a { --foo: red; color: --foo; }",
            message: rejected("--foo"),
            line: 1,
            column: 24,
            endLine: 1,
            endColumn: 29,
            description: "declared custom property",
        },
        {
            code: "@property --foo {} a { color: --foo; }",
            message: rejected("--foo"),
            line: 1,
            column: 31,
            endLine: 1,
            endColumn: 36,
            description: "declared via at-property custom property",
        },
        {
            code: "@property --foo, --bar {} a { color: --bar; }",
            message: rejected("--bar"),
            line: 1,
            column: 38,
            endLine: 1,
            endColumn: 43,
            description: "declared via at-property with a list of names",
        },
        {
            code: ":root { --bar: 0; } a { --foo: --bar; }",
            message: rejected("--bar"),
            line: 1,
            column: 32,
            endLine: 1,
            endColumn: 37,
            description: "declared in :root custom property",
        },
        {
            code: ":root { --bar: 0px; } a { color: calc(var(--foo) + --bar); }",
            message: rejected("--bar"),
            line: 1,
            column: 52,
            endLine: 1,
            endColumn: 57,
            description: "declared custom property and used inside calc",
        },
        {
            code: ":root { --foo: pink; }  a { color: --foo, red; }",
            message: rejected("--foo"),
            line: 1,
            column: 36,
            endLine: 1,
            endColumn: 41,
            description: "declared custom property and used with fallback",
        },
        {
            code: ":root { --bar: 0; } a { color: --foo(--bar); }",
            message: rejected("--bar"),
            line: 1,
            column: 38,
            endLine: 1,
            endColumn: 43,
            description: "declared custom property used inside custom function",
        },
        {
            code: ":root {\n\t--bar: 0;\n\t--baz: 0;\n}\na {\n\t--foo: --bar;\n\tcolor: --baz;\n}",
            warnings: [
                { message: rejected("--bar"), line: 6, column: 9, endLine: 6, endColumn: 14 },
                { message: rejected("--baz"), line: 7, column: 9, endLine: 7, endColumn: 14 },
            ],
            description: "two declared custom properties",
        },
        {
            code: "@property --bar {}\n@property --baz {}\na {\n\t--foo: --bar;\n\tcolor: --baz;\n}",
            warnings: [
                { message: rejected("--bar"), line: 4, column: 9, endLine: 4, endColumn: 14 },
                { message: rejected("--baz"), line: 5, column: 9, endLine: 5, endColumn: 14 },
            ],
            description: "two declared via at-property custom properties",
        },
        {
            code: "a {\n\t--foo: red;\n\t--bar: blue;\n\tcolor: var(--foo, --bar);\n}",
            message: rejected("--bar"),
            line: 4,
            column: 20,
            endLine: 4,
            endColumn: 25,
            description: "var() with missing var() syntax on fallback",
        },
        {
            code: "a {\n\t--foo: qux;\n\t--bar: baz;\n\tposition: running(var(--foo, --bar));\n}",
            message: rejected("--bar"),
            line: 4,
            column: 31,
            endLine: 4,
            endColumn: 36,
            description: "nested var() with missing var() syntax on fallback",
        },
        {
            code: "a {\n\t--foo: red;\n\t--bar: blue;\n\t--baz: green;\n\tcolor: var(--foo, var(--bar, --baz));\n}",
            warnings: [
                { message: rejected("--baz"), line: 5, column: 31, endLine: 5, endColumn: 36 },
            ],
            description: "nested var() with second-level fallback missing var() syntax",
        },
        {
            code: "a { --foo: 1; --bar: 2; color: if(style(--foo: --bar): red; else: green;); }",
            message: rejected("--bar"),
            line: 1,
            column: 48,
            endLine: 1,
            endColumn: 53,
            description: "custom property in style() query value missing var() syntax",
        },
        {
            code: "a { --foo: 1; --color: red; color: if(style(--foo: 1): --color; else: green;); }",
            message: rejected("--color"),
            line: 1,
            column: 56,
            endLine: 1,
            endColumn: 63,
            description: "custom property in if() consequent branch missing var() syntax",
        },
        {
            code: "a { --foo: 10px; --bar: 0; top: anchor(--bar bottom, --foo); }",
            message: rejected("--foo"),
            line: 1,
            column: 54,
            endLine: 1,
            endColumn: 59,
            description: "custom property in the anchor() fallback missing var() syntax",
        },
        {
            code: "a { --foo: 10px; --bar: 0; width: anchor-size(--bar width, --foo); }",
            message: rejected("--foo"),
            line: 1,
            column: 60,
            endLine: 1,
            endColumn: 65,
            description: "custom property in the anchor-size() fallback missing var() syntax",
        },
        {
            code: "a { --foo: 10px; --bar: 0; top: anchor(--bar calc(50% + --foo)); }",
            message: rejected("--foo"),
            line: 1,
            column: 57,
            endLine: 1,
            endColumn: 62,
            description: "custom property in the <anchor-side> of anchor() missing var() syntax, declared anchor name ignored",
        },
    ],
}

test_rule! {
    name: custom_property_no_missing_var_function_language_options,
    ruleName: "custom-property-no-missing-var-function",
    config: [true],
    skip: true,
    accept: [
        {
            code: ":root { --foo: red; } a { color: --foo; }",
        },
    ],
}
