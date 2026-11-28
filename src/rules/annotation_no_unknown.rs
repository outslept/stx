//! Disallows unknown annotations within declarations (`annotation-no-unknown`).
//!
//! # Behavior
//!
//! In CSS, `!important` (case-insensitive) is the only standard annotation
//! allowed on declarations. Preprocessors such as Sass define additional
//! annotations (`!default`, `!global`, `!optional`) for variable and `@extend`
//! statements, which are frequently written by mistake on standard CSS
//! declarations or misspelled (e.g. `!imprtant`).
//!
//! This rule inspects standard CSS declarations and flags any standalone value
//! word starting with `!` unless:
//! - the declaration carries a valid trailing `!important` flag,
//! - the declaration value uses non-standard preprocessor interpolation (e.g.
//!   `#{$green}`), or
//! - the annotation name (without the leading `!`) matches one of the patterns
//!   configured in `ignoreAnnotations`.
//!
//! Quoted strings (`"!imprtant"`), CSS comments (`/* !imprtant */`), and
//! `url(...)` arguments (`url(https://example.com/!imprtant.png)`) are ignored.

use oxc_css_parser::{Span, ast::Declaration};
use serde_json::{Map, Value};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, decl_value_raw, is_standard_syntax_value, options_matches,
        parse_option_matchers,
    },
};

/// Formats the diagnostic message matching Stylelint's `messages.rejected`.
fn unknown_annotation_diagnostic(annotation: &str) -> String {
    format!("Unknown annotation \"{annotation}\"")
}

crate::define_stx_lint! {
    /// Flag annotation words other than `!important` in declaration values.
    ///
    /// Enable with `true`. The secondary `ignoreAnnotations` list accepts names
    /// or slash-delimited regexes matched without `!`. Quoted text, comments,
    /// `url(...)` arguments, and nonstandard declaration syntax are not checked.
    /// `!important` is accepted in any case.
    ///
    /// ```css
    /// a { color: green !imprtant; } /* reported */
    /// a { color: green !important; } /* accepted */
    /// ```
    "annotation-no-unknown", correctness, recommended = true,
    short_description = "Disallow unknown annotations within declarations.";
    #[derive(Debug, Default, Clone)]
    pub struct AnnotationNoUnknown {
        /// Compiled matchers from `{ "ignoreAnnotations": [...] }`, tested
        /// against annotation names stripped of their leading `!`.
        ignore_annotations: Vec<OptionMatcher>,
    }
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
        if !ctx.is_standard_declaration(decl) {
            return;
        }
        if decl.important.is_some() {
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

/// Returns `true` if `b` is a token separator or quote in `postcss-value-parser`.
#[inline]
const fn is_word_delimiter(b: u8) -> bool {
    matches!(
        b,
        b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' | b',' | b':' | b'/' | b'(' | b')' | b'"' | b'\''
    )
}

/// Advances past a single- or double-quoted CSS string starting at `bytes[i]`,
/// respecting `\` escapes.
#[inline]
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

/// Advances past a `/* ... */` block comment starting at `bytes[i]` (`i` points
/// to the opening `/`).
#[inline]
fn skip_block_comment(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    i += 2;
    while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
        i += 1;
    }
    (i + 2).min(len)
}

/// Advances past the parenthesized argument list of `url(...)` starting at the
/// opening `(` at `bytes[i]`.
///
/// `postcss-value-parser` parses unquoted `url(...)` contents as a single word
/// token rather than splitting on `:` or `/`, so `!` inside
/// `url(https://example.com/!important.png)` is never a standalone annotation.
#[inline]
fn skip_url_args(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    i += 1; // skip '('
    while i < len && bytes[i] != b')' {
        match bytes[i] {
            b'\\' => i = (i + 2).min(len),
            b'"' | b'\'' => i = skip_quoted(bytes, i),
            _ => i += 1,
        }
    }
    if i < len && bytes[i] == b')' {
        i += 1;
    }
    i
}

/// Scans a raw CSS declaration value in the same manner as `postcss-value-parser`,
/// invoking `callback(word, start_offset, end_offset)` for every `word` token
/// that starts with `'!'`.
fn for_each_annotation_word<'a>(raw: &'a str, mut callback: impl FnMut(&'a str, u32, u32)) {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        match bytes[i] {
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i = skip_block_comment(bytes, i);
            }
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' | b',' | b':' | b'/' | b'(' | b')' => {
                i += 1;
            }
            b'"' | b'\'' => {
                i = skip_quoted(bytes, i);
            }
            _ => {
                let word_start = i;
                while i < len && !is_word_delimiter(bytes[i]) {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(len);
                    } else {
                        i += 1;
                    }
                }
                let word = &raw[word_start..i];
                if word.eq_ignore_ascii_case("url") && i < len && bytes[i] == b'(' {
                    i = skip_url_args(bytes, i);
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
    use crate::tester::{Tester, reject, reject_warnings};

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

    #[test]
    fn default_config_and_edge_cases() {
        let pass = [
            "a { color: green; }",
            "a { color: green !important; }",
            "a { color: green /* !imprtant */; }",
            "a { content: \"!imprtant\"; }",
            "a { content: '!imprtant'; }",
            "a { content: \"\\\"\"; }",
            "a { background: url(http://example.com/!imprtant.png); }",
            "a { background: url(\"http://example.com/!imprtant.png\"); }",
            "a { background: url('http://example.com/!imprtant.png'); }",
            "a { color: var(--!imprtant); }",
            "a { color: green!imprtant }",
        ];

        let fail = [
            reject(
                "a { color: green !imprtant }",
                None,
                unknown_annotation_diagnostic("!imprtant"),
                (1, 18, 1, 27),
            ),
            reject(
                "a { color: green/*comment*/!imprtant }",
                None,
                unknown_annotation_diagnostic("!imprtant"),
                (1, 28, 1, 37),
            ),
            reject(
                "a { color: var(--foo, !imprtant) }",
                None,
                unknown_annotation_diagnostic("!imprtant"),
                (1, 23, 1, 32),
            ),
            reject_warnings(
                "a { color: green !--foo !--bar }",
                None,
                vec![
                    (unknown_annotation_diagnostic("!--foo"), (1, 18, 1, 24)),
                    (unknown_annotation_diagnostic("!--bar"), (1, 25, 1, 31)),
                ],
            ),
        ];

        Tester::new(AnnotationNoUnknown::NAME, json!(true), pass, fail).test();
    }
}
