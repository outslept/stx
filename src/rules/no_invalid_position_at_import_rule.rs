use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::{Statement, Stylesheet};

pub struct NoInvalidPositionAtImportRule;

impl Rule for NoInvalidPositionAtImportRule {
    fn name(&self) -> &'static str {
        "no-invalid-position-at-import-rule"
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let mut seen_disallowed = false;

        for stmt in &stylesheet.statements {
            match stmt {
                Statement::AtRule(at_rule) => {
                    let name: &str = at_rule.name.name;
                    if name.eq_ignore_ascii_case("import") {
                        if seen_disallowed {
                            ctx.report(
                                self.name(),
                                at_rule.span,
                                "Unexpected invalid position @import rule".to_string(),
                            );
                        }
                    } else if name.eq_ignore_ascii_case("charset")
                        || name.eq_ignore_ascii_case("use")
                        || name.eq_ignore_ascii_case("forward")
                        || (name.eq_ignore_ascii_case("layer") && at_rule.block.is_none())
                    {
                    } else {
                        seen_disallowed = true;
                    }
                }
                _ => {
                    seen_disallowed = true;
                }
            }
        }
    }
}
