use std::collections::HashSet;

use oxc_css_parser::Span;
use oxc_css_parser::ast::Stylesheet;
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;

#[derive(Debug)]
enum IgnoreMatcher {
    Exact(String),
    Regex(Regex),
}

impl IgnoreMatcher {
    fn from_str(s: &str) -> Option<Self> {
        if s.starts_with('/') && s.len() >= 2 {
            if let Some(last_slash) = s.rfind('/') {
                if last_slash > 0 {
                    let pattern = &s[1..last_slash];
                    let flags = &s[last_slash + 1..];
                    let mut builder = RegexBuilder::new(pattern);
                    if flags.contains('i') {
                        builder.case_insensitive(true);
                    }
                    return builder.build().ok().map(IgnoreMatcher::Regex);
                }
            }
        }
        Some(IgnoreMatcher::Exact(s.to_string()))
    }

    fn matches(&self, prop: &str) -> bool {
        match self {
            IgnoreMatcher::Exact(s) => s == prop,
            IgnoreMatcher::Regex(re) => re.is_match(prop),
        }
    }
}

#[derive(Debug, Default)]
pub struct DeclarationBlockNoDuplicateCustomProperties {
    ignore_properties: Vec<IgnoreMatcher>,
}

impl DeclarationBlockNoDuplicateCustomProperties {
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Option<Self> {
        if primary.as_bool() == Some(false) {
            return None;
        }

        let mut ignore_properties = Vec::new();
        if let Some(sec) = secondary {
            if let Some(ig) = sec.get("ignoreProperties") {
                match ig {
                    Value::String(s) => {
                        if let Some(m) = IgnoreMatcher::from_str(s) {
                            ignore_properties.push(m);
                        }
                    }
                    Value::Array(arr) => {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                if let Some(m) = IgnoreMatcher::from_str(s) {
                                    ignore_properties.push(m);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Some(Self { ignore_properties })
    }

    fn is_ignored(&self, prop: &str) -> bool {
        self.ignore_properties.iter().any(|m| m.matches(prop))
    }
}

pub fn rejected_message(property: &str) -> String {
    format!("Duplicate custom property \"{property}\"")
}

fn is_standard_syntax_property(prop: &str) -> bool {
    !prop.starts_with('$')
        && !prop.starts_with('@')
        && !prop.contains("#{")
        && !prop.contains("@{")
        && !prop.contains("$(")
}

fn skip_braces(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    let mut depth = 0usize;
    while i < len {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
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

impl Rule for DeclarationBlockNoDuplicateCustomProperties {
    fn name(&self) -> &'static str {
        "declaration-block-no-duplicate-custom-properties"
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let source = ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();

        let mut stack: Vec<HashSet<String>> = Vec::new();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            if b == b'{' {
                stack.push(HashSet::new());
                i += 1;
                continue;
            }

            if b == b'}' {
                stack.pop();
                i += 1;
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

            let stmt_start = i;
            let mut paren_depth = 0usize;
            let mut first_colon: Option<usize> = None;

            while i < len {
                let c = bytes[i];
                if c == b'\\' {
                    i += 2;
                    continue;
                }
                if c == b'"' || c == b'\'' {
                    let quote = c;
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
                if c == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                    i += 2;
                    while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                        i += 1;
                    }
                    i = (i + 2).min(len);
                    continue;
                }
                if c == b'{' && i > stmt_start && (bytes[i - 1] == b'#' || bytes[i - 1] == b'@') {
                    i = skip_braces(bytes, i);
                    continue;
                }
                if c == b'(' {
                    paren_depth += 1;
                } else if c == b')' {
                    paren_depth = paren_depth.saturating_sub(1);
                } else if paren_depth == 0 {
                    if c == b':' && first_colon.is_none() {
                        first_colon = Some(i);
                    } else if c == b'{' || c == b';' || c == b'}' {
                        break;
                    }
                }
                i += 1;
            }

            if i < len && bytes[i] == b'{' {
                stack.push(HashSet::new());
                i += 1;
            } else {
                let stmt_slice = &source[stmt_start..i];
                if !stmt_slice.starts_with('@') {
                    if let Some(colon_pos) = first_colon {
                        let prop = source[stmt_start..colon_pos].trim();
                        if prop.starts_with("--")
                            && prop.len() > 2
                            && is_standard_syntax_property(prop)
                            && !self.is_ignored(prop)
                        {
                            if let Some(current_set) = stack.last_mut() {
                                if !current_set.insert(prop.to_string()) {
                                    let start = stmt_start as u32;
                                    let end = start + prop.len() as u32;
                                    ctx.report(
                                        self.name(),
                                        Span { start, end },
                                        rejected_message(prop),
                                    );
                                }
                            }
                        }
                    }
                }
                if i < len && bytes[i] == b';' {
                    i += 1;
                }
            }
        }
    }
}
