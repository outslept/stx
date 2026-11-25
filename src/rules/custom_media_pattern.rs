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
    if raw.starts_with('/')
        && raw.len() >= 2
        && let Some(last_slash) = raw.rfind('/')
        && last_slash > 0
    {
        let pattern = &raw[1..last_slash];
        let flags = &raw[last_slash + 1..];
        let mut builder = RegexBuilder::new(pattern);
        if flags.contains('i') {
            builder.case_insensitive(true);
        }
        return builder.build().ok();
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
    let raw = source.get(at_rule.span.start as usize..)?;
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
        } else if paren_depth == 0 && (b == b'{' || b == b';' || b == b'}') {
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

impl CustomMediaPattern {
    fn check_custom_media(
        &self,
        full_name: &str,
        custom_media_name: &str,
        start: u32,
        ctx: &mut LintContext<'_>,
    ) {
        if self.regex.is_match(custom_media_name) {
            return;
        }

        let end = start + full_name.len() as u32;
        ctx.report(
            self.name(),
            Span { start, end },
            expected_message(full_name, &self.raw_pattern),
        );
    }
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

        self.check_custom_media(
            full_name_match.as_str(),
            custom_media_name_match.as_str(),
            prelude_start,
            ctx,
        );
    }

    // @custom-media with no value (`@custom-media --foo`) makes the parser reject
    // the whole stylesheet, so fall back to scanning the raw source.
    fn check_stylesheet<'a>(
        &self,
        ast: &'a oxc_css_parser::ast::Stylesheet<'a>,
        ctx: &mut LintContext<'a>,
    ) {
        if !ast.statements.is_empty() {
            return;
        }

        let src = ctx.source;
        let bytes = src.as_bytes();
        let len = bytes.len();
        let mut i = 0usize;
        while i < len {
            if bytes[i] != b'@' {
                i += 1;
                continue;
            }
            let name_start = i + 1;
            let name_end = name_start + 12;
            if name_end > len || !src[name_start..name_end].eq_ignore_ascii_case("custom-media") {
                i += 1;
                continue;
            }
            if name_end < len && is_name_char(bytes[name_end]) {
                i += 1;
                continue;
            }
            let mut j = name_end;
            while j < len && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j + 2 > len || &src[j..j + 2] != "--" {
                i = name_end;
                continue;
            }
            let dashed_start = j;
            j += 2;
            while j < len && is_name_char(bytes[j]) {
                j += 1;
            }
            let full_name = &src[dashed_start..j];
            if full_name.len() > 2 {
                self.check_custom_media(full_name, &full_name[2..], dashed_start as u32, ctx);
            }
            i = j;
        }
    }
}

#[inline]
fn is_name_char(b: u8) -> bool {
    !b.is_ascii_whitespace() && !matches!(b, b';' | b'{' | b'}' | b'(' | b')' | b',')
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject};

    const NAME: &str = "custom-media-pattern";

    fn expected(media_name: &str, pattern: &str) -> String {
        format!("Expected \"{media_name}\" to match pattern \"{pattern}\"")
    }

    #[test]
    fn custom_media_pattern_regex_foo() {
        let pattern = "/foo-.+/";
        let pass = [
            "@keyframes foofoo {}",
            "@custom-media --foo-bar (min-width: 0);",
            "@custom-media --foo-foofoo (min-width: 0);",
            "@cUsToM-mEdIa --foo-foofoo (min-width: 0);",
            "@CUSTOM-MEDIA --foo-foofoo (min-width: 0);",
        ];
        let fail = [
            reject(
                "@custom-media --foa-bar (min-width: 0);",
                None,
                expected("--foa-bar", pattern),
                (1, 15, 1, 24),
            ),
            reject(
                "@cUsToM-mEdIa --foa-bar (min-width: 0);",
                None,
                expected("--foa-bar", pattern),
                (1, 15, 1, 24),
            ),
            reject(
                "@CUSTOM-MEDIA --foa-bar (min-width: 0);",
                None,
                expected("--foa-bar", pattern),
                (1, 15, 1, 24),
            ),
            reject(
                "@custom-media --foa (min-width: 0);",
                None,
                expected("--foa", pattern),
                (1, 15, 1, 20),
            ),
        ];
        Tester::new(NAME, json!([pattern]), pass, fail).test();
    }

    #[test]
    fn custom_media_pattern_string_foo() {
        let pattern = "foo-.+";
        let pass = [
            "@keyframes foofoo {}",
            "@custom-media --foo-bar (min-width: 0);",
            "@custom-media --foo-foofoo (min-width: 0);",
        ];
        let fail = [
            reject(
                "@custom-media --foa-bar (min-width: 0);",
                None,
                expected("--foa-bar", pattern),
                (1, 15, 1, 24),
            ),
            reject(
                "@custom-media --foa (min-width: 0);",
                None,
                expected("--foa", pattern),
                (1, 15, 1, 20),
            ),
        ];
        Tester::new(NAME, json!([pattern]), pass, fail).test();
    }

    #[test]
    fn custom_media_pattern_case_sensitive() {
        let pattern = "/^[A-Z][a-z]+-[a-z][a-zA-Z]+$/";
        let pass = [
            "@custom-media --Ape-ageLess",
            "@custom-media --Purr-piratePlant",
        ];
        let fail = [
            reject(
                "@custom-media --ape-ageLess",
                None,
                expected("--ape-ageLess", pattern),
                (1, 15, 1, 28),
            ),
            reject(
                "@custom-media --Ape-AgeLess",
                None,
                expected("--Ape-AgeLess", pattern),
                (1, 15, 1, 28),
            ),
        ];
        Tester::new(NAME, json!([pattern]), pass, fail).test();
    }
}
