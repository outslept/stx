//! Rule `at-rule-no-deprecated`: disallow deprecated at-rules.
//!
//! Reference: <https://stylelint.io/user-guide/rules/at-rule-no-deprecated>

use oxc_css_parser::{Span, ast::Stylesheet};
use serde_json::{Map, Value};

use crate::context::{Fix, LintContext};
use crate::rules::Rule;
use crate::utils::options::{OptionMatcher, options_matches, parse_option_matchers};
use crate::utils::reference::{DEPRECATED_AT_KEYWORDS, in_set_ignore_case};

pub struct AtRuleNoDeprecated {
    /// Compiled matchers for the `ignoreAtRules` secondary option.
    ignore_at_rules: Vec<OptionMatcher>,
}

impl Default for AtRuleNoDeprecated {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AtRuleNoDeprecated {
    /// Creates a new `at-rule-no-deprecated` rule instance from secondary options.
    pub fn new(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_at_rules: parse_option_matchers(secondary, "ignoreAtRules"),
        }
    }

    /// Formats the diagnostic message when a deprecated at-rule is encountered.
    #[inline]
    fn rejected_message(at_rule: &str) -> String {
        format!("Deprecated at-rule \"{at_rule}\"")
    }
}

impl Rule for AtRuleNoDeprecated {
    fn name(&self) -> &'static str {
        "at-rule-no-deprecated"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let bytes = ctx.source.as_bytes();
        let len = bytes.len();
        let mut i: u32 = 0;
        let len_u32 = len as u32;
        // Track whether the scanner is positioned where a statement / at-rule may start
        // (start of file, or after `{`, `}`, `;` ignoring whitespace and comments).
        let mut at_statement_start = true;
        while i < len_u32 {
            let b = bytes[i as usize];
            // Whitespace preserves `at_statement_start`.
            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }
            // Skip block comments `/* ... */` (preserving `at_statement_start`).
            if b == b'/' && i + 1 < len_u32 && bytes[(i + 1) as usize] == b'*' {
                i += 2;
                while i + 1 < len_u32
                    && !(bytes[i as usize] == b'*' && bytes[(i + 1) as usize] == b'/')
                {
                    i += 1;
                }
                if i + 1 < len_u32 {
                    i += 2;
                } else {
                    i = len_u32;
                }
                continue;
            }
            // Skip single-line comments `// ...` when not part of a URL scheme (`://`).
            if b == b'/'
                && i + 1 < len_u32
                && bytes[(i + 1) as usize] == b'/'
                && (i == 0 || bytes[(i - 1) as usize] != b':')
            {
                i += 2;
                while i < len_u32 && bytes[i as usize] != b'\n' {
                    i += 1;
                }
                continue;
            }
            // Skip quoted string literals.
            if b == b'"' || b == b'\'' {
                at_statement_start = false;
                let quote = b;
                i += 1;
                while i < len_u32 {
                    if bytes[i as usize] == b'\\' {
                        i = (i + 2).min(len_u32);
                        continue;
                    }
                    if bytes[i as usize] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            // Statement delimiters allow an at-rule to follow.
            if matches!(b, b'{' | b'}' | b';') {
                at_statement_start = true;
                i += 1;
                continue;
            }
            // Check for an at-rule at the start of a statement.
            if b == b'@' && at_statement_start {
                let at_start = i;
                let name_start = i + 1;
                if name_start < len_u32 && is_ident_start(bytes[name_start as usize]) {
                    let mut name_end = name_start + 1;
                    while name_end < len_u32 && is_ident_char(bytes[name_end as usize]) {
                        name_end += 1;
                    }
                    // Ensure standard syntax at-rule (ignore Less variables `@var:` and mixins `@mixin()`).
                    let next_non_ws = skip_spaces_and_tabs(bytes, name_end, len_u32);
                    let is_less_construct = next_non_ws < len_u32
                        && (bytes[next_non_ws as usize] == b':'
                            || (next_non_ws == name_end && bytes[next_non_ws as usize] == b'('));
                    if !is_less_construct {
                        let name = ctx.slice_range(name_start, name_end);
                        if !options_matches(&self.ignore_at_rules, name)
                            && in_set_ignore_case(DEPRECATED_AT_KEYWORDS, name)
                        {
                            let at_name = ctx.slice_range(at_start, name_end);
                            let span = Span {
                                start: at_start,
                                end: name_end,
                            };
                            // `@nest` is autofixable by stripping `@nest` and the trailing `afterName` whitespace.
                            let fix = if name.eq_ignore_ascii_case("nest") {
                                let mut fix_end = name_end;
                                while fix_end < len_u32
                                    && bytes[fix_end as usize].is_ascii_whitespace()
                                {
                                    fix_end += 1;
                                }
                                Some(Fix::single(
                                    Span {
                                        start: at_start,
                                        end: fix_end,
                                    },
                                    String::new(),
                                ))
                            } else {
                                None
                            };
                            ctx.report_with_fix(
                                self.name(),
                                span,
                                Self::rejected_message(at_name),
                                fix,
                            );
                        }
                    }
                    at_statement_start = false;
                    i = name_end;
                    continue;
                }
            }
            at_statement_start = false;
            i += 1;
        }
    }
}

/// Returns `true` if `b` can start a CSS identifier (ASCII letter, `_`, `-`, or non-ASCII byte).
#[inline]
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-' || b >= 0x80
}

/// Returns `true` if `b` can continue a CSS identifier.
#[inline]
fn is_ident_char(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

/// Advances past horizontal ASCII whitespace (`' '` and `'\t'`).
#[inline]
fn skip_spaces_and_tabs(bytes: &[u8], mut idx: u32, len: u32) -> u32 {
    while idx < len && matches!(bytes[idx as usize], b' ' | b'\t') {
        idx += 1;
    }
    idx
}
