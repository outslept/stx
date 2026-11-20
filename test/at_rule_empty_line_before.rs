const EXPECTED: &str = "Expected empty line before at-rule";
const REJECTED: &str = "Expected no empty line before at-rule";

test_rule! {
    name: always_default,
    ruleName: "at-rule-empty-line-before",
    config: ["always"],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {} b {}", description: "rule ignored" },
        { code: "@font-face {}", description: "first node ignored" },
        { code: "a {}\n\n@media {}" },
        { code: "a {}\n\n@mEdIa {}" },
        { code: "a {}\n\n@MEDIA {}" },
        { code: "@keyframes foo {}\n\n@media {}" },
        { code: "@kEyFrAmEs foo {}\n\n@MeDia {}" },
        { code: "@KEYFRAMES foo {}\n\n@MEDIA {}" },
        { code: "@-webkit-keyframes foo {}\n\n@media {}" },
        { code: "@-webkit-keyframes foo {}\n\n@-webkit-keyframes bar {}" },
        { code: "a {}\r\n\r\n@media {}", description: "windows" },
        { code: "a {}\n\r\n@media {}", description: "mixed" },
        { code: "a {\n\n  @mixin foo;\n}" },
    ],
    reject: [
        {
            code: "a {} @media {}",
            fixed: "a {}\n\n @media {}",
            message: EXPECTED,
            fix: { range: [3, 4], text: "}\n\n" },
        },
        {
            code: "a {} @mEdIa {}",
            fixed: "a {}\n\n @mEdIa {}",
            message: EXPECTED,
            fix: { range: [3, 4], text: "}\n\n" },
        },
        {
            code: "a {} @MEDIA {}",
            fixed: "a {}\n\n @MEDIA {}",
            message: EXPECTED,
            fix: { range: [3, 4], text: "}\n\n" },
        },
        {
            code: "@keyframes foo {} @media {}",
            fixed: "@keyframes foo {}\n\n @media {}",
            message: EXPECTED,
            fix: { range: [16, 17], text: "}\n\n" },
        },
        {
            code: "@-webkit-keyframes foo {} @media {}",
            fixed: "@-webkit-keyframes foo {}\n\n @media {}",
            message: EXPECTED,
            fix: { range: [24, 25], text: "}\n\n" },
        },
        {
            code: "@-webkit-keyframes foo {} @-webkit-keyframes bar {}",
            fixed: "@-webkit-keyframes foo {}\n\n @-webkit-keyframes bar {}",
            message: EXPECTED,
            fix: { range: [24, 25], text: "}\n\n" },
        },
        {
            code: "a {}\n@media {}",
            fixed: "a {}\n\n@media {}",
            message: EXPECTED,
            fix: { range: [4, 5], text: "\n\n" },
        },
        {
            code: "a {}\r\n@media {}",
            fixed: "a {}\r\n\r\n@media {}",
            message: EXPECTED,
            fix: { range: [5, 6], text: "\n\r\n" },
        },
        {
            code: "a {}\n\n/* comment */\n@media {}",
            fixed: "a {}\n\n/* comment */\n\n@media {}",
            message: EXPECTED,
            fix: { range: [19, 20], text: "\n\n" },
        },
        {
            code: "a {}\r\n\r\n/* comment */\r\n@media {}",
            fixed: "a {}\r\n\r\n/* comment */\r\n\r\n@media {}",
            message: EXPECTED,
            fix: { range: [22, 23], text: "\n\r\n" },
        },
    ],
}

test_rule! {
    name: always_except_blockless_after_blockless_1,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "except": ["blockless-after-blockless"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {} b {}", description: "rule ignored" },
        { code: "@font-face {}", description: "first node ignored" },
        { code: "a {}\n\n@media {}" },
        { code: "a {\n\n  @mixin foo;\n}" },
        { code: "@keyframes foo {}\n\n@import 'x.css'", description: "empty line not blockless pair" },
        { code: "@import 'x.css';\n@import 'y.css'", description: "no empty line blockless pair" },
        { code: "@import 'x.css';", description: "single blockless rule" },
    ],
    reject: [
        {
            code: "@keyframes foo {}\n@import 'x.css'",
            fixed: "@keyframes foo {}\n\n@import 'x.css'",
            message: EXPECTED,
            fix: { range: [17, 18], text: "\n\n" },
        },
        {
            code: "@import 'x.css';\n\n@import 'y.css'",
            fixed: "@import 'x.css';\n@import 'y.css'",
            message: REJECTED,
            fix: { range: [17, 18], text: "" },
        },
    ],
}

