use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rules::Rule;
use oxc_css_parser::token::Comment;
use serde_json::Value;

pub struct CommentNoEmpty {
    pub severity: Severity,
}

pub const REJECTED_MSG: &str = "Empty comment";

impl CommentNoEmpty {
    pub fn from_config(primary: &Value, severity: Severity) -> Option<Self> {
        if primary.as_bool() != Some(true) {
            return None;
        }
        Some(Self { severity })
    }
}

impl Rule for CommentNoEmpty {
    fn name(&self) -> &'static str {
        "comment-no-empty"
    }

    fn check_comment<'a>(&self, comment: &'a Comment<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(comment.span);

        if !(raw.starts_with("/*") && raw.ends_with("*/") && raw.len() >= 4) {
            return;
        }

        let text = raw[2..raw.len() - 2].trim();
        if !text.is_empty() {
            return;
        }

        ctx.report(self.name(), comment.span, REJECTED_MSG.to_string());
    }
}
