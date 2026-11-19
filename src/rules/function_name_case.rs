use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, value_utils::extract_function_name_and_args},
};
use oxc_css_parser::ast::ComponentValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMode {
    Lower,
    Upper,
}

pub struct FunctionNameCase {
    pub mode: CaseMode,
}

impl Default for FunctionNameCase {
    fn default() -> Self {
        Self {
            mode: CaseMode::Lower,
        }
    }
}

impl Rule for FunctionNameCase {
    fn name(&self) -> &'static str {
        "function-name-case"
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        let ComponentValue::Function(func) = val else {
            return;
        };
        let Some((fn_name, name_span, _)) = extract_function_name_and_args(ctx.source, func.span)
        else {
            return;
        };

        let expected = match self.mode {
            CaseMode::Lower => fn_name.to_ascii_lowercase(),
            CaseMode::Upper => fn_name.to_ascii_uppercase(),
        };

        if fn_name != expected {
            ctx.report_with_fix(
                self.name(),
                name_span,
                format!("Expected \"{}\" to be \"{}\"", fn_name, expected),
                Some(Fix::single(name_span, expected)),
            );
        }
    }
}
