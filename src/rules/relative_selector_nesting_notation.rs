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
        if let Some(comment) = trimmed.strip_prefix("/*")
            && let Some(end) = comment.find("*/")
        {
            selector = &comment[end + 2..];
            continue;
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
        let mut prev_end = None;
        for selector in &rule.selector.selectors {
            let report_span = match prev_end.replace(selector.span.end) {
                // postcss-selector-parser starts later selectors right after the comma and
                // measures them by their trimmed text, leading comments included.
                Some(prev) => {
                    let between = ctx.slice_range(prev, selector.span.start);
                    let start = prev + between.find(',').map_or(0, |i| i + 1) as u32;
                    let text = ctx.slice_range(start, selector.span.end).trim_start();
                    Span {
                        start,
                        end: start + text.len() as u32,
                    }
                }
                None => selector.span,
            };
            let raw = ctx.slice(selector.span);
            let trimmed = raw.trim();
            let leading = strip_leading_comments(trimmed);
            let has_ampersand = trimmed.contains('&');
            let explicit_ampersand = leading.strip_prefix('&');
            // Implicit nesting applies only when `&` is followed by a combinator.
            let implicit_valid = explicit_ampersand.is_none_or(|rest| {
                !rest.starts_with(|c: char| c.is_whitespace() || "/>+~|".contains(c))
            });
            let valid = if self.explicit {
                has_ampersand && !starts_with_combinator(leading)
            } else {
                implicit_valid
            };
            if valid {
                continue;
            }
            let fix = if self.explicit {
                let at =
                    selector.span.start + (raw.len() - strip_leading_comments(raw).len()) as u32;
                Some(Fix::single(Span { start: at, end: at }, "& "))
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
                report_span,
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
    use crate::tester::{Tester, reject_message, reject_warnings};
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
            reject_message(
                "a { + a & {} }",
                Some("a { & + a & {} }"),
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
        let pass = [
            "a {}",
            "a { a {} }",
            "a { > a {} }",
            "a { &:hover {} }",
            "a { &.foo {} }",
        ];
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

    #[test]
    fn later_selectors_span_from_the_comma() {
        let msg = || "Expected explicit nesting notation".to_string();
        let fail = [
            reject_warnings(
                "a { > .foo, /* bar */ + .baz {} }",
                None,
                vec![(msg(), (1, 5, 1, 11)), (msg(), (1, 12, 1, 28))],
            ),
            reject_warnings(
                "a { > .foo,\n  + .baz {} }",
                None,
                vec![(msg(), (1, 5, 1, 11)), (msg(), (1, 12, 2, 6))],
            ),
        ];
        Tester::new(
            "relative-selector-nesting-notation",
            json!(["explicit"]),
            [],
            fail,
        )
        .test();
    }
}
