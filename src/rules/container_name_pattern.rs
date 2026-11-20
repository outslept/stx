use oxc_css_parser::Span;
use oxc_css_parser::ast::{AtRule, Declaration};
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;
use crate::utils::ast::decl_value_raw;

#[derive(Debug)]
pub struct ContainerNamePattern {
    regex: Regex,
    raw_pattern: String,
}

impl ContainerNamePattern {
    pub fn from_config(primary: &Value, _secondary: Option<&Map<String, Value>>) -> Option<Self> {
        let raw_pattern = primary.as_str()?.to_string();
        let regex = compile_pattern(&raw_pattern)?;
        Some(Self { regex, raw_pattern })
    }
}

fn compile_pattern(raw: &str) -> Option<Regex> {
    if raw.starts_with('/') && raw.len() >= 2 {
        if let Some(last_slash) = raw.rfind('/') {
            if last_slash > 0 {
                let pattern = &raw[1..last_slash];
                let flags = &raw[last_slash + 1..];
                let mut builder = RegexBuilder::new(pattern);
                if flags.contains('i') {
                    builder.case_insensitive(true);
                }
                return builder.build().ok();
            }
        }
    }
    Regex::new(raw).ok()
}

pub fn expected_message(container_name: &str, pattern: &str) -> String {
    format!("Expected \"{container_name}\" to match pattern \"{pattern}\"")
}

const CSS_WIDE_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "unset",
    "revert",
    "revert-layer",
    "none",
];

const AT_CONTAINER_KEYWORDS: &[&str] = &["and", "or", "none", "not"];

#[derive(Debug, PartialEq, Eq)]
enum ValueToken<'a> {
    Word(&'a str, usize),
    Slash,
}

/// Tokenizes top-level words and slashes in the same way `postcss-value-parser` walks top-level
/// nodes when `if (!isValueWord(node)) return false;` is used (skipping functions/parentheses).
fn tokenize_top_level_words(s: &str) -> Vec<ValueToken<'_>> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < len {
        let b = bytes[i];

        // Whitespace
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Comments
        if b == b'/' && i + 1 < len {
            if bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                continue;
            } else if bytes[i + 1] == b'/' {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
        }

        // Strings
        if b == b'"' || b == b'\'' {
            let quote = b;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Div separators: `/`, `,`, `:`
        if b == b'/' {
            tokens.push(ValueToken::Slash);
            i += 1;
            continue;
        }
        if b == b',' || b == b':' {
            i += 1;
            continue;
        }

        // Standalone parentheses `(...)`
        if b == b'(' {
            i = skip_parens(bytes, i);
            continue;
        }

        if b == b')' {
            i += 1;
            continue;
        }

        // Word or function call `word(...)`
        let start = i;
        while i < len {
            let c = bytes[i];
            if c == b'\\' && i + 1 < len {
                i += 2;
                continue;
            }
            if c.is_ascii_whitespace()
                || c == b'/'
                || c == b','
                || c == b':'
                || c == b'('
                || c == b')'
                || c == b'"'
                || c == b'\''
            {
                break;
            }
            i += 1;
        }

        // If immediately followed by `(`, it is a function node in postcss-value-parser, not a word
        if i < len && bytes[i] == b'(' {
            i = skip_parens(bytes, i);
            continue;
        }

        if i > start {
            tokens.push(ValueToken::Word(&s[start..i], start));
        }
    }

    tokens
}

fn skip_parens(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    let mut depth = 0usize;
    while i < len {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        if b == b'"' || b == b'\'' {
            let quote = b;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                i += 1;
                break;
            }
        }
        i += 1;
    }
    i.min(len)
}

fn is_standard_syntax_property(prop: &str) -> bool {
    !prop.starts_with('$')
        && !prop.starts_with('@')
        && !prop.contains("#{")
        && !prop.contains("@{")
        && !prop.contains("$(")
}

fn is_standard_syntax_value(val: &str) -> bool {
    !val.starts_with('$')
        && !val.starts_with('@')
        && !val.contains("#{")
        && !val.contains("@{")
        && !val.contains("$(")
}

