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

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    const RULE_NAME: &str = "no-empty-source";
    const REJECTED: &str = "Empty source";

    #[test]
    fn default_config() {
        let pass = [
            ".class {}",
            "   .class {}   ",
            "/* comment */",
            "\n.class {}",
            "\r\n.class {}",
            "/* stylelint-disable */",
            "/* stylelint-disable no-empty-source */",
        ];
        let fail = [
            reject("", None, REJECTED, (1, 1, 1, 2)),
            reject("   ", None, REJECTED, (1, 1, 1, 5)),
            reject("\t", None, REJECTED, (1, 1, 1, 3)),
            reject("\n", None, REJECTED, (1, 1, 2, 2)),
            reject("\r\n", None, REJECTED, (1, 1, 2, 2)),
            reject("\n\n\n", None, REJECTED, (1, 1, 4, 2)),
            reject("\r\n\r\n\r\n", None, REJECTED, (1, 1, 4, 2)),
            reject("  \n  ", None, REJECTED, (1, 1, 2, 4)),
            reject("  \r\n  ", None, REJECTED, (1, 1, 2, 4)),
            reject("/* stylelint-disable foo */", None, REJECTED, (1, 1, 1, 29)),
        ];

        Tester::new(RULE_NAME, json!([true]), pass, fail).test();
    }

    #[test]
    #[ignore = "postcss-html not supported"]
    fn html_syntax() {
        let fail = [reject("<style>\n</style>", None, REJECTED, (2, 1, 2, 2))];

        Tester::new(RULE_NAME, json!([true]), ["<html></html>"], fail)
            .with_custom_syntax("postcss-html")
            .test();
    }

    #[test]
    #[ignore = "naiveCssInJs not supported"]
    fn css_in_js_syntax() {
        Tester::new(RULE_NAME, json!([true]), ["css` $var `;"], [])
            .with_custom_syntax("naiveCssInJs")
            .test();
    }
}
