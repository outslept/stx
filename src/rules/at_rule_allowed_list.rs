//! Rule: `at-rule-allowed-list`
//!
//! Specify a list of allowed at-rules.

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::Value;

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::{
    OptionMatcher, has_interpolation, options_matches, parse_primary_option_matchers,
};

/// ## `at-rule-allowed-list`
///
/// Specify a list of allowed at-rules.
///
/// ### What it does
///
/// Restricts the at-rules permitted in a stylesheet to an explicit allowlist of names
/// or regular expression patterns.
///
/// ### Why is this bad?
///
/// Projects may want to restrict the use of certain preprocessor directives (such as
/// `@mixin` or `@Debug`) or limit stylesheets to a specific subset of CSS at-rules
/// (`@media`, `@keyframes`, `@import`).
///
/// ### Options
///
/// #### Primary
///
/// `string | RegExp | Array<string | RegExp>`: Allowed at-rule names (without the leading `@`)
/// or regex strings.
///
/// ```json
/// {
///   "at-rule-allowed-list": ["extend", "import", "/^(-webkit-|-moz-)?keyframes$/i"]
/// }
/// ```
///
/// ### Examples
///
/// #### With `["extend", "import", "/^(-webkit-|-moz-)?keyframes$/i"]`
///
/// **Incorrect:**
/// ```scss
/// @mixin name() {}
/// ```
/// ```css
/// @media screen and (max-width: 1000px) {}
/// ```
///
/// **Correct:**
/// ```scss
/// a {
///   @extend %placeholder;
/// }
/// ```
/// ```css
/// @keyframes name {
///   from { top: 10px; }
///   to { top: 20px; }
/// }
/// ```
///
/// ### References
///
/// - [Stylelint Documentation: `at-rule-allowed-list`](https://stylelint.io/user-guide/rules/at-rule-allowed-list)
#[derive(Default)]
pub struct AtRuleAllowedList {
    /// Compiled matchers for allowed at-rule names.
    pub allowed: Vec<OptionMatcher>,
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
        "at-rule-allowed-list"
    }

    fn check_stylesheet<'a>(&self, _stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        // Fast path: skip if there are no `@` characters in the source.
        if !ctx.source.contains('@') {
            return;
        }

        for_each_standard_at_rule(ctx.source, |name, span| {
            if options_matches(&self.allowed, name) {
                return;
            }

            ctx.report(self.name(), span, format!("Disallowed at-rule \"@{name}\""));
        });
    }
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
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
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
                            lookahead += 2;
                            while lookahead + 1 < len
                                && !(bytes[lookahead] == b'*' && bytes[lookahead + 1] == b'/')
                            {
                                lookahead += 1;
                            }
                            lookahead = (lookahead + 2).min(len);
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
