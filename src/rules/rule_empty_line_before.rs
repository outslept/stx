use oxc_css_parser::ast::{Statement, Stylesheet};

use crate::{
    config::RuleSetting,
    context::LintContext,
    rules::{
        Rule,
        comment_empty_line_before::{ChildKind, walk_siblings},
    },
    utils::{is_single_line_string, is_standard_syntax_selector, secondary_has_flag},
};

const NAME: &str = "rule-empty-line-before";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleEmptyLineMode {
    Always,
    AlwaysMultiLine,
    Never,
    NeverMultiLine,
}

pub struct RuleEmptyLineBefore {
    mode: RuleEmptyLineMode,
    ignore_after_comment: bool,
    ignore_first_nested: bool,
    ignore_inside_block: bool,
    except_after_rule: bool,
    except_after_single_line_comment: bool,
    except_first_nested: bool,
    except_inside_block_and_after_rule: bool,
    except_inside_block: bool,
}

impl RuleEmptyLineBefore {
    pub fn from_setting(setting: &RuleSetting) -> Self {
        let flag = |key, name| secondary_has_flag(setting.secondary.as_ref(), key, name);
        Self {
            mode: match setting.primary.as_str() {
                Some("always") => RuleEmptyLineMode::Always,
                Some("never") => RuleEmptyLineMode::Never,
                Some("never-multi-line") => RuleEmptyLineMode::NeverMultiLine,
                _ => RuleEmptyLineMode::AlwaysMultiLine,
            },
            ignore_after_comment: flag("ignore", "after-comment"),
            ignore_first_nested: flag("ignore", "first-nested"),
            ignore_inside_block: flag("ignore", "inside-block"),
            except_after_rule: flag("except", "after-rule"),
            except_after_single_line_comment: flag("except", "after-single-line-comment"),
            except_first_nested: flag("except", "first-nested"),
            except_inside_block_and_after_rule: flag("except", "inside-block-and-after-rule"),
            except_inside_block: flag("except", "inside-block"),
        }
    }
}

