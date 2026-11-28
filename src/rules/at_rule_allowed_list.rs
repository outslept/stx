//! Specifies a list of allowed at-rules (`at-rule-allowed-list`).
//!
//! # Checked Constructs
//!
//! 1. **Standard syntax at-rules**: Every standard at-rule in the stylesheet is
//!    checked against the configured allowlist of literal strings and `/regex/`
//!    patterns, matched against the at-rule name **without** the leading `@`
//!    symbol.
//! 2. **Ignored non-standard constructs**: Non-standard preprocessor constructs
//!    that PostCSS represents as at-rules are ignored in accordance with
//!    Stylelint's `isStandardSyntaxAtRule`:
//!    - Blockless at-rules with empty parameters (`@content;`).
//!    - Less variable declarations (`@var: 10px;`) and detached ruleset
//!      definitions (`@detached-ruleset: { background: red; };`).
//!    - Less mixin and detached ruleset calls (`@detached-ruleset();`).
//!    - At-rules with SCSS, Less, or PostCSS interpolation in their name
//!      (`@#{$var}`, `@{var}`, `@$(var)`).

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::Value;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, has_interpolation, options_matches, parse_primary_option_matchers},
};

/// Formats the diagnostic message matching Stylelint's `messages.rejected`.
#[must_use]
fn disallowed_at_rule_diagnostic(name: &str) -> String {
    format!("Disallowed at-rule \"@{name}\"")
}

crate::define_stx_lint! {
    /// Specify a list of allowed at-rules.
    ///
    /// Restricts at-rules in a stylesheet to those matching the configured string or `/regex/`
    /// patterns (matched without the leading `@`), ignoring non-standard preprocessor constructs
    /// such as `@content;`, Less variables, detached ruleset calls, and interpolated at-rule names.
    "at-rule-allowed-list", restriction,
    short_description = "Specify a list of allowed at-rules.";

    #[derive(Debug, Default, Clone)]
    pub struct AtRuleAllowedList {
        /// Compiled matchers from the rule's primary configuration option.
        allowed: Vec<OptionMatcher>,
    }
}

impl AtRuleAllowedList {
    /// Constructs [`AtRuleAllowedList`] from the rule's primary option value.
    #[must_use]
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

/// Advances past a `/* ... */` block comment starting at `bytes[i]`.
#[inline]
#[must_use]
fn skip_block_comment(bytes: &[u8], mut i: usize) -> usize {
    i += 2;
    while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
        i += 1;
    }
    (i + 2).min(bytes.len())
}

/// Advances past a single- or double-quoted string literal starting at `bytes[i]`.
#[inline]
#[must_use]
fn skip_quoted(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    let quote = bytes[i];
    i += 1;
    while i < len {
        if bytes[i] == b'\\' {
            i = (i + 2).min(len);
        } else if bytes[i] == quote {
            return i + 1;
        } else {
            i += 1;
        }
    }
    i
}

/// Advances past a balanced pair of delimiters (`open` / `close`, such as
/// `{` / `}` or `(` / `)`) starting at `bytes[i]` where `bytes[i] == open`.
#[inline]
#[must_use]
fn skip_balanced_delims(bytes: &[u8], mut i: usize, open: u8, close: u8) -> usize {
    let len = bytes.len();
    let mut depth = 0usize;
    while i < len {
        if bytes[i] == b'\\' {
            i = (i + 2).min(len);
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            i = skip_quoted(bytes, i);
        } else if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i = skip_block_comment(bytes, i);
        } else if bytes[i] == open {
            depth += 1;
            i += 1;
        } else if bytes[i] == close {
            depth -= 1;
            i += 1;
            if depth == 0 {
                break;
            }
        } else {
            i += 1;
        }
    }
    i
}

