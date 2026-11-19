use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::ComponentValue};

pub struct FunctionLinearGradientNoNonstandardDirection;

fn is_valid_direction(dir: &str) -> bool {
    let lower = dir.trim().to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("to ") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        return match parts.as_slice() {
            ["top" | "bottom" | "left" | "right"] => true,
            ["top" | "bottom", "left" | "right"] => true,
            ["left" | "right", "top" | "bottom"] => true,
            _ => false,
        };
    }

    for unit in ["deg", "grad", "rad", "turn"] {
        if let Some(num_str) = lower.strip_suffix(unit) {
            return num_str.trim().parse::<f64>().is_ok();
        }
    }

    false
}

fn looks_like_direction(arg: &str) -> bool {
    let lower = arg.trim().to_ascii_lowercase();
    if lower.starts_with("to ")
        || lower.starts_with("top")
        || lower.starts_with("bottom")
        || lower.starts_with("left")
        || lower.starts_with("right")
    {
        return true;
    }

    if lower
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c == '-' || c == '+' || c == '.')
    {
        return true;
    }

    false
}

impl Rule for FunctionLinearGradientNoNonstandardDirection {
    fn name(&self) -> &'static str {
        "function-linear-gradient-no-nonstandard-direction"
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        if let ComponentValue::Function(func) = val {
            let raw = ctx.slice(func.span);
            let Some(paren_idx) = raw.find('(') else {
                return;
            };
            let func_name = raw[..paren_idx].trim();
            if !func_name.eq_ignore_ascii_case("linear-gradient")
                && !func_name.eq_ignore_ascii_case("repeating-linear-gradient")
            {
                return;
            }

            let inner_start = func.span.start as usize + paren_idx + 1;
            let inner_end = (func.span.end as usize).saturating_sub(1);
            if inner_start >= inner_end || inner_end > ctx.source.len() {
                return;
            }

            let inner = &ctx.source[inner_start..inner_end];
            let Some(comma_idx) = inner.find(',') else {
                return;
            };

            let first_arg = &inner[..comma_idx];
            let trimmed = first_arg.trim();
            if trimmed.is_empty() || !looks_like_direction(trimmed) {
                return;
            }

            if !is_valid_direction(trimmed) {
                let leading_ws = first_arg.len() - first_arg.trim_start().len();
                let start = (inner_start + leading_ws) as u32;
                let end = start + trimmed.len() as u32;
                ctx.report(
                    self.name(),
                    Span { start, end },
                    format!("Unexpected nonstandard direction \"{trimmed}\""),
                );
            }
        }
    }
}
