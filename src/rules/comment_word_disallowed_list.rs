use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rules::Rule;
use oxc_css_parser::Span;
use oxc_css_parser::token::Comment;
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Clone)]
enum WordPattern {
    String { raw: String },
    Regex { raw: String, regex: Regex },
}

impl WordPattern {
    fn parse(s: &str) -> Option<Self> {
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
                    return Some(Self::Regex {
                        raw: s.to_string(),
                        regex: re,
                    });
                }
            }
        }
        Some(Self::String { raw: s.to_string() })
    }

    fn raw(&self) -> &str {
        match self {
            Self::String { raw } | Self::Regex { raw, .. } => raw,
        }
    }
}

pub struct CommentWordDisallowedList {
    patterns: Vec<WordPattern>,
    pub severity: Severity,
}

pub fn rejected_msg(pattern: &str) -> String {
    format!("Disallowed word matching pattern \"{pattern}\"")
}

impl CommentWordDisallowedList {
    pub fn from_config(primary: &Value, severity: Severity) -> Option<Self> {
        let mut patterns = Vec::new();
        match primary {
            Value::String(s) => {
                if let Some(p) = WordPattern::parse(s) {
                    patterns.push(p);
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        if let Some(p) = WordPattern::parse(s) {
                            patterns.push(p);
                        }
                    } else if let Some(inner) = item.as_array() {
                        for sub in inner {
                            if let Some(s) = sub.as_str() {
                                if let Some(p) = WordPattern::parse(s) {
                                    patterns.push(p);
                                }
                            }
                        }
                    }
                }
            }
            _ => return None,
        }

        if patterns.is_empty() {
            return None;
        }

        Some(Self { patterns, severity })
    }

    fn find_match<'a>(&'a self, text: &str) -> Option<(&'a str, usize, usize)> {
        // 1. matchesStringOrRegExp(text, primary)
        for pat in &self.patterns {
            match pat {
                WordPattern::Regex { raw, regex } => {
                    if let Some(m) = regex.find(text) {
                        return Some((raw.as_str(), m.start(), m.end()));
                    }
                }
                WordPattern::String { raw } => {
                    if text == raw {
                        return Some((raw.as_str(), 0, text.len()));
                    }
                }
            }
        }

        // 2. containsString(text, primary)
        for pat in &self.patterns {
            if let WordPattern::String { raw } = pat {
                if !raw.is_empty() {
                    if let Some(pos) = text.find(raw.as_str()) {
                        return Some((raw.as_str(), pos, pos + raw.len()));
                    }
                }
            }
        }

        None
    }
}

impl Rule for CommentWordDisallowedList {
    fn name(&self) -> &'static str {
        "comment-word-disallowed-list"
    }

    fn check_comment<'a>(&self, comment: &'a Comment, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(comment.span);

        // Return early if sourcemap
        if raw.starts_with("/*# ") {
            return;
        }

        let (text, text_offset) = extract_comment_text_with_offset(raw);

        if let Some((pattern_str, match_start, match_end)) = self.find_match(text) {
            let start = comment.span.start + (text_offset + match_start) as u32;
            let end = comment.span.start + (text_offset + match_end) as u32;
            ctx.report(self.name(), Span { start, end }, rejected_msg(pattern_str));
        }
    }
}