/// Extracts the at-rule name and (prelude_slice, prelude_start_offset) from the raw source.
fn extract_at_rule_name_and_prelude<'a>(
    source: &'a str,
    at_rule: &AtRule<'a>,
) -> Option<(&'a str, &'a str, u32)> {
    let raw = source.get(at_rule.span.start as usize..at_rule.span.end as usize)?;
    if !raw.starts_with('@') {
        return None;
    }
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut i = 1;
    while i < len {
        let b = bytes[i];
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
            i += 1;
        } else {
            break;
        }
    }
    let name = &raw[1..i];

    // Find end of prelude (top-level `{` or `;` or end of span)
    let prelude_start = i;
    let mut j = i;
    let mut paren_depth = 0usize;
    while j < len {
        let b = bytes[j];
        if b == b'\\' {
            j += 2;
            continue;
        }
        if b == b'"' || b == b'\'' {
            let quote = b;
            j += 1;
            while j < len {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == quote {
                    j += 1;
                    break;
                }
                j += 1;
            }
            continue;
        }
        if b == b'/' && j + 1 < len && bytes[j + 1] == b'*' {
            j += 2;
            while j + 1 < len && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
                j += 1;
            }
            j = (j + 2).min(len);
            continue;
        }
        if b == b'(' {
            paren_depth += 1;
        } else if b == b')' {
            paren_depth = paren_depth.saturating_sub(1);
        } else if paren_depth == 0 && (b == b'{' || b == b';') {
            break;
        }
        j += 1;
    }

    let prelude_raw = &raw[prelude_start..j.min(len)];
    let trimmed_start = prelude_raw.len() - prelude_raw.trim_start().len();
    let prelude_trimmed = prelude_raw.trim();
    let abs_start = at_rule.span.start + (prelude_start + trimmed_start) as u32;

    Some((name, prelude_trimmed, abs_start))
}

impl Rule for ContainerNamePattern {
    fn name(&self) -> &'static str {
        "container-name-pattern"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let prop = ctx
            .slice_range(decl.span.start, decl.colon_span.start)
            .trim();
        if !is_standard_syntax_property(prop) {
            return;
        }
        if !prop.eq_ignore_ascii_case("container") && !prop.eq_ignore_ascii_case("container-name") {
            return;
        }

        let Some((val_str, val_span)) = decl_value_raw(ctx.source, decl) else {
            return;
        };

        for token in tokenize_top_level_words(val_str) {
            match token {
                ValueToken::Slash => break,
                ValueToken::Word(word, offset) => {
                    if CSS_WIDE_KEYWORDS
                        .iter()
                        .any(|kw| kw.eq_ignore_ascii_case(word))
                    {
                        continue;
                    }
                    if !is_standard_syntax_value(word) {
                        continue;
                    }
                    if self.regex.is_match(word) {
                        continue;
                    }
                    let start = val_span.start + offset as u32;
                    let end = start + word.len() as u32;
                    ctx.report(
                        self.name(),
                        Span { start, end },
                        expected_message(word, &self.raw_pattern),
                    );
                }
            }
        }
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let Some((name, prelude, prelude_start)) =
            extract_at_rule_name_and_prelude(ctx.source, at_rule)
        else {
            return;
        };

        if !name.eq_ignore_ascii_case("container") {
            return;
        }

        if prelude.contains("#{") || prelude.contains("@{") || prelude.contains("$(") {
            return;
        }

        for token in tokenize_top_level_words(prelude) {
            if let ValueToken::Word(word, offset) = token {
                if AT_CONTAINER_KEYWORDS
                    .iter()
                    .any(|kw| kw.eq_ignore_ascii_case(word))
                {
                    continue;
                }
                if self.regex.is_match(word) {
                    continue;
                }
                let start = prelude_start + offset as u32;
                let end = start + word.len() as u32;
                ctx.report(
                    self.name(),
                    Span { start, end },
                    expected_message(word, &self.raw_pattern),
                );
            }
        }
    }
}