test_rule! {
    name: always_except_blockless_after_blockless_2,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "except": ["blockless-after-blockless"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "@import 'y.css'; @import 'x.css';" },
        {
            code: "@charset \"UTF-8\";\n@import url(x.css); /* comment */\n@import url(y.css);",
            description: "shared-line comment accepted",
        },
    ],
    reject: [
        {
            code: "@keyframes foo {}\n@import 'x.css'",
            fixed: "@keyframes foo {}\n\n@import 'x.css'",
            message: EXPECTED,
            fix: { range: [17, 18], text: "\n\n" },
        },
        {
            code: "@import 'x.css';\n\n@import 'y.css'",
            fixed: "@import 'x.css';\n@import 'y.css'",
            message: REJECTED,
            fix: { range: [17, 18], text: "" },
        },
    ],
}

test_rule! {
    name: always_ignore_blockless_after_blockless,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "ignore": ["blockless-after-blockless"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "@charset 'UTF-8'; @import 'x.css';" },
    ],
    reject: [
        {
            code: "@import 'x.css'; @media {};",
            fixed: "@import 'x.css';\n\n @media {};",
            message: EXPECTED,
            line: 1,
            column: 18,
            fix: { range: [15, 16], text: ";\n\n" },
        },
        {
            code: "@import 'test'; @include mixin(1) { @content; };",
            fixed: "@import 'test';\n\n @include mixin(1) { @content; };",
            message: EXPECTED,
            fix: { range: [14, 15], text: ";\n\n" },
        },
    ],
}

