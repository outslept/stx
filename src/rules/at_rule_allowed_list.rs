//! Rule: `at-rule-allowed-list`
//!
//! Specify a list of allowed at-rules.

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::Value;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, has_interpolation, options_matches, parse_primary_option_matchers},
};

fn disallowed_at_rule_diagnostic(name: &str) -> String {
    format!("Disallowed at-rule \"@{name}\"")
}

crate::define_stx_lint! {
    /// Allow only at-rules whose names match a configured list.
    ///
    /// The primary setting is a name, a slash-delimited regex string, or a list of
    /// either, matched without `@`. There are no secondary options. An empty list
    /// allows no checked at-rules. Less variables and detached ruleset calls are skipped.
    ///
    /// With `["media", "/^(-webkit-)?keyframes$/"]`:
    ///
    /// ```css
    /// @import "base.css"; /* reported */
    /// @media print { a { color: black; } } /* accepted */
    /// ```
    "at-rule-allowed-list", restriction;
    #[derive(Debug, Default, Clone)]
    pub struct AtRuleAllowedList {
        allowed: Vec<OptionMatcher>,
    }
}

impl AtRuleAllowedList {
    /// Constructs [`AtRuleAllowedList`] from the rule's primary option value.
    pub fn from_primary(primary: &Value) -> Self {
        Self {
            allowed: parse_primary_option_matchers(primary),
        }
    }
}

impl Rule for AtRuleAllowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        if !ctx.source.contains('@') {
            return;
        }

        for_each_standard_at_rule(ctx.source, |name, span| {
            if options_matches(&self.allowed, name) {
                return;
            }
            ctx.report(Self::NAME, span, disallowed_at_rule_diagnostic(name));
        });
    }
}

fn skip_block_comment(bytes: &[u8], mut i: usize) -> usize {
    i += 2;
    while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
        i += 1;
    }
    (i + 2).min(bytes.len())
}

/// Scans `source` for standard syntax at-rules at statement boundaries, invoking
/// `callback(at_rule_name, at_rule_name_span)` for each one.
///
/// Ignores Less variable declarations (`@var: val;`), Less detached ruleset calls (`@mixin();`),
/// and interpolated at-rule names (`@#{var}`).
fn for_each_standard_at_rule<'a>(source: &'a str, mut callback: impl FnMut(&'a str, Span)) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut paren_depth: usize = 0;
    let mut at_stmt_start = true;

    while i < len {
        match bytes[i] {
            // Whitespace preserves `at_stmt_start`.
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' => {
                i += 1;
            }
            // Block comments `/* ... */` preserve `at_stmt_start`.
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i = skip_block_comment(bytes, i);
            }
            // Single-line comments `// ...` preserve `at_stmt_start`.
            b'/' if i + 1 < len && bytes[i + 1] == b'/' => {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            // Skip quoted string literals.
            b'"' | b'\'' => {
                at_stmt_start = false;
                let quote = bytes[i];
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(len);
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
            }
            b'(' | b'[' => {
                paren_depth += 1;
                at_stmt_start = false;
                i += 1;
            }
            b')' | b']' => {
                paren_depth = paren_depth.saturating_sub(1);
                at_stmt_start = false;
                i += 1;
            }
            b';' | b'{' | b'}' if paren_depth == 0 => {
                at_stmt_start = true;
                i += 1;
            }
            b'@' if at_stmt_start && paren_depth == 0 => {
                let at_start = i;
                i += 1;
                let name_start = i;
                while i < len {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(len);
                    } else if matches!(
                        bytes[i],
                        b' ' | b'\t'
                            | b'\n'
                            | b'\r'
                            | b'\x0C'
                            | b';'
                            | b'{'
                            | b'}'
                            | b'('
                            | b')'
                            | b'['
                            | b']'
                            | b':'
                            | b'"'
                            | b'\''
                            | b'/'
                    ) {
                        break;
                    } else {
                        i += 1;
                    }
                }
                let name_end = i;
                let name = &source[name_start..name_end];
                at_stmt_start = false;

                if name.is_empty() || has_interpolation(name) {
                    continue;
                }

                // Check what follows `@name` to filter out Less variables (`@var: ...`)
                // and Less detached ruleset calls (`@mixin();`).
                let immediate_open_paren = i < len && bytes[i] == b'(';
                let mut lookahead = i;
                while lookahead < len {
                    match bytes[lookahead] {
                        b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' => lookahead += 1,
                        b'/' if lookahead + 1 < len && bytes[lookahead + 1] == b'*' => {
                            lookahead = skip_block_comment(bytes, lookahead);
                        }
                        _ => break,
                    }
                }

                // Less variable declaration: `@var: value;`
                if lookahead < len && bytes[lookahead] == b':' {
                    continue;
                }

                // Less mixin / detached ruleset call: `@mixin();`
                if immediate_open_paren {
                    continue;
                }

                let span = Span {
                    start: at_start as u32,
                    end: name_end as u32,
                };
                callback(name, span);
            }
            _ => {
                at_stmt_start = false;
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AtRuleAllowedList, disallowed_at_rule_diagnostic};
    use crate::tester::{Tester, reject};

    fn rejected(at_rule: &str) -> String {
        disallowed_at_rule_diagnostic(at_rule.strip_prefix('@').unwrap_or(at_rule))
    }

    #[test]
    fn array_config() {
        let pass = &[
            "a { color: pink; }",
            "a { @extend %placeholder; }",
            "\n      a {\n        @extend\n        %placeholder;\n      }\n    ",
            "\n      @keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            "\n      @Keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            "\n      @-moz-keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            "\n\t@-WEBKIT-KEYFRAMES name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
        ];

        let fail = &[reject(
            "\n      @mixin name () {}\n    ",
            None,
            rejected("@mixin"),
            (2, 7, 2, 13),
        )];

        Tester::new(
            AtRuleAllowedList::NAME,
            json!(["extend", "import", "/^(-webkit-|-moz-)?keyframes$/i"]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn string_regex_config() {
        let pass = &[
            "\n      @keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            "\n      @Keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            "\n      @-moz-keyframes name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
            "\n\t@-WEBKIT-KEYFRAMES name {\n        from { top: 10px; }\n        to { top: 20px; }\n      }\n    ",
        ];

        let fail = &[
            reject(
                "\n      @mixin name ($p) {}\n    ",
                None,
                rejected("@mixin"),
                (2, 7, 2, 13),
            ),
            reject(
                "@import 'path/to/file.css';",
                None,
                rejected("@import"),
                (1, 1, 1, 8),
            ),
            reject(
                "@media screen and (max-witdh: 1000px) {}",
                None,
                rejected("@media"),
                (1, 1, 1, 7),
            ),
        ];

        Tester::new(
            AtRuleAllowedList::NAME,
            json!("/^(-webkit-|-moz-)?keyframes$/i"),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn less_syntax() {
        let pass = &["\n        .mixin() { margin: 0; }\n        span { .mixin(); }\n      "];

        Tester::new(AtRuleAllowedList::NAME, json!(["keyframes"]), pass, &[])
            .with_custom_syntax("postcss-less")
            .test();
    }
}