impl Rule for RuleEmptyLineBefore {
    fn name(&self) -> &'static str {
        NAME
    }

    fn check_stylesheet<'a>(&self, ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        walk_siblings(ast, ctx, |siblings, ctx| {
            let ChildKind::Statement(stmt) = siblings.current().kind else {
                return;
            };
            let selector = match stmt {
                Statement::QualifiedRule(rule) => ctx.slice(rule.selector.span),
                Statement::UnknownQualifiedRule(rule) => ctx.slice(rule.prelude.span),
                Statement::LessConditionalQualifiedRule(rule) => {
                    ctx.slice_range(rule.span.start, rule.block.span.start)
                }
                Statement::KeyframeBlock(block) => {
                    ctx.slice_range(block.span.start, block.block.span.start)
                }
                _ => return,
            };
            let nested = siblings.parent.is_some();
            let first_nested = (self.ignore_first_nested || self.except_first_nested)
                && siblings.first_nested(ctx);
            if (self.ignore_inside_block && nested)
                || (self.ignore_first_nested && first_nested)
                || (self.ignore_after_comment
                    && siblings
                        .previous()
                        .is_some_and(|prev| prev.kind.is_comment()))
                || !is_standard_syntax_selector(selector)
                || (matches!(
                    self.mode,
                    RuleEmptyLineMode::AlwaysMultiLine | RuleEmptyLineMode::NeverMultiLine
                ) && is_single_line_string(ctx.slice(stmt.span())))
            {
                return;
            }

            let after_rule = (self.except_after_rule
                || (self.except_inside_block_and_after_rule && nested))
                && siblings
                    .previous_non_shared(ctx)
                    .is_some_and(|prev| prev.kind.is_rule());
            let except = after_rule
                || (self.except_inside_block && nested)
                || (self.except_first_nested && first_nested)
                || (self.except_after_single_line_comment && siblings.after_comment(ctx, true));
            let always = matches!(
                self.mode,
                RuleEmptyLineMode::Always | RuleEmptyLineMode::AlwaysMultiLine
            );
            siblings.check_gap(ctx, NAME, "rule", stmt.span(), always != except);
        });
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::NAME;
    use crate::tester::{Tester, reject, reject_messages, reject_start};

    const ADD: &str = "Expected empty line before rule";
    const REMOVE: &str = "Expected no empty line before rule";

    #[test]
    fn spacing_always() {
        Tester::new(
            NAME,
            json!("always"),
            ["\n\n a {}", "b {}\n  \t\n\na {}", "@media {\n\na {} }"],
            [
                reject("b {} a {};", "b {}\n\n a {};", ADD, (1, 6, 1, 10)),
                reject_start("/* note */\r\nb {}", "/* note */\r\n\r\nb {}", ADD, (2, 1)),
                reject_start(
                    "a {}; /* keep */ b {}",
                    "a {}; /* keep */\n\n b {}",
                    ADD,
                    (1, 18),
                ),
                reject_messages(
                    "@media { b {}\n\n/* comment */\na {} }",
                    "@media {\n\n b {}\n\n/* comment */\n\na {} }",
                    vec![ADD.into(), ADD.into()],
                ),
                reject_messages(
                    "@keyframes spin { from {} to {} }",
                    "@keyframes spin {\n\n from {}\n\n to {} }",
                    vec![ADD.into(), ADD.into()],
                ),
            ],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_never() {
        Tester::new(
            NAME,
            json!("never"),
            ["\r\n\r\na {}", "a {} b {}", "@media {\n a {}\n}"],
            [
                reject_start("a {} \r\n  \n\n\tb {}", "a {} \r\n\tb {}", REMOVE, (4, 2)),
                reject_start("/* note */ \n\n a {}", "/* note */ \n a {}", REMOVE, (3, 2)),
            ],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_multi_line_modes() {
        Tester::new(
            NAME,
            json!("always-multi-line"),
            ["a {}\nb {}", "a\n{}", "a {}\n\nb,\nc {}"],
            [reject_start(
                "@media {\n b\n {} }",
                "@media {\n\n b\n {} }",
                ADD,
                (2, 2),
            )],
        )
        .with_fix(true)
        .test();
        Tester::new(
            NAME,
            json!("never-multi-line"),
            ["a {}\n\nb {}"],
            [reject_start(
                "a {}\n\nb,\nc {}",
                "a {}\nb,\nc {}",
                REMOVE,
                (3, 1),
            )],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_nonstandard_selectors() {
        Tester::new(NAME, json!("always"), ["a {}\n%base {}\n.x-#{$var} {}"], [])
            .with_custom_syntax("postcss-scss")
            .test();
        Tester::new(NAME, json!("always"), ["a {}\n.mixin-call() {}"], [])
            .with_custom_syntax("postcss-less")
            .test();
    }

    #[test]
    fn spacing_ignore_options() {
        Tester::new(
            NAME,
            json!(["always", {"ignore": ["after-comment", "first-nested"]}]),
            ["a {} /* trailing */ b {}"],
            [reject_start(
                "@media { a {} b {} }",
                "@media { a {}\n\n b {} }",
                ADD,
                (1, 15),
            )],
        )
        .with_fix(true)
        .test();
        Tester::new(
            NAME,
            json!(["always", {"ignore": "inside-block", "except": "inside-block"}]),
            [],
            [reject_start(
                "@media {\n\n a {} b {} } c {}",
                "@media {\n\n a {} b {} }\n\n c {}",
                ADD,
                (3, 14),
            )],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_except_options() {
        Tester::new(
            NAME,
            json!(["always", {"except": ["after-rule", "after-single-line-comment", "first-nested"]}]),
            ["/* own line */ a {}", "a {} /* trailing */\nb {}"],
            [
                reject_messages(
                    "@media { /* attached */\n\n a {} /* trailing */\n\n b {} }",
                    "@media { /* attached */\n a {} /* trailing */\n b {} }",
                    vec![REMOVE.into(), REMOVE.into()],
                ),
                reject_start("/* own line */\n\na {}", "/* own line */\na {}", REMOVE, (3, 1)),
                reject_start("/* multi\nline */\na {}", "/* multi\nline */\n\na {}", ADD, (3, 1)),
                reject_start("@media {}\na {}", "@media {}\n\na {}", ADD, (2, 1)),
            ],
        )
        .with_fix(true)
        .test();
        Tester::new(
            NAME,
            json!(["always", {"except": "inside-block-and-after-rule"}]),
            [],
            [reject_messages(
                "@media {\n\n a {}\n\n b {} }\n\nc {}\nd {}",
                "@media {\n\n a {}\n b {} }\n\nc {}\n\nd {}",
                vec![REMOVE.into(), ADD.into()],
            )],
        )
        .with_fix(true)
        .test();
        Tester::new(
            NAME,
            json!(["always", {"except": ["inside-block", "first-nested"]}]),
            [],
            [reject_start(
                "@media {\n\n a {} b {} }",
                "@media {\n a {} b {} }",
                REMOVE,
                (3, 2),
            )],
        )
        .with_fix(true)
        .test();
    }
}
