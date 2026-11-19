use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, value_utils::extract_function_name_and_args},
};
use oxc_css_parser::ast::ComponentValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFunctionAliasMode {
    WithoutAlpha,
    WithAlpha,
}

pub struct ColorFunctionAliasNotation {
    pub mode: ColorFunctionAliasMode,
}

impl Default for ColorFunctionAliasNotation {
    fn default() -> Self {
        Self {
            mode: ColorFunctionAliasMode::WithoutAlpha,
        }
    }
}

impl Rule for ColorFunctionAliasNotation {
    fn name(&self) -> &'static str {
        "color-function-alias-notation"
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        let ComponentValue::Function(func) = val else {
            return;
        };
        let Some((fn_name, name_span, _)) = extract_function_name_and_args(ctx.source, func.span)
        else {
            return;
        };
        let lower = fn_name.to_ascii_lowercase();

        match self.mode {
            ColorFunctionAliasMode::WithoutAlpha => {
                if lower == "rgba" || lower == "hsla" {
                    let expected = &fn_name[..3];
                    ctx.report_with_fix(
                        self.name(),
                        name_span,
                        format!("Expected \"{}\" to be \"{}\"", fn_name, expected),
                        Some(Fix::single(name_span, expected.to_string())),
                    );
                }
            }
            ColorFunctionAliasMode::WithAlpha => {
                if lower == "rgb" || lower == "hsl" {
                    let suffix = if fn_name.chars().all(|c| c.is_ascii_uppercase()) {
                        "A"
                    } else {
                        "a"
                    };
                    let expected = format!("{}{}", fn_name, suffix);
                    ctx.report_with_fix(
                        self.name(),
                        name_span,
                        format!("Expected \"{}\" to be \"{}\"", fn_name, expected),
                        Some(Fix::single(name_span, expected)),
                    );
                }
            }
        }
    }
}
