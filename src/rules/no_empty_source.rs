use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::Stylesheet};

pub struct NoEmptySource;

fn is_configuration_comment(content: &str) -> bool {
    let trimmed = content.trim();
    let base = match trimmed.split_once("--") {
        Some((before, _)) => before.trim(),
        None => trimmed,
    };
    for prefix in [
        "stylelint-disable-next-line",
        "stylelint-disable-line",
        "stylelint-disable",
        "stylelint-enable",
        "stylelint-configure",
    ] {
        if let Some(rest) = base.strip_prefix(prefix) {
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                return true;
            }
        }
    }
    false
}

impl Rule for NoEmptySource {
    fn name(&self) -> &'static str {
        "no-empty-source"
    }

    fn check_stylesheet<'a>(&self, ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        if !ast.statements.is_empty() {
            return;
        }

        let mut cursor = 0usize;
        for comment in ctx.comments {
            let c_start = comment.span.start as usize;
            let c_end = comment.span.end as usize;
            if c_start > cursor {
                let gap = &ctx.source[cursor..c_start];
                if gap.chars().any(|ch| !ch.is_whitespace()) {
                    return;
                }
            }
            if !is_configuration_comment(comment.content) {
                return;
            }
            cursor = c_end;
        }

        if cursor < ctx.source.len() {
            let tail = &ctx.source[cursor..];
            if tail.chars().any(|ch| !ch.is_whitespace()) {
                return;
            }
        }

        ctx.report(
            self.name(),
            Span {
                start: 0,
                end: ctx.source.len() as u32 + 1,
            },
            "Empty source".to_string(),
        );
    }
}
