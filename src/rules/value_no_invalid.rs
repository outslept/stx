use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        ValueToken, ValueTokenKind, decl_value_span, ident_name, is_custom_property,
        is_standard_syntax_value, tokenize_value,
    },
};
use oxc_css_parser::{Span, ast::Declaration};

pub struct ValueNoInvalid;

fn is_dim(kind: &ValueTokenKind) -> bool {
    matches!(
        kind,
        ValueTokenKind::Dimension { .. } | ValueTokenKind::Percentage
    )
}

fn is_math_function(token: &ValueToken<'_>) -> bool {
    if token.kind != ValueTokenKind::Function {
        return false;
    }
    let name = token.raw.strip_suffix('(').unwrap_or(token.raw);
    matches!(
        name.to_ascii_lowercase().as_str(),
        "calc" | "min" | "max" | "clamp"
    )
}

fn is_numeric_random(tokens: &[ValueToken<'_>], start: usize) -> bool {
    let Some([function, lower, comma, upper, close]) = tokens.get(start..start + 5) else {
        return false;
    };
    function.kind == ValueTokenKind::Function
        && function.raw.eq_ignore_ascii_case("random(")
        && lower.kind == ValueTokenKind::Number
        && comma.kind == ValueTokenKind::Comma
        && upper.kind == ValueTokenKind::Number
        && close.kind == ValueTokenKind::CloseParen
}

impl Rule for ValueNoInvalid {
    fn name(&self) -> &'static str {
        "value-no-invalid"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if !ctx.is_standard_declaration(decl) {
            return;
        }
        let Some(span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let value = ctx.slice(span);
        if !is_standard_syntax_value(value) {
            return;
        }

        let tokens = tokenize_value(ctx.source, span);
        let mut syntax_error = None;
        let check_syntax = ident_name(&decl.name).is_some_and(|prop| !is_custom_property(prop));
        // (is math, index of the function token)
        let mut function_stack: Vec<(bool, usize)> = Vec::new();
        let mut math_depth = 0usize;
        // css-calc stops at the first error of a math expression.
        let mut reported = false;
        for (index, token) in tokens.iter().enumerate() {
            if token.kind == ValueTokenKind::Function || token.raw == "(" {
                let math = is_math_function(token);
                function_stack.push((math, index));
                math_depth += usize::from(math);
                continue;
            }
            if check_syntax
                && syntax_error.is_none()
                && token.raw == "="
                && function_stack
                    .last()
                    .is_some_and(|&(_, open)| tokens[open].raw.eq_ignore_ascii_case("alpha("))
            {
                syntax_error = Some((token.span, "\")\" is expected"));
            }

            let closed = if token.kind == ValueTokenKind::CloseParen {
                let Some((math, open)) = function_stack.pop() else {
                    if check_syntax {
                        syntax_error.get_or_insert((token.span, "unexpected input"));
                    }
                    continue;
                };
                math_depth -= usize::from(math);
                Some((math, open))
            } else {
                None
            };
            if reported || math_depth == 0 || index + 2 >= tokens.len() {
                continue;
            }

            let operator = &tokens[index + 1];
            let right = &tokens[index + 2];
            if !matches!(operator.raw, "+" | "-") {
                continue;
            }
            let (left_start, left_is_dim, left_is_number) = match closed {
                Some((math, open)) => (
                    tokens[open].span.start,
                    math && tokens[open..index].iter().any(|t| is_dim(&t.kind)),
                    index == open + 4 && is_numeric_random(&tokens, open),
                ),
                None => (
                    token.span.start,
                    is_dim(&token.kind),
                    token.kind == ValueTokenKind::Number,
                ),
            };
            let right_end = if left_is_number && is_dim(&right.kind)
                || left_is_dim && right.kind == ValueTokenKind::Number
            {
                right.span.end
            } else if left_is_dim && is_numeric_random(&tokens, index + 2) {
                tokens[index + 6].span.end
            } else {
                continue;
            };
            reported = true;

            let range = Span {
                start: left_start,
                end: right_end,
            };
            let subject = ctx.slice(range);
            let action = if operator.raw == "+" {
                "added"
            } else {
                "subtracted"
            };
            let reason = format!("a number and a dimension or percentage cannot be {action}");
            ctx.report(
                self.name(),
                range,
                format!("Invalid value \"{subject}\", {reason}"),
            );
        }
        if let Some((span, reason)) = syntax_error {
            ctx.report(
                self.name(),
                span,
                format!("Invalid value \"{value}\", {reason}"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_message, reject_warnings};
    use serde_json::json;

    #[test]
    fn reports_math_before_legacy_alpha_syntax() {
        Tester::new(
            "value-no-invalid",
            json!(true),
            ["a { filter: alpha(0.3); --raw: alpha(opacity=30); }"],
            [
                reject(
                    "a { filter: alpha(opacity=30); }",
                    None,
                    "Invalid value \"alpha(opacity=30)\", \")\" is expected",
                    (1, 26, 1, 27),
                ),
                reject_warnings(
                    "a { filter: alpha(opacity=calc(20px + 10)); }",
                    None,
                    vec![
                        ("Invalid value \"20px + 10\", a number and a dimension or percentage cannot be added".into(), (1, 32, 1, 41)),
                        ("Invalid value \"alpha(opacity=calc(20px + 10))\", \")\" is expected".into(), (1, 26, 1, 27)),
                    ],
                ),
            ],
        )
        .test();
    }

    #[test]
    fn random_with_number_bounds_is_a_number() {
        Tester::new(
            "value-no-invalid",
            json!(true),
            ["a { top: calc(random(10, 50) + 1); }", "a { top: calc(random(1px, 2px) + 1px); }"],
            [
                reject(
                    "a { top: calc(random(10, 50) + 1px); }",
                    None,
                    "Invalid value \"random(10, 50) + 1px\", a number and a dimension or percentage cannot be added",
                    (1, 15, 1, 35),
                ),
                reject_message(
                    "a { top: calc(1px - random(10, 50)); }",
                    None,
                    "Invalid value \"1px - random(10, 50)\", a number and a dimension or percentage cannot be subtracted",
                ),
            ],
        )
        .test();
    }

    #[test]
    fn default_config() {
        let pass = vec![
            "a { width: calc(20px + 10px); }",
            "a { width: 10px; }",
            "a { width: min(20px, 10px); }",
            "a { --value: { color: red; }; }",
        ];
        let fail = vec![
            reject_message(
                "a { width: calc(20px + 10); }",
                None,
                "Invalid value \"20px + 10\", a number and a dimension or percentage cannot be added",
            ),
            reject_message(
                "a { width: calc(10 - 20px); }",
                None,
                "Invalid value \"10 - 20px\", a number and a dimension or percentage cannot be subtracted",
            ),
            reject_message(
                "a { height: calc(max(10px, 5px) + 2 + 3px); }",
                None,
                "Invalid value \"max(10px, 5px) + 2\", a number and a dimension or percentage cannot be added",
            ),
            reject_message(
                "a { width: min(10px + 2, 20px); }",
                None,
                "Invalid value \"10px + 2\", a number and a dimension or percentage cannot be added",
            ),
        ];
        Tester::new("value-no-invalid", json!(true), &pass, &fail).test();
    }
}
