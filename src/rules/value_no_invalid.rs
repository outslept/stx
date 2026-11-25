use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        ValueToken, ValueTokenKind, decl_value_span, is_standard_syntax_declaration,
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

impl Rule for ValueNoInvalid {
    fn name(&self) -> &'static str {
        "value-no-invalid"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if !is_standard_syntax_declaration(decl) {
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
        let mut function_stack = Vec::new();
        let mut math_depth = 0usize;
        for (index, token) in tokens.iter().enumerate() {
            if token.kind == ValueTokenKind::Function {
                let math = is_math_function(token);
                function_stack.push(math);
                math_depth += usize::from(math);
                continue;
            }
            if token.kind == ValueTokenKind::CloseParen {
                if let Some(math) = function_stack.pop() {
                    math_depth -= usize::from(math);
                } else {
                    ctx.report(
                        self.name(),
                        token.span,
                        "Invalid value \")\", unexpected input",
                    );
                }
                continue;
            }
            if math_depth == 0 || index + 2 >= tokens.len() {
                continue;
            }

            let left = token;
            let operator = &tokens[index + 1];
            let right = &tokens[index + 2];
            if !matches!(operator.raw, "+" | "-") {
                continue;
            }
            let invalid = (is_dim(&left.kind) && right.kind == ValueTokenKind::Number)
                || (left.kind == ValueTokenKind::Number && is_dim(&right.kind));
            if !invalid {
                continue;
            }

            let range = Span {
                start: left.span.start,
                end: right.span.end,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject_message};
    use serde_json::json;

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
                "a { width: min(10px + 2, 20px); }",
                None,
                "Invalid value \"10px + 2\", a number and a dimension or percentage cannot be added",
            ),
        ];
        Tester::new("value-no-invalid", json!(true), &pass, &fail).test();
    }
}
