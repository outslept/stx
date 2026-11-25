use oxc_css_parser::{Span, ast::QualifiedRule};

use crate::{
    context::{AstParent, LintContext},
    fix::Fix,
    rules::Rule,
};

pub struct RelativeSelectorNestingNotation {
    explicit: bool,
}

impl RelativeSelectorNestingNotation {
    pub fn from_config(primary: &serde_json::Value) -> Option<Self> {
        Some(Self {
            explicit: primary.as_str()? == "explicit",
        })
    }
}

fn strip_leading_comments(mut selector: &str) -> &str {
    loop {
        let trimmed = selector.trim_start();
        if let Some(comment) = trimmed.strip_prefix("/*") {
            if let Some(end) = comment.find("*/") {
                selector = &comment[end + 2..];
                continue;
            }
        }
        return trimmed;
    }
}

fn starts_with_combinator(selector: &str) -> bool {
    selector.starts_with(['>', '+', '~']) || selector.starts_with("||")
}

impl Rule for RelativeSelectorNestingNotation {
    fn name(&self) -> &'static str {
        "relative-selector-nesting-notation"
    }

    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        if !ctx
            .ancestors
            .iter()
            .any(|parent| matches!(parent, AstParent::QualifiedRule(_)))
        {
            return;
        }
        for selector in &rule.selector.selectors {
            let raw = ctx.slice(selector.span);
            let trimmed = raw.trim();
            let leading = strip_leading_comments(trimmed);
            let has_ampersand = trimmed.contains('&');
            let explicit_ampersand = leading.strip_prefix('&');
            let implicit_valid =
                explicit_ampersand.is_none_or(|rest| strip_leading_comments(rest).starts_with(':'));
            let valid = if self.explicit {
                has_ampersand && !starts_with_combinator(leading)
            } else {
                implicit_valid
            };
            if valid {
                continue;
            }
            let fix = if self.explicit && !has_ampersand {
                Some(Fix::single(
                    Span {
                        start: selector.span.start,
                        end: selector.span.start,
                    },
                    "& ",
                ))
            } else if let Some(rest) = explicit_ampersand.filter(|_| !self.explicit) {
                let amp_start = selector.span.start + (raw.len() - leading.len()) as u32;
                let whitespace = rest.len() - rest.trim_start().len();
                Some(Fix::single(
                    Span {
                        start: amp_start,
                        end: amp_start + 1 + whitespace as u32,
                    },
                    "",
                ))
            } else {
                None
            };
            ctx.report_with_fix(
                self.name(),
                selector.span,
                format!(
                    "Expected {} nesting notation",
                    if self.explicit {
                        "explicit"
                    } else {
                        "implicit"
                    }
                ),
                fix,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_message};
    use serde_json::json;

    #[test]
    fn explicit_autofix() {
        let fail = [
            reject_message(
                "a { a {} }",
                Some("a { & a {} }"),
                "Expected explicit nesting notation",
            ),
            reject_message(
                "a { > a {} }",
                Some("a { & > a {} }"),
                "Expected explicit nesting notation",
            ),
        ];
        Tester::new(
            "relative-selector-nesting-notation",
            json!(["explicit"]),
            [],
            fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn implicit_accepts_type_selectors_and_autofixes_relative_ampersands() {
        let pass = ["a {}", "a { a {} }", "a { > a {} }", "a { &:hover {} }"];
        let fail = [
            reject_message(
                "a { & a {} }",
                Some("a { a {} }"),
                "Expected implicit nesting notation",
            ),
            reject_message(
                "a { & > a {} }",
                Some("a { > a {} }"),
                "Expected implicit nesting notation",
            ),
        ];
        Tester::new(
            "relative-selector-nesting-notation",
            json!(["implicit"]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }
}
