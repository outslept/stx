use oxc_css_parser::{
    Span,
    ast::{SimpleBlock, Statement, Stylesheet},
    token::{Comment, CommentKind},
};

use crate::{
    config::RuleSetting,
    context::LintContext,
    fix::Fix,
    rules::{Rule, at_rule_empty_line_before::EmptyLineMode},
    utils::{
        OptionMatcher, is_configuration_comment, is_whitespace, options_matches,
        parse_option_matchers, secondary_has_flag,
    },
};

const NAME: &str = "comment-empty-line-before";

pub struct CommentEmptyLineBefore {
    mode: EmptyLineMode,
    except_first_nested: bool,
    ignore_commands: bool,
    ignore_after_comment: bool,
    ignore_comments: Vec<OptionMatcher>,
}

impl CommentEmptyLineBefore {
    pub fn from_setting(setting: &RuleSetting) -> Self {
        let secondary = setting.secondary.as_ref();
        Self {
            mode: if setting.primary.as_str() == Some("never") {
                EmptyLineMode::Never
            } else {
                EmptyLineMode::Always
            },
            except_first_nested: secondary_has_flag(secondary, "except", "first-nested"),
            ignore_commands: secondary_has_flag(secondary, "ignore", "stylelint-commands"),
            ignore_after_comment: secondary_has_flag(secondary, "ignore", "after-comment"),
            ignore_comments: parse_option_matchers(secondary, "ignoreComments"),
        }
    }
}

impl Rule for CommentEmptyLineBefore {
    fn name(&self) -> &'static str {
        NAME
    }

    fn check_stylesheet<'a>(&self, ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        walk_siblings(ast, ctx, |siblings, ctx| {
            let ChildKind::Comment(comment) = siblings.current().kind else {
                return;
            };
            let text = comment.content.trim();
            if comment.kind != CommentKind::Block
                || (self.ignore_commands && is_configuration_comment(text))
                || (self.ignore_after_comment && siblings.after_comment(ctx, false))
                || options_matches(&self.ignore_comments, text)
                || siblings.comment_shares_line(ctx, None)
            {
                return;
            }
            let expect_empty = self.mode == EmptyLineMode::Always
                && !(self.except_first_nested && siblings.first_nested(ctx));
            siblings.check_gap(ctx, NAME, "comment", comment.span, expect_empty);
        });
    }
}

#[derive(Clone, Copy)]
pub(super) enum ChildKind<'a> {
    Statement(&'a Statement<'a>),
    Comment(&'a Comment<'a>),
}

impl ChildKind<'_> {
    pub(super) fn is_comment(self) -> bool {
        matches!(self, Self::Comment(_))
    }

    pub(super) fn is_rule(self) -> bool {
        matches!(
            self,
            Self::Statement(
                Statement::QualifiedRule(_)
                    | Statement::KeyframeBlock(_)
                    | Statement::UnknownQualifiedRule(_)
                    | Statement::LessConditionalQualifiedRule(_)
                    | Statement::LessMixinDefinition(_)
            )
        )
    }
}

#[derive(Clone, Copy)]
pub(super) struct Child<'a> {
    pub kind: ChildKind<'a>,
    pub span: Span,
}

#[derive(Clone, Copy)]
pub(super) struct Siblings<'a, 's> {
    children: &'s [Child<'a>],
    index: usize,
    pub parent: Option<(&'a Statement<'a>, Span)>,
    opening_brace_end: Option<u32>,
    newline: &'static str,
}

