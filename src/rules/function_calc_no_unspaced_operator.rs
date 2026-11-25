use std::{borrow::Cow, sync::LazyLock};

use oxc_css_parser::{Span, ast::Declaration};
use regex::Regex;

use crate::{
    context::{Fix, LintContext},
    rules::Rule,
    utils::{ValueToken, ValueTokenKind, is_math_function, is_only_whitespace, tokenize_value},
};

const NAME: &str = "function-calc-no-unspaced-operator";

pub struct FunctionCalcNoUnspacedOperator;

impl Rule for FunctionCalcNoUnspacedOperator {
    fn name(&self) -> &'static str {
        NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let span = Span {
            start: decl.colon_span.end,
            end: decl
                .important
                .as_ref()
                .map_or(decl.span.end, |imp| imp.span.start),
        };
        let value = ctx.slice(span);
        if !value.contains('(') || !value.contains(['+', '-']) {
            return;
        }

        let masked = mask_value(value, span, ctx);
        let mut tokens = tokenize_value(
            &masked,
            Span {
                start: 0,
                end: span.end - span.start,
            },
        )
        .into_iter()
        .peekable();
        let mut groups: Vec<Group> = Vec::new();
        let mut terms = Vec::new();
        while let Some(token) = tokens.next() {
            let mut start = span.start + token.span.start;
            let end = span.start + token.span.end;
            let in_math = groups.last().is_some_and(|group| group.math);
            let in_url = groups.last().is_some_and(|group| group.url);
            let function =
                (token.kind == ValueTokenKind::Function).then(|| token.raw.trim_end_matches('('));
            let close = match token.raw {
                "(" => Some(")"),
                "[" => Some("]"),
                "{" => Some("}"),
                "#" if tokens
                    .next_if(|next| next.raw == "{" && next.span.start == token.span.end)
                    .is_some() =>
                {
                    Some("}")
                }
                _ if function.is_some() => Some(")"),
                _ => None,
            };

            if let Some(close) = close {
                let function = if function == Some("-") {
                    if in_math {
                        terms.push(Term {
                            span: Span {
                                start,
                                end: start + 1,
                            },
                            kind: TermKind::Operator,
                        });
                    }
                    start += 1;
                    None
                } else {
                    function
                };
                let url = in_url || function.is_some_and(|name| name.eq_ignore_ascii_case("url"));
                let math = !url && function.map_or(close == ")" && in_math, is_math_function);
                groups.push(Group {
                    start,
                    first: terms.len(),
                    close,
                    math,
                    url,
                    operand: math || function.is_none_or(|name| name.eq_ignore_ascii_case("var")),
                });
            } else if matches!(token.raw, ")" | "]" | "}") {
                let Some(group) = groups.pop() else { continue };
                if group.close != token.raw {
                    break;
                }
                if group.math {
                    check_operations(&terms[group.first..], ctx);
                }
                terms.truncate(group.first);
                if groups.last().is_some_and(|parent| parent.math) {
                    let kind = if group.operand {
                        TermKind::operand(false, false)
                    } else {
                        TermKind::Other
                    };
                    terms.push(Term {
                        span: Span {
                            start: group.start,
                            end,
                        },
                        kind,
                    });
                }
            } else if in_math {
                push_term(&mut terms, token, Span { start, end });
            }
        }
    }
}

struct Group {
    start: u32,
    first: usize,
    close: &'static str,
    math: bool,
    url: bool,
    operand: bool,
}

struct Term {
    span: Span,
    kind: TermKind,
}

#[derive(Clone, Copy)]
enum TermKind {
    Operand { signed: bool, trailing_minus: bool },
    Operator,
    Boundary,
    Other,
}

impl TermKind {
    fn operand(signed: bool, trailing_minus: bool) -> Self {
        Self::Operand {
            signed,
            trailing_minus,
        }
    }
}

fn mask_value<'a>(value: &'a str, span: Span, ctx: &LintContext<'a>) -> Cow<'a, str> {
    static ESCAPE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\\(?:[0-9a-fA-F]{1,6}(?:\r\n|[ \t\r\n\x0c])?|[^\r\n\x0c])").unwrap()
    });

    let comments = ctx.comments_in_range(span.start, span.end);
    if comments.is_empty() && !value.contains('\\') {
        return Cow::Borrowed(value);
    }
    let mut masked = value.as_bytes().to_vec();
    for comment in comments {
        let range =
            (comment.span.start - span.start) as usize..(comment.span.end - span.start) as usize;
        masked[range].fill(b' ');
    }
    if value.contains('\\') {
        let mut comments = comments.iter().peekable();
        for escape in ESCAPE.find_iter(value) {
            let start = span.start + escape.start() as u32;
            while comments
                .next_if(|comment| comment.span.end <= start)
                .is_some()
            {}
            if comments
                .peek()
                .is_none_or(|comment| start < comment.span.start)
            {
                masked[escape.range()].fill(b'_');
            }
        }
    }
    Cow::Owned(String::from_utf8(masked).unwrap())
}

