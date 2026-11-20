use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rules::Rule;
use oxc_css_parser::token::Comment;
use regex::Regex;
use serde_json::Value;

pub struct CommentPattern {
    raw_pattern: String,
    regex: Regex,
    pub severity: Severity,
}

pub fn expected_msg(pattern: &str) -> String {
    format!("Expected comment to match pattern \"{pattern}\"")
}

impl CommentPattern {
    pub fn from_config(primary: &Value, severity: Severity) -> Option<Self> {
        let s = primary.as_str()?;
        let regex = compile_pattern(s)?;
        Some(Self {
            raw_pattern: s.to_string(),
            regex,
            severity,
        })
    }
}

fn compile_pattern(s: &str) -> Option<Regex> {
    if s.starts_with('/') && s.len() >= 2 {
        if let Some(last_slash) = s[1..].rfind('/') {
            let pattern = &s[1..=last_slash];
            let flags = &s[last_slash + 2..];
            let full_pattern = if flags.contains('i') {
                format!("(?i){pattern}")
            } else {
                pattern.to_string()
            };
            if let Ok(re) = Regex::new(&full_pattern) {
                return Some(re);
            }
        }
    }
    Regex::new(s).ok()
}

impl Rule for CommentPattern {
    fn name(&self) -> &'static str {
        "comment-pattern"
    }

    fn check_comment<'a>(&self, comment: &'a Comment<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(comment.span);
        let text = extract_comment_text(raw);

        if self.regex.is_match(text) {
            return;
        }

        ctx.report(self.name(), comment.span, expected_msg(&self.raw_pattern));
    }
}

fn extract_comment_text(raw: &str) -> &str {
    if raw.len() >= 4 && raw.starts_with("/*") && raw.ends_with("*/") {
        raw[2..raw.len() - 2].trim()
    } else if let Some(rest) = raw.strip_prefix("//") {
        rest.trim()
    } else {
        raw.trim()
    }
}