/// Scans an at-rule name starting right after `@` at `bytes[i]`, keeping any
/// embedded interpolation blocks (`#{...}`, `@{...}`, `$(...)`) attached to the
/// name so [`has_interpolation`] can detect them.
#[must_use]
fn scan_at_rule_name(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    while i < len {
        if bytes[i] == b'\\' {
            i = (i + 2).min(len);
        } else if matches!(bytes[i], b'#' | b'@') && i + 1 < len && bytes[i + 1] == b'{' {
            i = skip_balanced_delims(bytes, i + 1, b'{', b'}');
        } else if bytes[i] == b'$' && i + 1 < len && bytes[i + 1] == b'(' {
            i = skip_balanced_delims(bytes, i + 1, b'(', b')');
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
    i
}

/// Returns `true` if the at-rule prelude starting at `bytes[i]` ends with an
/// opening `{` block rather than `;`, `}`, or EOF (i.e. `Boolean(atRule.nodes)`
/// in PostCSS).
///
/// Used to distinguish standard CSS at-rules without space before `(` (such as
/// `@media(min-width: 700px) {}`) from Less detached ruleset calls
/// (`@detached-ruleset();`).
#[must_use]
fn has_at_rule_block(bytes: &[u8], mut i: usize) -> bool {
    let len = bytes.len();
    let mut paren_depth = 0usize;
    while i < len {
        match bytes[i] {
            b'\\' => {
                i = (i + 2).min(len);
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i = skip_block_comment(bytes, i);
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'/' && (i == 0 || bytes[i - 1] != b':') => {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' | b'\'' => {
                i = skip_quoted(bytes, i);
            }
            b'#' if i + 1 < len && bytes[i + 1] == b'{' => {
                i = skip_balanced_delims(bytes, i + 1, b'{', b'}');
            }
            b'(' | b'[' => {
                paren_depth += 1;
                i += 1;
            }
            b')' | b']' => {
                paren_depth = paren_depth.saturating_sub(1);
                i += 1;
            }
            b'{' if paren_depth == 0 => return true,
            b';' | b'}' if paren_depth == 0 => return false,
            _ => i += 1,
        }
    }
    false
}

/// Scans `source` for standard syntax at-rules at statement boundaries, invoking
/// `callback(at_rule_name, at_rule_name_span)` for each one.
///
/// Ignores blockless parameterless at-rules (`@content;`), Less variable declarations
/// (`@var: val;`), Less detached ruleset calls (`@mixin();`), and interpolated at-rule
/// names (`@#{var}`).
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
            // Single-line comments `// ...` (excluding `://` in unquoted URLs) preserve `at_stmt_start`.
            b'/' if i + 1 < len && bytes[i + 1] == b'/' && (i == 0 || bytes[i - 1] != b':') => {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            // Escape sequences.
            b'\\' => {
                at_stmt_start = false;
                i = (i + 2).min(len);
            }
            // Skip quoted string literals.
            b'"' | b'\'' => {
                at_stmt_start = false;
                i = skip_quoted(bytes, i);
            }
            // Skip SCSS `#{...}` interpolation blocks so their inner `{}` do not affect `at_stmt_start`.
            b'#' if i + 1 < len && bytes[i + 1] == b'{' => {
                at_stmt_start = false;
                i = skip_balanced_delims(bytes, i + 1, b'{', b'}');
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
                let name_start = i + 1;
                let name_end = scan_at_rule_name(bytes, name_start);
                i = name_end;

                let name = &source[name_start..name_end];
                at_stmt_start = false;

                if name.is_empty() || has_interpolation(name) {
                    continue;
                }

                // Check what follows `@name` to filter out blockless parameterless at-rules
                // (`@content;`), Less variables (`@var: ...`), and Less detached ruleset calls (`@mixin();`).
                let immediate_open_paren = i < len && bytes[i] == b'(';

                let mut lookahead = i;
                while lookahead < len {
                    match bytes[lookahead] {
                        b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' => lookahead += 1,
                        b'/' if lookahead + 1 < len && bytes[lookahead + 1] == b'*' => {
                            lookahead = skip_block_comment(bytes, lookahead);
                        }
                        b'/' if lookahead + 1 < len
                            && bytes[lookahead + 1] == b'/'
                            && (lookahead == 0 || bytes[lookahead - 1] != b':') =>
                        {
                            lookahead += 2;
                            while lookahead < len && bytes[lookahead] != b'\n' {
                                lookahead += 1;
                            }
                        }
                        _ => break,
                    }
                }

                if lookahead >= len || matches!(bytes[lookahead], b';' | b'}') {
                    continue;
                }

                if bytes[lookahead] == b':' {
                    continue;
                }

                if immediate_open_paren && !has_at_rule_block(bytes, i) {
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
    use crate::tester::{Tester, reject, reject_warnings};

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

    #[test]
    fn edge_cases_and_preprocessors() {
        Tester::new(
            AtRuleAllowedList::NAME,
            json!(["keyframes", "import"]),
            &[
                "/* @media screen {} */",
                "a { content: \"@media screen {}\"; }",
                "/* 日本語コメント */ @keyframes fade { from { opacity: 0; } to { opacity: 1; } }",
            ],
            &[
                reject(
                    "@media(min-width: 700px) {}",
                    None,
                    rejected("@media"),
                    (1, 1, 1, 7),
                ),
                reject(
                    "@supports(display: grid) {}",
                    None,
                    rejected("@supports"),
                    (1, 1, 1, 10),
                ),
                reject(
                    "@import url(http://example.com/a.css); @media screen {}",
                    None,
                    rejected("@media"),
                    (1, 40, 1, 46),
                ),
            ],
        )
        .test();

        Tester::new(
            AtRuleAllowedList::NAME,
            json!(["keyframes"]),
            &[
                "@#{$var} {}",
                "@prefix-#{$var} {}",
                "@#{$var}-suffix {}",
                "// @import 'foo.css';\n@keyframes fade {}",
                "@keyframes fade { @content; }",
            ],
            &[
                reject_warnings(
                    "@mixin foo { @content; }",
                    None,
                    vec![(rejected("@mixin"), (1, 1, 1, 7))],
                ),
                reject(
                    "#{$selector} { @import 'foo.css'; }",
                    None,
                    rejected("@import"),
                    (1, 16, 1, 23),
                ),
            ],
        )
        .with_custom_syntax("postcss-scss")
        .test();

        Tester::new(
            AtRuleAllowedList::NAME,
            json!(["keyframes"]),
            &[
                "@var: 10px;\na { width: @var; }",
                "@detached-ruleset: { background: red; };\n.top { @detached-ruleset(); }",
                "@{var} {}",
            ],
            &[],
        )
        .with_custom_syntax("postcss-less")
        .test();
    }
}