impl<'a> Siblings<'a, '_> {
    pub(super) fn current(self) -> Child<'a> {
        self.children[self.index]
    }

    pub(super) fn previous(self) -> Option<Child<'a>> {
        self.index.checked_sub(1).map(|i| self.children[i])
    }

    pub(super) fn previous_non_shared(self, ctx: &LintContext<'_>) -> Option<Child<'a>> {
        (0..self.index).rev().find_map(|i| {
            let candidate = self.children[i];
            (!candidate.kind.is_comment() || !self.comment_starts_on_neighbor_line(ctx, i))
                .then_some(candidate)
        })
    }

    pub(super) fn first_nested(self, ctx: &LintContext<'_>) -> bool {
        self.parent.is_some()
            && self.previous().is_none_or(|prev| {
                self.opening_brace_end
                    .is_some_and(|start| same_line(ctx, start, prev.span.end))
                    && self.children[..self.index]
                        .iter()
                        .all(|child| child.kind.is_comment())
            })
    }

    pub(super) fn after_comment(self, ctx: &LintContext<'_>, single_line_only: bool) -> bool {
        self.index.checked_sub(1).is_some_and(|index| {
            let prev = Self { index, ..self };
            let child = prev.current();
            child.kind.is_comment()
                && (!single_line_only || same_line(ctx, child.span.start, child.span.end))
                && !prev.comment_shares_line(ctx, single_line_only.then_some(self.index))
        })
    }

    fn comment_starts_on_neighbor_line(self, ctx: &LintContext<'_>, index: usize) -> bool {
        let start = self.children[index].span.start;
        (index > 0 && same_line(ctx, self.children[index - 1].span.start, start))
            || self
                .children
                .get(index + 1)
                .is_some_and(|next| same_line(ctx, start, next.span.start))
    }

    fn comment_shares_line(self, ctx: &LintContext<'_>, ignored_next: Option<usize>) -> bool {
        let span = self.current().span;
        if self
            .previous_non_shared(ctx)
            .is_some_and(|prev| same_line(ctx, prev.span.end, span.start))
            || (self.index == 0
                && self
                    .opening_brace_end
                    .is_some_and(|start| same_line(ctx, start, span.start)))
        {
            return true;
        }
        (self.index + 1..self.children.len())
            .find(|&i| {
                !self.children[i].kind.is_comment() || !self.comment_starts_on_neighbor_line(ctx, i)
            })
            .is_some_and(|i| {
                Some(i) != ignored_next && same_line(ctx, span.end, self.children[i].span.start)
            })
    }

    pub(super) fn check_gap(
        self,
        ctx: &mut LintContext<'_>,
        rule: &'static str,
        subject: &str,
        span: Span,
        expect_empty: bool,
    ) {
        let floor = self
            .previous()
            .map(|prev| prev.span.end)
            .or(self.opening_brace_end);
        let prefix = ctx.slice_range(floor.unwrap_or(0), span.start);
        let before = &prefix[prefix.trim_end_matches(is_whitespace).len()..];
        let has_empty = before.split_inclusive('\n').skip(1).any(|line| {
            line.strip_suffix('\n')
                .is_some_and(|line| line.trim_matches(['\r', '\t', ' ']).is_empty())
        });
        if expect_empty == has_empty {
            return;
        }

        let start = span.start as usize - before.len();
        let first_lf = before.find('\n');
        let first_break = first_lf.map(|lf| lf - usize::from(before[..lf].ends_with('\r')));
        let fix = if expect_empty {
            let at = start + first_break.unwrap_or(0);
            let text = self.newline.repeat(if first_lf.is_some() { 1 } else { 2 });
            Fix::single(byte_span(at, at), text)
        } else {
            let first_lf = first_lf.unwrap();
            let first_break = first_break.unwrap();
            let (offset, text) = if &before[first_break..=first_lf] == self.newline {
                (first_lf + 1, "")
            } else {
                (first_break, self.newline)
            };
            let end = start + before.rfind('\n').unwrap() + 1;
            Fix::single(byte_span(start + offset, end), text)
        };
        let no = if expect_empty { "" } else { "no " };
        ctx.report_with_fix(
            rule,
            span,
            format!("Expected {no}empty line before {subject}"),
            fix,
        );
    }
}

fn byte_span(start: usize, end: usize) -> Span {
    Span {
        start: start as u32,
        end: end as u32,
    }
}

fn same_line(ctx: &LintContext<'_>, start: u32, end: u32) -> bool {
    !ctx.slice_range(start, end).contains('\n')
}

pub(super) fn walk_siblings<'a>(
    ast: &'a Stylesheet<'a>,
    ctx: &mut LintContext<'a>,
    mut visit: impl FnMut(Siblings<'a, '_>, &mut LintContext<'a>),
) {
    let newline = match ctx.source.find('\n') {
        Some(i) if ctx.source[..i].ends_with('\r') => "\r\n",
        _ => "\n",
    };
    walk_container(&ast.statements, ast.span, None, ctx, newline, &mut visit);
}

