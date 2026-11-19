use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        value_utils::{
            ValueTokenKind, extract_function_name_and_args, format_clean_float, tokenize_value,
        },
    },
};
use oxc_css_parser::ast::ComponentValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightnessMode {
    Percentage,
    Number,
}

pub struct LightnessNotation {
    pub mode: LightnessMode,
}

impl Default for LightnessNotation {
    fn default() -> Self {
        Self {
            mode: LightnessMode::Percentage,
        }
    }
}

impl Rule for LightnessNotation {
    fn name(&self) -> &'static str {
        "lightness-notation"
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        let ComponentValue::Function(func) = val else {
            return;
        };
        let Some((fn_name, _, args_span)) = extract_function_name_and_args(ctx.source, func.span)
        else {
            return;
        };
        let lower = fn_name.to_ascii_lowercase();
        let is_ok = match lower.as_str() {
            "lab" | "lch" => false,
            "oklab" | "oklch" => true,
            _ => return,
        };

        let tokens = tokenize_value(ctx.source, args_span);
        let Some(first) = tokens.first() else {
            return;
        };
        if first.kind == ValueTokenKind::Ident && first.raw.eq_ignore_ascii_case("from") {
            return;
        }

        match self.mode {
            LightnessMode::Percentage => {
                if first.kind == ValueTokenKind::Number {
                    if let Ok(num) = first.raw.parse::<f64>() {
                        let pct = if is_ok { num * 100.0 } else { num };
                        let expected = format!("{}%", format_clean_float(pct));
                        ctx.report_with_fix(
                            self.name(),
                            first.span,
                            format!("Expected \"{}\" to be \"{}\"", first.raw, expected),
                            Some(Fix::single(first.span, expected)),
                        );
                    }
                }
            }
            LightnessMode::Number => {
                if first.kind == ValueTokenKind::Percentage {
                    let num_part = &first.raw[..first.raw.len() - 1];
                    if let Ok(pct) = num_part.parse::<f64>() {
                        let num = if is_ok { pct / 100.0 } else { pct };
                        let expected = format_clean_float(num);
                        ctx.report_with_fix(
                            self.name(),
                            first.span,
                            format!("Expected \"{}\" to be \"{}\"", first.raw, expected),
                            Some(Fix::single(first.span, expected)),
                        );
                    }
                }
            }
        }
    }
}
