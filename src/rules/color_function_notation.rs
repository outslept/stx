use crate::{
    context::LintContext,
    fix::{Edit, Fix},
    rules::{
        Rule,
        value_utils::{ValueTokenKind, extract_function_name_and_args, tokenize_value},
    },
};
use oxc_css_parser::{Span, ast::ComponentValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFunctionNotationMode {
    Modern,
    Legacy,
}

pub struct ColorFunctionNotation {
    pub mode: ColorFunctionNotationMode,
}

impl Default for ColorFunctionNotation {
    fn default() -> Self {
        Self {
            mode: ColorFunctionNotationMode::Modern,
        }
    }
}

impl Rule for ColorFunctionNotation {
    fn name(&self) -> &'static str {
        "color-function-notation"
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        let ComponentValue::Function(func) = val else {
            return;
        };
        let Some((fn_name, name_span, args_span)) =
            extract_function_name_and_args(ctx.source, func.span)
        else {
            return;
        };
        let lower = fn_name.to_ascii_lowercase();
        if !matches!(lower.as_str(), "rgb" | "rgba" | "hsl" | "hsla") {
            return;
        }

        let tokens = tokenize_value(ctx.source, args_span);
        let mut depth = 0usize;
        let mut comma_spans = Vec::new();
        let mut has_var_or_scss = false;

        for tok in &tokens {
            match tok.kind {
                ValueTokenKind::Function => {
                    if tok.raw.eq_ignore_ascii_case("var(") {
                        has_var_or_scss = true;
                    }
                    depth += 1;
                }
                ValueTokenKind::CloseParen => depth = depth.saturating_sub(1),
                ValueTokenKind::Comma if depth == 0 => comma_spans.push(tok.span),
                ValueTokenKind::Ident if tok.raw.starts_with('$') => has_var_or_scss = true,
                _ => {}
            }
        }

        if self.mode == ColorFunctionNotationMode::Modern {
            if has_var_or_scss || !matches!(comma_spans.len(), 2 | 3) {
                return;
            }

            let mut edits = Vec::new();
            if lower == "rgba" || lower == "hsla" {
                edits.push(Edit {
                    span: name_span,
                    replacement: fn_name[..3].to_string(),
                });
            }

            let bytes = ctx.source.as_bytes();
            let args_start = args_span.start as usize;
            let args_end = args_span.end as usize;
            for (i, c_span) in comma_spans.iter().enumerate() {
                let mut start = c_span.start as usize;
                while start > args_start && bytes[start - 1].is_ascii_whitespace() {
                    start -= 1;
                }
                let mut end = c_span.end as usize;
                while end < args_end && bytes[end].is_ascii_whitespace() {
                    end += 1;
                }
                let replacement = if i == 2 { " / " } else { " " };
                edits.push(Edit {
                    span: Span {
                        start: start as u32,
                        end: end as u32,
                    },
                    replacement: replacement.to_string(),
                });
            }

            ctx.report_with_fix(
                self.name(),
                func.span,
                "Expected modern color-function notation".to_string(),
                Some(Fix { edits }),
            );
        }
    }
}