fn extract_comment_text_with_offset(raw: &str) -> (&str, usize) {
    if raw.len() >= 4 && raw.starts_with("/*") && raw.ends_with("*/") {
        let inner = &raw[2..raw.len() - 2];
        let trimmed_start = inner.trim_start();
        let lead = inner.len() - trimmed_start.len();
        let text = trimmed_start.trim_end();
        (text, 2 + lead)
    } else if let Some(inner) = raw.strip_prefix("//") {
        let trimmed_start = inner.trim_start();
        let lead = inner.len() - trimmed_start.len();
        let text = trimmed_start.trim_end();
        (text, 2 + lead)
    } else {
        let trimmed_start = raw.trim_start();
        let lead = raw.len() - trimmed_start.len();
        let text = trimmed_start.trim_end();
        (text, lead)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject};

    const RULE_NAME: &str = "comment-word-disallowed-list";
    const BAD_WORD_MSG: &str = "Disallowed word matching pattern \"bad-word\"";
    const TODO_MSG: &str = "Disallowed word matching pattern \"/^TODO:/\"";

    #[test]
    fn single_string() {
        let pass = ["/* comment */", "/*# bad-word */"];
        let fail = [
            reject(
                "/* Comment with bad-word  */",
                None,
                BAD_WORD_MSG,
                (1, 17, 1, 25),
            ),
            reject("/* bad-word */", None, BAD_WORD_MSG, (1, 4, 1, 12)),
            reject("/*** bad-word ***/", None, BAD_WORD_MSG, (1, 6, 1, 14)),
            reject("/*! bad-word */", None, BAD_WORD_MSG, (1, 5, 1, 13)),
            reject("/** bad-word **/", None, BAD_WORD_MSG, (1, 5, 1, 13)),
        ];

        Tester::new(RULE_NAME, json!("bad-word"), pass, fail).test();
    }

    #[test]
    fn array_patterns() {
        let pass = [
            "/* comment */",
            "/* comment comment */",
            "/* comment\ncomment */",
            "/* comment\n\ncomment */",
            "/** comment */",
            "/**** comment ***/",
            "/*\ncomment\n*/",
            "/*\tcomment   */",
            "/*! copyright */",
            "/*# sourcemap */",
            "/*# sourcemap bad-word */",
            "a { color: pink; /* comment */\ntop: 0; }",
            "a {} /* comment */",
            "/* todo */",
            "/* todo: */",
            "/* todo: comment */",
            "/* tOdO: comment */",
            "/* Todo: comment */",
            "/*! Todo: comment */",
            "/*# Todo: comment */",
            "/** TODO: comment **/",
            "/*** TODO: comment ***/",
        ];
        let fail = [
            reject("/* TODO: */", None, TODO_MSG, (1, 4, 1, 9)),
            reject("/* TODO: comment */", None, TODO_MSG, (1, 4, 1, 9)),
            reject(
                "/* TODO: comment\n next line */",
                None,
                TODO_MSG,
                (1, 4, 1, 9),
            ),
            reject(
                "/* TODO: comment\r\n next line */",
                None,
                TODO_MSG,
                (1, 4, 1, 9),
            ),
            reject(
                "/* TODO: comment\n\n next line */",
                None,
                TODO_MSG,
                (1, 4, 1, 9),
            ),
            reject("/*\n TODO: comment */", None, TODO_MSG, (2, 2, 2, 7)),
            reject("/*\r\n TODO: comment */", None, TODO_MSG, (2, 2, 2, 7)),
            reject("/*\n\n TODO: comment */", None, TODO_MSG, (3, 2, 3, 7)),
            reject("/*\r\n\r\n TODO: comment */", None, TODO_MSG, (3, 2, 3, 7)),
            reject("/* bad-word */", None, BAD_WORD_MSG, (1, 4, 1, 12)),
            reject(
                "/* Comment with bad-word */",
                None,
                BAD_WORD_MSG,
                (1, 17, 1, 25),
            ),
            reject(
                "/*! copyright bad-word */",
                None,
                BAD_WORD_MSG,
                (1, 15, 1, 23),
            ),
            reject("/** bad-word **/", None, BAD_WORD_MSG, (1, 5, 1, 13)),
            reject("/*** bad-word ***/", None, BAD_WORD_MSG, (1, 6, 1, 14)),
        ];

        Tester::new(RULE_NAME, json!(["/^TODO:/", "bad-word"]), pass, fail).test();
    }

    #[test]
    fn scss() {
        let pass = ["// comment", "// todo", "// todo:", "// Todo:", "// tOdO:"];
        let fail = [
            reject("// TODO:", None, TODO_MSG, (1, 4, 1, 9)),
            reject("// TODO: comment", None, TODO_MSG, (1, 4, 1, 9)),
            reject("// bad-word", None, BAD_WORD_MSG, (1, 4, 1, 12)),
        ];

        Tester::new(RULE_NAME, json!(["/^TODO:/", "bad-word"]), pass, fail)
            .with_custom_syntax("postcss-scss")
            .test();
    }

    #[test]
    fn less() {
        let pass = ["// comment", "// todo:", "// Todo:", "// tOdO:"];
        let fail = [
            reject("// TODO:", None, TODO_MSG, (1, 4, 1, 9)),
            reject("// TODO: comment", None, TODO_MSG, (1, 4, 1, 9)),
            reject("// bad-word", None, BAD_WORD_MSG, (1, 4, 1, 12)),
        ];

        Tester::new(RULE_NAME, json!(["/^TODO:/", "bad-word"]), pass, fail)
            .with_custom_syntax("postcss-less")
            .test();
    }

    #[test]
    fn regex_literal_array() {
        let pass = ["/* comment */"];
        let fail = [reject("/* TODO: */", None, TODO_MSG, (1, 4, 1, 9))];

        Tester::new(RULE_NAME, json!(["/^TODO:/", "bad-word"]), pass, fail).test();
    }
}