fn walk_container<'a>(
    statements: &'a [Statement<'a>],
    span: Span,
    parent: Option<&'a Statement<'a>>,
    ctx: &mut LintContext<'a>,
    newline: &'static str,
    visit: &mut impl FnMut(Siblings<'a, '_>, &mut LintContext<'a>),
) {
    let braced = parent.is_some() && ctx.slice(span).starts_with('{');
    let body_start = span.start + u32::from(braced);
    let body_end = span.end - u32::from(braced && ctx.slice(span).ends_with('}'));
    let mut children = Vec::with_capacity(statements.len());
    let mut end = body_start;
    let push_comments = |start, end, children: &mut Vec<_>| {
        children.extend(
            ctx.comments_in_range(start, end)
                .iter()
                .map(|comment| Child {
                    kind: ChildKind::Comment(comment),
                    span: comment.span,
                }),
        );
    };
    for (i, statement) in statements.iter().enumerate() {
        let mut span = statement.span();
        push_comments(end, span.start, &mut children);
        let limit = statements
            .get(i + 1)
            .map_or(body_end, |next| next.span().start);
        span.end = statement_end(ctx, statement, span.end, limit);
        children.push(Child {
            kind: ChildKind::Statement(statement),
            span,
        });
        end = span.end;
    }
    push_comments(end, body_end, &mut children);

    for (index, child) in children.iter().enumerate() {
        if index > 0 || parent.is_some() {
            visit(
                Siblings {
                    children: &children,
                    index,
                    parent: parent.map(|parent| (parent, span)),
                    opening_brace_end: braced.then_some(body_start),
                    newline,
                },
                ctx,
            );
        }
        if let ChildKind::Statement(statement) = child.kind {
            let mut visit_block = |block: &'a SimpleBlock<'a>| {
                walk_container(
                    &block.statements,
                    block.span,
                    Some(statement),
                    ctx,
                    newline,
                    visit,
                );
            };
            match statement {
                Statement::QualifiedRule(node) => visit_block(&node.block),
                Statement::UnknownQualifiedRule(node) => visit_block(&node.block),
                Statement::KeyframeBlock(node) => visit_block(&node.block),
                Statement::LessConditionalQualifiedRule(node) => visit_block(&node.block),
                Statement::LessMixinDefinition(node) => visit_block(&node.block),
                Statement::AtRule(node) => node.block.iter().for_each(visit_block),
                Statement::UnknownSassAtRule(node) => node.block.iter().for_each(visit_block),
                Statement::SassIfAtRule(node) => {
                    visit_block(&node.if_clause.block);
                    for clause in &node.else_if_clauses {
                        visit_block(&clause.block);
                    }
                    node.else_clause.iter().for_each(visit_block);
                }
                _ => {}
            }
        }
    }
}

