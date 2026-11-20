use oxc_css_parser::Span;
use oxc_css_parser::ast::AtRule;
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};

use crate::context::LintContext;
use crate::rules::Rule;

#[derive(Debug)]
pub struct CustomMediaPattern {
    regex: Regex,
    raw_pattern: String,
    param_regex: Regex,
}

impl CustomMediaPattern {
    pub fn from_config(primary: &Value, _secondary: Option<&Map<String, Value>>) -> Option<Self> {
        let raw_pattern = primary.as_str()?.to_string();
        let regex = compile_pattern(&raw_pattern)?;
        let param_regex = Regex::new(r"^--(\S+)\b").ok()?;
        Some(Self {
            regex,
            raw_pattern,
            param_regex,
        })
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

pub fn expected_message(media_name: &str, pattern: &str) -> String {
    format!("Expected \"{media_name}\" to match pattern \"{pattern}\"")
}

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

impl Rule for CustomMediaPattern {
    fn name(&self) -> &'static str {
        "custom-media-pattern"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let Some((name, prelude, prelude_start)) =
            extract_at_rule_name_and_prelude(ctx.source, at_rule)
        else {
            return;
        };

        if !name.eq_ignore_ascii_case("custom-media") {
            return;
        }

        let Some(caps) = self.param_regex.captures(prelude) else {
            return;
        };
        let Some(full_name_match) = caps.get(0) else {
            return;
        };
        let Some(custom_media_name_match) = caps.get(1) else {
            return;
        };

        let full_name = full_name_match.as_str();
        let custom_media_name = custom_media_name_match.as_str();

        if self.regex.is_match(custom_media_name) {
            return;
        }

        let start = prelude_start;
        let end = start + full_name.len() as u32;

        ctx.report(
            self.name(),
            Span { start, end },
            expected_message(full_name, &self.raw_pattern),
        );
    }
}
