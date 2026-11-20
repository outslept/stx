//! Rule: `annotation-no-unknown`
//!
//! Disallow unknown annotations within declarations.

use oxc_css_parser::{Span, ast::Declaration};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::{
    OptionMatcher, decl_value_raw, is_standard_syntax_value, options_matches, parse_option_matchers,
};

/// ## `annotation-no-unknown`
///
/// Disallow unknown annotations in declaration values (such as `!imprtant` instead of `!important`).
///
/// ### What it does
///
/// Checks declaration values for annotation tokens (words starting with `!`) that are not
/// recognized as standard CSS `!important`.
///
/// ### Why is this bad?
///
/// Unknown annotations are almost always typos of `!important` (e.g. `!imprtant`) or leftover
/// preprocessor flags in plain CSS. Browsers ignore or invalidate declarations containing
/// unrecognized annotations.
///
/// ### Options
///
/// #### Primary
///
/// - `true`: Enable the rule.
///
/// ```json
/// {
///   "annotation-no-unknown": true
/// }
/// ```
///
/// #### Secondary (optional)
///
/// - `"ignoreAnnotations": ["array", "of", "strings", "/or/", "/regex/"]`: Ignore specific
///   annotations (matched without the leading `!`).
///
/// ```json
/// {
///   "annotation-no-unknown": [
///     true,
///     {
///       "ignoreAnnotations": ["default", "/^--foo-/", "--bar"]
///     }
///   ]
/// }
/// ```
///
/// ### Examples
///
/// #### Default (`true`)
///
/// **Incorrect:**
/// ```css
/// a {
///   color: green !imprtant;
/// }
/// ```
///
/// **Correct:**
/// ```css
/// a {
///   color: green !important;
/// }
/// ```
///
/// #### With `{ "ignoreAnnotations": ["/^--foo-/", "--bar"] }`
///
/// **Correct:**
/// ```css
/// a {
///   color: green !--foo--bar;
///   color: green !--bar;
/// }
/// ```
///
/// ### References
///
/// - [Stylelint Documentation: `annotation-no-unknown`](https://stylelint.io/user-guide/rules/annotation-no-unknown)
#[derive(Default)]
pub struct AnnotationNoUnknown {
    /// Compiled matchers for the `"ignoreAnnotations"` secondary option.
    pub ignore_annotations: Vec<OptionMatcher>,
}

impl AnnotationNoUnknown {
    /// Constructs [`AnnotationNoUnknown`] from the rule's secondary options object.
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_annotations: parse_option_matchers(secondary, "ignoreAnnotations"),
        }
    }
}

impl Rule for AnnotationNoUnknown {
    fn name(&self) -> &'static str {
        "annotation-no-unknown"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        // Standard `!important` (in any ASCII case) is already parsed into `decl.important`.
        if decl.important.is_some() {
            return;
        }

        let Some((raw_value, value_span)) = decl_value_raw(ctx.source, decl) else {
            return;
        };

        // Fast path: skip if there is no `!` or if the value uses non-standard preprocessor
        // interpolation (e.g. `color: #{$green} !imprtant;`).
        if !raw_value.contains('!') || !is_standard_syntax_value(raw_value) {
            return;
        }

        for_each_annotation_word(raw_value, |word, rel_start, rel_end| {
            let token_value = &word[1..];
            if options_matches(&self.ignore_annotations, token_value) {
                return;
            }

            let span = Span {
                start: value_span.start + rel_start,
                end: value_span.start + rel_end,
            };
            ctx.report(self.name(), span, format!("Unknown annotation \"{word}\""));
        });
    }
}

/// Scans a raw CSS declaration value in the same manner as `postcss-value-parser`,
/// invoking `callback(word, start_offset, end_offset)` for every value word token
/// that starts with `'!'`.
fn for_each_annotation_word<'a>(raw: &'a str, mut callback: impl FnMut(&'a str, u32, u32)) {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        match bytes[i] {
            // Whitespace, dividers (`,`, `:`, `/`), and parentheses act as word boundaries.
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' | b',' | b':' | b'/' | b'(' | b')' => {
                // Skip block comments `/* ... */`.
                if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                    i += 2;
                    while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                        i += 1;
                    }
                    i = (i + 2).min(len);
                } else {
                    i += 1;
                }
            }
            // Skip quoted string literals (`"..."` and `'...'`).
            b'"' | b'\'' => {
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
            // Consume a value word token.
            _ => {
                let word_start = i;
                while i < len {
                    match bytes[i] {
                        b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' | b',' | b':' | b'/' | b'('
                        | b')' | b'"' | b'\'' => break,
                        _ => i += 1,
                    }
                }

                let word = &raw[word_start..i];

                // In `postcss-value-parser`, `url(...)` arguments are treated as a single token,
                // so `!` inside `url(https://example.com/!important)` is not an annotation word.
                if word.eq_ignore_ascii_case("url") && i < len && bytes[i] == b'(' {
                    i += 1;
                    while i < len && bytes[i] != b')' {
                        if bytes[i] == b'"' || bytes[i] == b'\'' {
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
                        } else {
                            i += 1;
                        }
                    }
                    if i < len && bytes[i] == b')' {
                        i += 1;
                    }
                    continue;
                }

                if word.starts_with('!') {
                    callback(word, word_start as u32, i as u32);
                }
            }
        }
    }
}