fn statement_end(ctx: &LintContext<'_>, statement: &Statement<'_>, end: u32, limit: u32) -> u32 {
    let mut cursor = end;
    let accepts_comments_before_semicolon = matches!(
        statement,
        Statement::Declaration(_)
            | Statement::SassVariableDeclaration(_)
            | Statement::LessVariableDeclaration(_)
            | Statement::PostcssSimpleVarDeclaration(_)
    ) || matches!(statement, Statement::AtRule(at_rule) if at_rule.block.is_none());
    let mut comments = ctx.comments_in_range(end, limit).iter();
    loop {
        let gap = ctx.slice_range(cursor, limit);
        let rest = gap.trim_start_matches(is_whitespace);
        cursor += (gap.len() - rest.len()) as u32;
        if rest.starts_with(';') {
            return cursor + 1;
        }
        if !accepts_comments_before_semicolon {
            return end;
        }
        let Some(comment) = comments
            .next()
            .filter(|c| c.kind == CommentKind::Block && c.span.start == cursor)
        else {
            return end;
        };
        cursor = comment.span.end;
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::NAME;
    use crate::tester::{Tester, reject, reject_start};

    const ADD: &str = "Expected empty line before comment";
    const REMOVE: &str = "Expected no empty line before comment";

    #[test]
    fn spacing_always() {
        Tester::new(
            NAME,
            json!("always"),
            [
                "/** comment */",
                "a {} /** comment */",
                "a { color: pink; /** comment */\ntop: 0; }",
                "a,\n b { /** shared-line comment */\n color: pink; }",
                "a {}\n\r\n/** comment */",
                "a {\n\n  /* comment */\n  color: pink;\n}",
                "a,\n/* selector */\nb { color:\n/* value */\n pink; }",
            ],
            [
                reject(
                    "a {\n  /* comment */\n  color: pink;\n}",
                    "a {\n\n  /* comment */\n  color: pink;\n}",
                    ADD,
                    (2, 3, 2, 16),
                ),
                reject_start(
                    "/** comment */\r\n/** comment */",
                    "/** comment */\r\n\r\n/** comment */",
                    ADD,
                    (2, 1),
                ),
                reject_start(
                    "a { color: pink;\n/** comment */\ntop: 0; }",
                    "a { color: pink;\n\n/** comment */\ntop: 0; }",
                    ADD,
                    (2, 1),
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
                "\r\n\r\n/** comment */",
                "a {} /** comment */",
                "a { color: pink;\n/** comment */\n\ntop: 0; }",
            ],
            [
                reject(
                    "/** comment */\n\n/** comment */",
                    "/** comment */\n/** comment */",
                    REMOVE,
                    (3, 1, 3, 15),
                ),
                reject_start(
                    "a {} \r\n\t\r\n\r\n  /** comment */",
                    "a {} \r\n  /** comment */",
                    REMOVE,
                    (4, 3),
                ),
            ],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_except_first_nested() {
        Tester::new(
            NAME,
            json!(["always", {"except": ["first-nested"]}]),
            [
                "a {\n  /* comment */\n  color: pink;\n}",
                "a { /* shared-line comment */\n  /* comment */\n  color: pink;\n}",
            ],
            [
                reject_start(
                    "a {\n\n  /* comment */\n  color: pink;\n}",
                    "a {\n  /* comment */\n  color: pink;\n}",
                    REMOVE,
                    (3, 3),
                ),
                reject_start(
                    "a { /* shared-line comment */\n\n  /* comment */\n  color: pink;\n}",
                    "a { /* shared-line comment */\n  /* comment */\n  color: pink;\n}",
                    REMOVE,
                    (3, 3),
                ),
            ],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_ignore_after_comment() {
        Tester::new(
            NAME,
            json!(["always", {"ignore": ["after-comment"]}]),
            ["/* a */\n/* b */\n/* c */\nbody {}"],
            [
                reject_start(
                    "a { color: pink;\n/** comment */\n/** comment */\ntop: 0; }",
                    "a { color: pink;\n\n/** comment */\n/** comment */\ntop: 0; }",
                    ADD,
                    (2, 1),
                ),
                reject_start(
                    "a {} /* trailing */\n/* comment */\nb {}",
                    "a {} /* trailing */\n\n/* comment */\nb {}",
                    ADD,
                    (2, 1),
                ),
            ],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_ignore_commands_and_comments() {
        Tester::new(
            NAME,
            json!(["always", {
                "ignore": ["stylelint-commands"],
                "ignoreComments": ["/^ignore/", "/ignore$/", "string-ignore"]
            }]),
            [
                "a {\ncolor: pink;\n/* stylelint-disable color-no-hex */\ntop: 0;\n}",
                "a {\ncolor: pink;\n/* ignore-at-start */\ntop: 0;\n}",
                "a {\ncolor: pink;\n/* at-end-ignore */\ntop: 0;\n}",
                "a {\ncolor: pink;\n/* string-ignore */\ntop: 0;\n}",
            ],
            [reject_start(
                "a {}\n/* not a configuration comment */",
                "a {}\n\n/* not a configuration comment */",
                ADD,
                (2, 1),
            )],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn spacing_ignore_line_comments() {
        for syntax in ["postcss-scss", "postcss-less"] {
            Tester::new(
                NAME,
                json!("always"),
                [
                    "a { color: pink;\n// comment\ntop: 0; }",
                    "// first line\n// second line\na { color: pink; }",
                ],
                [],
            )
            .with_custom_syntax(syntax)
            .test();
        }
    }
}
