use crate::context::{Fix, LintContext};
use crate::diagnostic::Severity;
use crate::rules::Rule;
use oxc_css_parser::Span;
use oxc_css_parser::token::Comment;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Always,
    Never,
}

pub struct CommentWhitespaceInside {
    pub mode: Mode,
    pub severity: Severity,
}

pub const EXPECTED_OPENING_MSG: &str = "Expected whitespace after \"/*\"";
pub const REJECTED_OPENING_MSG: &str = "Expected no whitespace after \"/*\"";
pub const EXPECTED_CLOSING_MSG: &str = "Expected whitespace before \"*/\"";
pub const REJECTED_CLOSING_MSG: &str = "Expected no whitespace before \"*/\"";

impl CommentWhitespaceInside {
    pub fn from_config(primary: &Value, severity: Severity) -> Option<Self> {
        let mode = match primary.as_str()? {
            "always" => Mode::Always,
            "never" => Mode::Never,
            _ => return None,
        };
        Some(Self { mode, severity })
    }
}

impl Rule for CommentWhitespaceInside {
    fn name(&self) -> &'static str {
        "comment-whitespace-inside"
    }

    fn check_comment<'a>(&self, comment: &'a Comment<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(comment.span);

        if !(raw.starts_with("/*") && raw.ends_with("*/") && raw.len() >= 4) {
            return;
        }

        let bytes = raw.as_bytes();

        // Return early if sourcemap or copyright comment: /^\/\*[#!]\s/
        if bytes.len() >= 4
            && (bytes[2] == b'#' || bytes[2] == b'!')
            && bytes[3].is_ascii_whitespace()
        {
            return;
        }

        // Opener: /^\/\*+/
        let opener_stars = raw[1..raw.len() - 1]
            .bytes()
            .take_while(|&b| b == b'*')
            .count();
        let opener_len = 1 + opener_stars;

        // Closer: /\*+\/$/
        let closer_stars = raw[opener_len..raw.len() - 1]
            .bytes()
            .rev()
            .take_while(|&b| b == b'*')
            .count();
        let closer_len = 1 + closer_stars;
        let closer_start = raw.len() - closer_len;

        // Left whitespace after opener
        let left_word_len = raw[opener_len..closer_start]
            .bytes()
            .take_while(|b| b.is_ascii_whitespace())
            .count();

        // Right whitespace before closer
        let right_word_len = raw[opener_len..closer_start]
            .bytes()
            .rev()
            .take_while(|b| b.is_ascii_whitespace())
            .count();

        let base = comment.span.start;

        match self.mode {
            Mode::Never => {
                if left_word_len > 0 {
                    let start = base + opener_len as u32;
                    let end = start + left_word_len as u32;
                    let span = Span { start, end };
                    ctx.report_with_fix(
                        self.name(),
                        span,
                        REJECTED_OPENING_MSG.to_string(),
                        Some(Fix::single(span, "")),
                    );
                }
                if right_word_len > 0 {
                    let end = base + closer_start as u32;
                    let start = end - right_word_len as u32;
                    let span = Span { start, end };
                    ctx.report_with_fix(
                        self.name(),
                        span,
                        REJECTED_CLOSING_MSG.to_string(),
                        Some(Fix::single(span, "")),
                    );
                }
            }
            Mode::Always => {
                if left_word_len == 0 {
                    let pos = base + opener_len as u32;
                    let warn_span = Span {
                        start: pos,
                        end: (pos + 1).min(comment.span.end),
                    };
                    let fix_span = Span {
                        start: pos,
                        end: pos,
                    };
                    ctx.report_with_fix(
                        self.name(),
                        warn_span,
                        EXPECTED_OPENING_MSG.to_string(),
                        Some(Fix::single(fix_span, " ")),
                    );
                }
                if right_word_len == 0 {
                    let warn_start = base + closer_start.saturating_sub(1) as u32;
                    let warn_span = Span {
                        start: warn_start,
                        end: (warn_start + 1).min(comment.span.end),
                    };
                    let fix_pos = base + closer_start as u32;
                    let fix_span = Span {
                        start: fix_pos,
                        end: fix_pos,
                    };
                    ctx.report_with_fix(
                        self.name(),
                        warn_span,
                        EXPECTED_CLOSING_MSG.to_string(),
                        Some(Fix::single(fix_span, " ")),
                    );
                }
            }
        }
    }
}