test_rule! {
    name: always_ignore_after_comment,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "ignore": ["after-comment"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "/* foo */\n@media {}" },
        { code: "/* foo */\n\n@media{}" },
        { code: "/* foo */\r\n\r\n@media {}", description: "CRLF" },
    ],
    reject: [
        {
            code: "a {} @media {}",
            fixed: "a {}\n\n @media {}",
            message: EXPECTED,
            fix: { range: [3, 4], text: "}\n\n" },
        },
        {
            code: "bar {} /* foo */\n@media {}",
            fixed: "bar {} /* foo */\n\n@media {}",
            message: EXPECTED,
            line: 2,
            column: 1,
            description: "after shared-line comment",
            fix: { range: [16, 17], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: always_ignore_first_nested,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "ignore": ["first-nested"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "@supports {\n  @media {}\n}" },
        { code: "@supports {\n\n  @media {}\n}" },
    ],
    reject: [
        {
            code: "@supports {\n  @media {}\n  @media{}\n}",
            fixed: "@supports {\n  @media {}\n\n  @media{}\n}",
            message: EXPECTED,
            line: 3,
            column: 3,
            fix: { range: [23, 24], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: always_except_inside_block,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "except": ["inside-block"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {}\n\n@media {}" },
        { code: "a {\n  @mixin foo;\n  color: pink;\n}" },
        { code: "a {\n  color: pink;\n  @mixin foo;\n}" },
    ],
    reject: [
        {
            code: "a {\n\n  @mixin foo;\n  color: pink;\n}",
            fixed: "a {\n  @mixin foo;\n  color: pink;\n}",
            message: REJECTED,
            fix: { range: [4, 5], text: "" },
        },
        {
            code: "a {\n\n  color: pink;\n\n  @mixin foo;\n}",
            fixed: "a {\n\n  color: pink;\n  @mixin foo;\n}",
            message: REJECTED,
            fix: { range: [20, 21], text: "" },
        },
    ],
}

test_rule! {
    name: always_except_first_nested,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "except": ["first-nested"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {}\n\n@media {}" },
        { code: "a {\n  @mixin foo;\n  color: pink;\n}" },
        {
            code: "a { /* comment */\n  @mixin foo;\n  color: pink;\n}",
            description: "shared-line comment",
        },
    ],
    reject: [
        {
            code: "a {\n  color: pink;\n  @mixin foo;\n}",
            fixed: "a {\n  color: pink;\n\n  @mixin foo;\n}",
            message: EXPECTED,
            fix: { range: [18, 19], text: "\n\n" },
        },
        {
            code: "a {\n\n  @mixin foo;\n  color: pink;\n}",
            fixed: "a {\n  @mixin foo;\n  color: pink;\n}",
            message: REJECTED,
            fix: { range: [4, 5], text: "" },
        },
        {
            code: "a {/* comment */\n\n  @mixin foo;\n  color: pink;\n}",
            fixed: "a {/* comment */\n  @mixin foo;\n  color: pink;\n}",
            message: REJECTED,
            fix: { range: [17, 18], text: "" },
        },
    ],
}

test_rule! {
    name: always_ignore_inside_block,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "ignore": ["inside-block"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {}\n\n@media {}" },
        { code: "a {\n  @mixin foo;\n  color: pink;\n}" },
        { code: "a {\n  color: pink;\n  @mixin foo;\n}" },
        { code: "a {\n\n  @mixin foo;\n  color: pink;\n}" },
        { code: "a {\n\n  color: pink;\n\n  @mixin foo;\n}" },
    ],
}

test_rule! {
    name: always_except_blockless_and_inside_block,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "except": ["blockless-after-blockless", "inside-block"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {\n  @mixin foo;\n  color: pink;\n}" },
        { code: "a {\n  color: pink;\n  @mixin foo;\n}" },
        { code: "@keyframes foo {}\n\n@import 'x.css'", description: "empty line not blockless pair" },
        { code: "@import 'x.css';\n@import 'y.css'", description: "no empty line blockless pair" },
        { code: "@import 'x.css';", description: "single blockless rule" },
        { code: "a {\n  @mixin foo;\n  @mixin bar;\n}" },
    ],
    reject: [
        {
            code: "@keyframes foo {}\n@import 'x.css'",
            fixed: "@keyframes foo {}\n\n@import 'x.css'",
            message: EXPECTED,
            fix: { range: [17, 18], text: "\n\n" },
        },
        {
            code: "@import 'x.css';\n\n@import 'y.css'",
            fixed: "@import 'x.css';\n@import 'y.css'",
            message: REJECTED,
            fix: { range: [17, 18], text: "" },
        },
        {
            code: "a {\n\n  @mixin foo;\n  color: pink;\n}",
            fixed: "a {\n  @mixin foo;\n  color: pink;\n}",
            message: REJECTED,
            fix: { range: [4, 5], text: "" },
        },
        {
            code: "a {\n\n  color: pink;\n\n  @mixin foo;\n}",
            fixed: "a {\n\n  color: pink;\n  @mixin foo;\n}",
            message: REJECTED,
            fix: { range: [20, 21], text: "" },
        },
    ],
}

test_rule! {
    name: never_default,
    ruleName: "at-rule-empty-line-before",
    config: ["never"],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {}\n\nb {}", description: "rule ignored" },
        { code: "\n\n@font-face {}", description: "first node ignored" },
        { code: "a {}\n@media {}" },
        { code: "a {} @media {}" },
        { code: "@keyframes foo {}\n@media {}" },
        { code: "@keyframes foo {} @media {}" },
        { code: "a {\n  @mixin foo;\n}" },
    ],
    reject: [
        {
            code: "a {}\n\n@media {}",
            fixed: "a {}\n@media {}",
            message: REJECTED,
            fix: { range: [5, 6], text: "" },
        },
        {
            code: "a {}\r\n\r\n@media {}",
            fixed: "a {}\r\n@media {}",
            message: REJECTED,
            fix: { range: [6, 8], text: "" },
        },
        {
            code: "@keyframes foo {}\n/* comment */\n\n@media {}",
            fixed: "@keyframes foo {}\n/* comment */\n@media {}",
            message: REJECTED,
            fix: { range: [32, 33], text: "" },
        },
        {
            code: "@keyframes foo {}\r\n/* comment */\r\n\r\n@media {}",
            fixed: "@keyframes foo {}\r\n/* comment */\r\n@media {}",
            message: REJECTED,
            fix: { range: [34, 36], text: "" },
        },
    ],
}

test_rule! {
    name: never_except_blockless_after_blockless,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "except": ["blockless-after-blockless"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {\n  @mixin foo;\n}" },
        { code: "@keyframes foo {}\n@import 'x.css'", description: "no empty line not blockless pair" },
        { code: "@import 'x.css';\n\n@import 'y.css'", description: "empty line blockless pair" },
        { code: "@import 'x.css';", description: "single blockless rule" },
    ],
    reject: [
        {
            code: "@keyframes foo {}\n\n@import 'x.css'",
            fixed: "@keyframes foo {}\n@import 'x.css'",
            message: REJECTED,
            fix: { range: [18, 19], text: "" },
        },
        {
            code: "@import 'x.css';\n@import 'y.css'",
            fixed: "@import 'x.css';\n\n@import 'y.css'",
            message: EXPECTED,
            fix: { range: [16, 17], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: never_except_inside_block,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "except": ["inside-block"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {\n\n  @mixin foo;\n  color: pink;\n}" },
        { code: "a {\n  color: pink;\n\n  @mixin foo;\n}" },
    ],
    reject: [
        {
            code: "a {\n  @mixin foo;\n  color: pink;\n}",
            fixed: "a {\n\n  @mixin foo;\n  color: pink;\n}",
            message: EXPECTED,
            fix: { range: [3, 4], text: "\n\n" },
        },
        {
            code: "a {\n\n  color: pink;\n  @mixin foo;\n}",
            fixed: "a {\n\n  color: pink;\n\n  @mixin foo;\n}",
            message: EXPECTED,
            fix: { range: [19, 20], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: never_except_first_nested,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "except": ["first-nested"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {\n\n  @mixin foo;\n  color: pink;\n}" },
    ],
    reject: [
        {
            code: "a {\n  color: pink;\n\n  @mixin foo;\n}",
            fixed: "a {\n  color: pink;\n  @mixin foo;\n}",
            message: REJECTED,
            fix: { range: [19, 20], text: "" },
        },
        {
            code: "a {\n  @mixin foo;\n  color: pink;\n}",
            fixed: "a {\n\n  @mixin foo;\n  color: pink;\n}",
            message: EXPECTED,
            fix: { range: [3, 4], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: never_ignore_blockless_after_blockless,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "ignore": ["blockless-after-blockless"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "\n      @media {};\n      @import 'x.css';\n    " },
        { code: "\n      @import 'x.css';\n      @import 'y.css';\n    " },
        { code: "\n      @import 'x.css';\n\n      @import 'y.css';\n    " },
    ],
    reject: [
        {
            code: "\n      @import 'x.css';\n\n      @media {};\n    ",
            fixed: "\n      @import 'x.css';\n      @media {};\n    ",
            message: REJECTED,
            line: 4,
            column: 7,
            fix: { range: [24, 25], text: "" },
        },
    ],
}

test_rule! {
    name: never_ignore_after_comment,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "ignore": ["after-comment"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "/* foo */\n@media {}" },
        { code: "/* foo */\r\n\r\n@media {}", description: "CRLF" },
        { code: "/* foo */\n\n@media {}" },
    ],
    reject: [
        {
            code: "b {}\n\n@media {}",
            fixed: "b {}\n@media {}",
            message: REJECTED,
            fix: { range: [5, 6], text: "" },
        },
        {
            code: "b {}\r\n\r\n@media {}",
            fixed: "b {}\r\n@media {}",
            description: "CRLF",
            message: REJECTED,
            fix: { range: [6, 8], text: "" },
        },
        {
            code: "b {} /* comment */\n\n@media {}",
            fixed: "b {} /* comment */\n@media {}",
            description: "after shared-line comment",
            message: REJECTED,
            line: 3,
            column: 1,
            fix: { range: [19, 20], text: "" },
        },
    ],
}

test_rule! {
    name: never_ignore_inside_block,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "ignore": ["inside-block"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "a {\n  @mixin foo;\n  color: pink;\n}" },
        { code: "a {\n  color: pink;\n  @mixin foo;\n}" },
        { code: "a {\n\n  @mixin foo;\n  color: pink;\n}" },
        { code: "a {\n\n  color: pink;\n\n  @mixin foo;\n}" },
    ],
}

test_rule! {
    name: always_ignore_at_rules_else,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "ignoreAtRules": ["else"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "\n      @if(true) {\n      } @else {\n      }\n    " },
        { code: "\n      @if(true) {\n      }\n      @else {\n      }\n    " },
        { code: "\n      @if(true) {}\n\n      @else if(true) {\n      }  @else {\n      }\n    " },
    ],
    reject: [
        {
            code: "\n      @if(true) {\n      } @else-mixin {\n      }\n    ",
            fixed: "\n      @if(true) {\n      }\n\n @else-mixin {\n      }\n    ",
            message: EXPECTED,
            line: 3,
            column: 9,
            fix: { range: [25, 26], text: "}\n\n" },
        },
        {
            code: "\n      @if (true) {}\n      @if (false) {\n      }\n    ",
            fixed: "\n      @if (true) {}\n\n      @if (false) {\n      }\n    ",
            message: EXPECTED,
            line: 3,
            column: 7,
            fix: { range: [20, 21], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: always_ignore_at_rules_regex_prefix,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "ignoreAtRules": "/^--my-/" }],
    accept: [
        { code: "\n\t\t\ta {}\n\t\t\t@--my-at-rule {}\n\t\t" },
    ],
}

test_rule! {
    name: always_ignore_at_rules_regex_el,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "ignoreAtRules": "/el/" }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "@keyframes {}; @an-element-mixin();" },
        { code: "\n      @if true {}\n      @else {\n      }\n    " },
    ],
    reject: [
        {
            code: "\n      @else {\n        color: pink;\n      }\n      @if true {}\n    ",
            fixed: "\n      @else {\n        color: pink;\n      }\n\n      @if true {}\n    ",
            message: EXPECTED,
            line: 5,
            column: 7,
            fix: { range: [43, 44], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: never_ignore_at_rules_else,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "ignoreAtRules": ["else"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "\n      @if(true) {\n      } @else {\n      }\n    " },
        { code: "\n      @if(true) {\n      }\n\n      @else {\n      }\n    " },
        { code: "\n      @if(true) {}\n\n      @else if(true) {}\n\n      @else {}\n    " },
    ],
    reject: [
        {
            code: "\n      @if(true) {\n      }\n\n      @else-mixin {}\n    ",
            fixed: "\n      @if(true) {\n      }\n      @else-mixin {}\n    ",
            message: REJECTED,
            line: 5,
            column: 7,
            fix: { range: [27, 28], text: "" },
        },
        {
            code: "\n      @if (true)\n      {}\n\n      @if (false) {\n      }\n    ",
            fixed: "\n      @if (true)\n      {}\n      @if (false) {\n      }\n    ",
            message: REJECTED,
            line: 5,
            column: 7,
            fix: { range: [27, 28], text: "" },
        },
    ],
}

test_rule! {
    name: never_ignore_at_rules_regex_el,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "ignoreAtRules": "/el/" }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "\n      @keyframes {}\n\n      @someelement-mixin();\n    " },
        { code: "\n      @if true {}\n\n      @else {\n      }\n    " },
    ],
    reject: [
        {
            code: "\n      @else {\n        color: pink;\n      }\n\n      @if true {}\n    ",
            fixed: "\n      @else {\n        color: pink;\n      }\n      @if true {}\n    ",
            message: REJECTED,
            line: 6,
            column: 7,
            fix: { range: [44, 45], text: "" },
        },
    ],
}

test_rule! {
    name: always_ignore_blockless_after_same_name_blockless,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "ignore": ["blockless-after-same-name-blockless"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "@charset \"UTF-8\";\n\n@import url(x.css);\n@import url(y.css);" },
        { code: "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include loop;\n  @include doo;\n}" },
        {
            code: "@charset \"UTF-8\";\n\n@import url(x.css); /* comment */\n@import url(y.css);",
            description: "shared-line comment accepted",
        },
    ],
    reject: [
        {
            code: "@charset \"UTF-8\";\n@import url(x.css);\n@import url(y.css);",
            fixed: "@charset \"UTF-8\";\n\n@import url(x.css);\n@import url(y.css);",
            message: EXPECTED,
            line: 2,
            column: 1,
            fix: { range: [17, 18], text: "\n\n" },
        },
        {
            code: "a {\n\n  @extends .foo;\n  @extends .bar;\n  @include loop;\n  @include doo;\n}",
            fixed: "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include loop;\n  @include doo;\n}",
            message: EXPECTED,
            line: 5,
            column: 3,
            fix: { range: [38, 39], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: always_except_blockless_after_same_name_blockless,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "except": ["blockless-after-same-name-blockless"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "@charset \"UTF-8\";\n\n@import url(x.css);\n@import url(y.css);" },
        { code: "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include loop;\n  @include doo;\n}" },
        {
            code: "@charset \"UTF-8\";\n\n@import url(x.css); /* comment */\n@import url(y.css);",
            description: "shared-line comment accepted",
        },
    ],
    reject: [
        {
            code: "@charset \"UTF-8\";\n@import url(x.css);\n@import url(y.css);",
            fixed: "@charset \"UTF-8\";\n\n@import url(x.css);\n@import url(y.css);",
            message: EXPECTED,
            line: 2,
            column: 1,
            fix: { range: [17, 18], text: "\n\n" },
        },
        {
            code: "a {\n\n  @extends .foo;\n  @extends .bar;\n  @include loop;\n  @include doo;\n}",
            fixed: "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include loop;\n  @include doo;\n}",
            message: EXPECTED,
            line: 5,
            column: 3,
            fix: { range: [38, 39], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: never_except_blockless_after_same_name_blockless,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "except": ["blockless-after-same-name-blockless"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "@charset \"UTF-8\";\n@import url(x.css);\n\n@import url(y.css);" },
        { code: "a {\n  @extends .foo;\n\n  @extends .bar;\n  @include loop;\n\n  @include doo;\n}" },
    ],
    reject: [
        {
            code: "@charset \"UTF-8\";\n@import url(x.css);\n@import url(y.css);",
            fixed: "@charset \"UTF-8\";\n@import url(x.css);\n\n@import url(y.css);",
            message: EXPECTED,
            line: 3,
            column: 1,
            fix: { range: [37, 38], text: "\n\n" },
        },
        {
            code: "a {\n  @extends .bar;\n  @include loop;\n  @include doo;\n}",
            fixed: "a {\n  @extends .bar;\n  @include loop;\n\n  @include doo;\n}",
            message: EXPECTED,
            line: 4,
            column: 3,
            fix: { range: [37, 38], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: always_except_after_same_name,
    ruleName: "at-rule-empty-line-before",
    config: ["always", { "except": ["after-same-name"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "@charset \"UTF-8\";\n\n@include x;\n@include y {}" },
        { code: "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include y {}\n  @include x;\n}" },
    ],
    reject: [
        {
            code: "@charset \"UTF-8\";\n@include x;\n@include y {}",
            fixed: "@charset \"UTF-8\";\n\n@include x;\n@include y {}",
            message: EXPECTED,
            line: 2,
            column: 1,
            fix: { range: [17, 18], text: "\n\n" },
        },
        {
            code: "a {\n\n  @extends .foo;\n  @extends .bar;\n  @include y {}\n  @include x;\n}",
            fixed: "a {\n\n  @extends .foo;\n  @extends .bar;\n\n  @include y {}\n  @include x;\n}",
            message: EXPECTED,
            line: 5,
            column: 3,
            fix: { range: [38, 39], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: never_except_after_same_name,
    ruleName: "at-rule-empty-line-before",
    config: ["never", { "except": ["after-same-name"] }],
    fix: true,
    computeEditInfo: true,
    accept: [
        { code: "@charset \"UTF-8\";\n@include x;\n\n@include y {}" },
        { code: "a {\n  @extends .foo;\n\n  @extends .bar;\n  @include y {}\n\n  @include x;\n}" },
    ],
    reject: [
        {
            code: "@charset \"UTF-8\";\n@include x;\n@include y {}",
            fixed: "@charset \"UTF-8\";\n@include x;\n\n@include y {}",
            message: EXPECTED,
            line: 3,
            column: 1,
            fix: { range: [29, 30], text: "\n\n" },
        },
        {
            code: "a {\n  @extends .bar;\n  @include x;\n  @include y {}\n}",
            fixed: "a {\n  @extends .bar;\n  @include x;\n\n  @include y {}\n}",
            message: EXPECTED,
            line: 4,
            column: 3,
            fix: { range: [34, 35], text: "\n\n" },
        },
    ],
}

test_rule! {
    name: less_syntax,
    ruleName: "at-rule-empty-line-before",
    config: ["always"],
    customSyntax: "postcss-less",
    computeEditInfo: true,
    accept: [
        {
            code: "\n        .someMixin() { margin: 0; }\n        span { .someMixin(); }\n      ",
            description: "ignore Less mixin",
        },
        {
            code: "\n        @myVariable: #f7f8f9;\n        span { background-color: @myVariable; }\n      ",
            description: "ignore Less variable",
        },
    ],
}
