//! Rule: `annotation-no-unknown`
//!
//! Disallow unknown annotations within declarations.

use oxc_css_parser::{Span, ast::Declaration};
use serde_json::{Map, Value};

use crate::{
    context::LintContext,
    declare_stx_lint,
    rules::Rule,
    utils::{
        OptionMatcher, decl_value_raw, is_standard_syntax_declaration, is_standard_syntax_value,
        options_matches, parse_option_matchers,
    },
};

fn unknown_annotation_diagnostic(annotation: &str) -> String {
    format!("Unknown annotation \"{annotation}\"")
}

#[derive(Debug, Default, Clone)]
pub struct AnnotationNoUnknown {
    ignore_annotations: Vec<OptionMatcher>,
}

declare_stx_lint! {
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
    AnnotationNoUnknown,
    "annotation-no-unknown",
    correctness,
    fix = none,
    recommended = true,
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
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if !is_standard_syntax_declaration(decl) {
            return;
        }

        let Some((raw_value, value_span)) = decl_value_raw(ctx.source, decl) else {
            return;
        };

        if !raw_value.contains('!') || !is_standard_syntax_value(raw_value) {
            return;
        }

        for_each_annotation_word(raw_value, |word, rel_start, rel_end| {
            let token_value = &word[1..];
            if token_value.eq_ignore_ascii_case("important")
                || options_matches(&self.ignore_annotations, token_value)
            {
                return;
            }

            let span = Span {
                start: value_span.start + rel_start,
                end: value_span.start + rel_end,
            };
            ctx.report(Self::NAME, span, unknown_annotation_diagnostic(word));
        });
    }
}

fn skip_quoted(bytes: &[u8], mut i: usize) -> usize {
    let quote = bytes[i];
    i += 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i = (i + 2).min(bytes.len()),
            b if b == quote => return i + 1,
            _ => i += 1,
        }
    }
    i
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
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' | b',' | b':' | b'/' | b'(' | b')' => {
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
            b'"' | b'\'' => i = skip_quoted(bytes, i),
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

                // `postcss-value-parser` treats `url(...)` arguments as a single token,
                // so `!` inside `url(https://example.com/!important)` is not an annotation word.
                if word.eq_ignore_ascii_case("url") && i < len && bytes[i] == b'(' {
                    i += 1;
                    while i < len && bytes[i] != b')' {
                        match bytes[i] {
                            b'"' | b'\'' => i = skip_quoted(bytes, i),
                            _ => i += 1,
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AnnotationNoUnknown, unknown_annotation_diagnostic};
    use crate::tester::{Tester, reject};

    #[test]
    fn ignore_annotations() {
        let pass = [
            "a { color: green !important; }",
            "a { color: green !IMPORTANT; }",
            "a { color: green !ImPoRtAnT; }",
            "a { color: green !--foo--bar; }",
            "a { color: green !--bar; }",
        ];

        let fail = [
            reject(
                "a { color: green !imprtant }",
                None,
                unknown_annotation_diagnostic("!imprtant"),
                (1, 18, 1, 27),
            ),
            reject(
                "a { color: green !IMPRTANT }",
                None,
                unknown_annotation_diagnostic("!IMPRTANT"),
                (1, 18, 1, 27),
            ),
            reject(
                "a { color: green !ImPrTaNt }",
                None,
                unknown_annotation_diagnostic("!ImPrTaNt"),
                (1, 18, 1, 27),
            ),
        ];

        Tester::new(
            AnnotationNoUnknown::NAME,
            json!([
                true,
                {
                    "ignoreAnnotations": ["/^--foo-/", "--bar"]
                }
            ]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn scss_syntax() {
        let pass = ["a { color: #{$green} !imprtant; }"];

        Tester::new(AnnotationNoUnknown::NAME, json!([true]), pass, [])
            .with_custom_syntax("postcss-scss")
            .test();
    }
}
