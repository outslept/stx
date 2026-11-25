use oxc_css_parser::ast::{ComponentValue, Declaration, Statement, Stylesheet};

use crate::{
    config::RuleSetting,
    context::LintContext,
    rules::{
        Rule,
        at_rule_empty_line_before::EmptyLineMode,
        comment_empty_line_before::{ChildKind, walk_siblings},
    },
    utils::{is_single_line_string, secondary_has_flag},
};

const NAME: &str = "declaration-empty-line-before";

pub struct DeclarationEmptyLineBefore {
    mode: EmptyLineMode,
    ignore_after_comment: bool,
    ignore_after_declaration: bool,
    ignore_first_nested: bool,
    ignore_inside_single_line_block: bool,
    except_first_nested: bool,
    except_after_block: bool,
    except_after_comment: bool,
    except_after_declaration: bool,
}

impl DeclarationEmptyLineBefore {
    pub fn from_setting(setting: &RuleSetting) -> Self {
        let flag = |key, name| secondary_has_flag(setting.secondary.as_ref(), key, name);
        Self {
            mode: if setting.primary.as_str() == Some("never") {
                EmptyLineMode::Never
            } else {
                EmptyLineMode::Always
            },
            ignore_after_comment: flag("ignore", "after-comment"),
            ignore_after_declaration: flag("ignore", "after-declaration"),
            ignore_first_nested: flag("ignore", "first-nested"),
            ignore_inside_single_line_block: flag("ignore", "inside-single-line-block"),
            except_first_nested: flag("except", "first-nested"),
            except_after_block: flag("except", "after-block"),
            except_after_comment: flag("except", "after-comment"),
            except_after_declaration: flag("except", "after-declaration"),
        }
    }
}

impl Rule for DeclarationEmptyLineBefore {
    fn name(&self) -> &'static str {
        NAME
    }

    fn check_stylesheet<'a>(&self, ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        walk_siblings(ast, ctx, |siblings, ctx| {
            let child = siblings.current();
            let ChildKind::Statement(Statement::Declaration(decl)) = child.kind else {
                return;
            };
            if !is_standard_property(ctx, decl) {
                return;
            }
            if let Some((parent, block)) = siblings.parent {
                if self.ignore_inside_single_line_block && is_single_line_string(ctx.slice(block)) {
                    return;
                }
                if matches!(
                    parent,
                    Statement::UnknownQualifiedRule(_) | Statement::LessMixinDefinition(_)
                ) {
                    let header = ctx.slice_range(parent.span().start, block.start).trim_end();
                    if (header.ends_with(':') && !header.starts_with("--"))
                        || (header.starts_with('#') && header.ends_with("()"))
                    {
                        return;
                    }
                }
            } else if self.ignore_inside_single_line_block && is_single_line_string(ctx.source) {
                return;
            }

            let first_nested = (self.ignore_first_nested || self.except_first_nested)
                && siblings.first_nested(ctx);
            let after_comment = (self.ignore_after_comment || self.except_after_comment)
                && siblings.after_comment(ctx, false);
            let after_declaration = (self.ignore_after_declaration
                || self.except_after_declaration)
                && matches!(
                    siblings.previous_non_shared(ctx).map(|prev| prev.kind),
                    Some(ChildKind::Statement(Statement::Declaration(prev))) if is_standard_property(ctx, prev)
                );
            if (self.ignore_first_nested && first_nested)
                || (self.ignore_after_comment && after_comment)
                || (self.ignore_after_declaration && after_declaration)
            {
                return;
            }

            let after_block = self.except_after_block
                && siblings.previous().is_some_and(|prev| {
                    prev.kind.is_rule()
                        || matches!(
                            prev.kind,
                            ChildKind::Statement(
                                Statement::AtRule(_)
                                    | Statement::SassIfAtRule(_)
                                    | Statement::UnknownSassAtRule(_)
                            )
                        )
                });
            let except = (self.except_first_nested && first_nested)
                || (self.except_after_comment && after_comment)
                || (self.except_after_declaration && after_declaration)
                || after_block;
            siblings.check_gap(
                ctx,
                NAME,
                "declaration",
                child.span,
                (self.mode == EmptyLineMode::Always) != except,
            );
        });
    }
}