fn push_term(terms: &mut Vec<Term>, token: ValueToken<'_>, mut span: Span) {
    static NEGATIVE_NUMBER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^-[0-9]+(?:[eE]-[0-9]+)?").unwrap());

    let raw = token.raw;
    let mut signed = raw.starts_with(['+', '-']);
    let kind = match (raw, token.kind) {
        ("+" | "-", _) => TermKind::Operator,
        ("," | ";" | "*" | "/", _) => TermKind::Boundary,
        (_, ValueTokenKind::Number | ValueTokenKind::Percentage) => {
            TermKind::operand(signed, false)
        }
        (_, ValueTokenKind::Dimension { num_len }) => {
            let mut unit = &raw[num_len..];
            while !unit.starts_with("--") {
                let Some(dash) = unit.find('-') else { break };
                let suffix = &unit[dash..];
                let Some(number) = NEGATIVE_NUMBER.find(suffix) else {
                    break;
                };
                let split = span.end - suffix.len() as u32;
                terms.push(Term {
                    span: Span {
                        start: span.start,
                        end: split,
                    },
                    kind: TermKind::operand(signed, false),
                });
                span.start = split;
                signed = true;
                unit = &suffix[number.end()..];
            }
            TermKind::operand(signed, unit.ends_with('-'))
        }
        (_, ValueTokenKind::Ident) if !raw.starts_with(['$', '@']) => {
            TermKind::operand(false, raw.ends_with('-'))
        }
        _ => TermKind::Other,
    };
    terms.push(Term { span, kind });
}

fn check_operations(terms: &[Term], ctx: &mut LintContext<'_>) {
    for part in terms.split(|term| matches!(term.kind, TermKind::Boundary)) {
        let mut terms = part.iter();
        let Some(mut lhs) = terms.next() else {
            continue;
        };
        while let TermKind::Operand { trailing_minus, .. } = lhs.kind {
            let Some(next) = terms.next() else { break };
            let (rhs, op, before, after, sign) = match next.kind {
                TermKind::Operator => {
                    let Some(rhs) = terms.next() else { break };
                    if !matches!(rhs.kind, TermKind::Operand { .. }) {
                        break;
                    }
                    (rhs, next.span.start, lhs.span.end, rhs.span.start, false)
                }
                TermKind::Operand { .. } if trailing_minus => {
                    let op = lhs.span.end - 1;
                    (next, op, op, next.span.start, false)
                }
                TermKind::Operand { signed: true, .. } => {
                    let op = next.span.start;
                    (next, op, lhs.span.end, op + 1, true)
                }
                TermKind::Operand { .. } => {
                    lhs = next;
                    continue;
                }
                _ => break,
            };
            check_spacing(op, before, after, sign, ctx);
            lhs = rhs;
        }
    }
}

fn check_spacing(op: u32, before: u32, after: u32, sign: bool, ctx: &mut LintContext<'_>) {
    let mut sides = [(before, op, op, "before"), (op + 1, after, op + 1, "after")];
    if sign {
        sides.swap(0, 1);
    }
    let mut edits = Vec::new();
    for (start, end, insert, side) in sides {
        let mut has_space = false;
        let mut cursor = start;
        let comments = ctx
            .comments_in_range(start, end)
            .iter()
            .map(|comment| comment.span);
        for comment in comments.chain(std::iter::once(Span { start: end, end })) {
            let whitespace = ctx.slice_range(cursor, comment.start);
            if !is_only_whitespace(whitespace) {
                return;
            }
            if !whitespace.is_empty() {
                has_space = true;
                if whitespace != " " {
                    let (len, replacement) = match whitespace.find('\n') {
                        Some(i) => (i - usize::from(whitespace[..i].ends_with('\r')), ""),
                        None => (whitespace.len(), " "),
                    };
                    if len != 0 {
                        edits.push((
                            false,
                            side,
                            Span {
                                start: cursor,
                                end: cursor + len as u32,
                            },
                            replacement,
                        ));
                    }
                }
            }
            cursor = comment.end;
        }
        if !has_space {
            edits.push((
                true,
                side,
                Span {
                    start: insert,
                    end: insert,
                },
                " ",
            ));
        }
    }

    let span = Span {
        start: op,
        end: op + 1,
    };
    let operator = ctx.slice(span);
    edits.sort_by_key(|(missing, ..)| !missing);
    for (_, side, range, replacement) in edits {
        ctx.report_with_fix(
            NAME,
            span,
            format!("Expected single space {side} \"{operator}\" operator"),
            Fix::single(range, replacement),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::NAME;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn checks_binary_addition_without_changing_signs_or_nested_non_math() {
        Tester::new(
            NAME,
            json!(true),
            [
                "a { x: calc(1e-3px + -2px*-.5); }",
                "a { x: calc(var(--x, 1px+2px) + [3px+4px]); }",
                "a { x: rem-calc(1px+2px); content: 'calc(2px+3px)'; }",
            ],
            [
                reject_warnings(
                    "a { x: min(2px-3px, -1px); }",
                    "a { x: min(2px - 3px, -1px); }",
                    vec![
                        (
                            "Expected single space after \"-\" operator".into(),
                            (1, 15, 1, 16),
                        ),
                        (
                            "Expected single space before \"-\" operator".into(),
                            (1, 15, 1, 16),
                        ),
                    ],
                ),
                reject(
                    "a { x: calc(1px + \t\r\n  -2px); }",
                    "a { x: calc(1px +\r\n  -2px); }",
                    "Expected single space after \"+\" operator",
                    (1, 17, 1, 18),
                ),
                reject(
                    r"a { x: calc(1\23 - 2px); }",
                    r"a { x: calc(1\23  - 2px); }",
                    "Expected single space before \"-\" operator",
                    (1, 18, 1, 19),
                ),
            ],
        )
        .with_fix(true)
        .test();
    }
}
