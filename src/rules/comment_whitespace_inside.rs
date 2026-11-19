use crate::{context::LintContext, fix::Fix, rules::Rule};
use oxc_css_parser::{Span, token::Comment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentWhitespaceMode {
    Always,
    Never,
}

pub struct CommentWhitespaceInside {
    pub mode: CommentWhitespaceMode,
}

impl Default for CommentWhitespaceInside {
    fn default() -> Self {
        Self {
            mode: CommentWhitespaceMode::Always,
        }
    }
}

impl Rule for CommentWhitespaceInside {
    fn name(&self) -> &'static str {
        "comment-whitespace-inside"
    }

    fn check_comment<'a>(&self, comment: &'a Comment<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(comment.span);
        if !raw.starts_with("/*") || !raw.ends_with("*/") || raw.len() < 4 {
            return;
        }
        let inner = &raw[2..raw.len() - 2];
        if inner.is_empty() || inner.bytes().all(|b| b == b'*') {
            return;
        }

        let lead_stars = inner.bytes().take_while(|&b| b == b'*').count();
        let after_open = &inner[lead_stars..];
        let open_pos = comment.span.start + 2 + lead_stars as u32;

        let trail_stars = inner.bytes().rev().take_while(|&b| b == b'*').count();
        let before_close = &inner[..inner.len() - trail_stars];
        let close_pos = comment.span.end - 2 - trail_stars as u32;

        match self.mode {
            CommentWhitespaceMode::Always => {
                if !after_open.starts_with(char::is_whitespace) {
                    let sp = Span {
                        start: open_pos,
                        end: open_pos,
                    };
                    ctx.report_with_fix(
                        self.name(),
                        sp,
                        "Expected whitespace after \"/*\"".to_string(),
                        Some(Fix::single(sp, " ".to_string())),
                    );
                }
                if !before_close.ends_with(char::is_whitespace) {
                    let sp = Span {
                        start: close_pos,
                        end: close_pos,
                    };
                    ctx.report_with_fix(
                        self.name(),
                        sp,
                        "Expected whitespace before \"*/\"".to_string(),
                        Some(Fix::single(sp, " ".to_string())),
                    );
                }
            }
            CommentWhitespaceMode::Never => {
                let ws_lead = after_open
                    .bytes()
                    .take_while(|b| b.is_ascii_whitespace())
                    .count();
                if ws_lead > 0 {
                    let sp = Span {
                        start: open_pos,
                        end: open_pos + ws_lead as u32,
                    };
                    ctx.report_with_fix(
                        self.name(),
                        sp,
                        "Unexpected whitespace after \"/*\"".to_string(),
                        Some(Fix::single(sp, String::new())),
                    );
                }
                let ws_trail = before_close
                    .bytes()
                    .rev()
                    .take_while(|b| b.is_ascii_whitespace())
                    .count();
                if ws_trail > 0 && open_pos + (ws_lead as u32) < close_pos {
                    let sp = Span {
                        start: close_pos - ws_trail as u32,
                        end: close_pos,
                    };
                    ctx.report_with_fix(
                        self.name(),
                        sp,
                        "Unexpected whitespace before \"*/\"".to_string(),
                        Some(Fix::single(sp, String::new())),
                    );
                }
            }
        }
    }
}