fn is_standard_property(ctx: &LintContext<'_>, decl: &Declaration<'_>) -> bool {
    let prop = ctx.slice(decl.name.span());
    !prop.starts_with("--")
        && !prop.starts_with('$')
        && !prop.contains(".$")
        && (!prop.starts_with('@') || prop.starts_with("@{"))
        && !matches!(
            decl.value.last(),
            Some(ComponentValue::SassNestingDeclaration(_))
        )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::NAME;
    use crate::tester::{Tester, reject, reject_messages, reject_start};

    const ADD: &str = "Expected empty line before declaration";
    const REMOVE: &str = "Expected no empty line before declaration";

    #[test]
    fn spacing_always() {
        Tester::new(
            NAME,
            json!("always"),
            [
                "a {\n\n color: red;\n \t\n background: blue\n}",
                "a {\r\n --x: red;\r\n\r\n color: var(--x);\r\n}",
                "a { --tokens: { x: '}' }; }",
            ],
            [
                reject(
                    "a { top: 15px; }",
                    "a {\n\n top: 15px; }",
                    ADD,
                    (1, 5, 1, 15),
                ),
                reject("a { top: 15px }", "a {\n\n top: 15px }", ADD, (1, 5, 1, 14)),
                reject_messages(
                    "a { top: 15px /* keep */; width: 0 }",
                    "a {\n\n top: 15px /* keep */;\n\n width: 0 }",
                    vec![ADD.into(), ADD.into()],
                ),
                reject_start(
                    "a {\n\n top: 15px; \n bottom: 5px; }",
                    "a {\n\n top: 15px; \n\n bottom: 5px; }",
                    ADD,
                    (4, 2),
                ),
                reject_start(
                    "a {\r\n /* label */\r\n\tcolor: red\r\n}",
                    "a {\r\n /* label */\r\n\r\n\tcolor: red\r\n}",
                    ADD,
                    (3, 2),
                ),
                reject_start(
                    "@keyframes spin {\n to {\n  opacity: 0\n }\n}",
                    "@keyframes spin {\n to {\n\n  opacity: 0\n }\n}",
                    ADD,
                    (3, 3),
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
            [
                "a { color: red; width: 0 }",
                "a {\n\n --x: 0;\n color: red }",
            ],
            [
                reject_start(
                    "a { color: red; /* keep */ \r\n  \t\n\n\tbackground: blue\n}",
                    "a { color: red; /* keep */ \r\n\tbackground: blue\n}",
                    REMOVE,
                    (4, 2),
                ),
                reject_start("a {\n\n top: 15px; }", "a {\n top: 15px; }", REMOVE, (3, 2)),
            ],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_interpolated_properties() {
        for (syntax, source, fixed) in [
            (
                "postcss-scss",
                "a { #{var}: 15px; }",
                "a {\n\n #{var}: 15px; }",
            ),
            (
                "postcss-less",
                "a { @{var}: 15px; }",
                "a {\n\n @{var}: 15px; }",
            ),
        ] {
            Tester::new(
                NAME,
                json!("always"),
                [],
                [reject_start(source, fixed, ADD, (1, 5))],
            )
            .with_custom_syntax(syntax)
            .with_fix(true)
            .test();
        }
    }

    #[test]
    fn spacing_ignore_options() {
        Tester::new(
            NAME,
            json!(["always", {
                "ignore": ["after-comment", "after-declaration", "first-nested", "inside-single-line-block"]
            }]),
            [
                "a { top: 15px; width: 0 }",
                "a { /* attached */\n top: 15px; /* trailing */\n width: 0 }",
                "a {\n /* note */\n top: 15px; }",
            ],
            [
                reject_start(
                    "a {\n /* note */ top: 15px; }",
                    "a {\n /* note */\n\n top: 15px; }",
                    ADD,
                    (2, 13),
                ),
                reject_start(
                    "a {\n --x: 0;\n width: 0 }",
                    "a {\n --x: 0;\n\n width: 0 }",
                    ADD,
                    (3, 2),
                ),
            ],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_except_options() {
        Tester::new(
            NAME,
            json!(["never", {"except": ["after-block", "after-comment", "after-declaration", "first-nested"]}]),
            [
                "a { b {} /* trailing */\n color: red }",
                "a {\n /* note */ color: red }",
            ],
            [
                reject_messages(
                    "a {\n top: 15px; /* trailing */\n width: 0 }",
                    "a {\n\n top: 15px; /* trailing */\n\n width: 0 }",
                    vec![ADD.into(), ADD.into()],
                ),
                reject_start(
                    "a { @include mixin; color: red }",
                    "a { @include mixin;\n\n color: red }",
                    ADD,
                    (1, 21),
                ),
                reject_start("a { b {}\n color: red }", "a { b {}\n\n color: red }", ADD, (2, 2)),
                reject_start(
                    "a {\n /* note */\n color: red }",
                    "a {\n /* note */\n\n color: red }",
                    ADD,
                    (3, 2),
                ),
            ],
        )
        .with_fix(true)
        .test();
        Tester::new(
            NAME,
            json!(["never", {"ignore": "first-nested", "except": "first-nested"}]),
            ["a { color: red }"],
            [],
        )
        .test();
    }
}
