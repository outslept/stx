use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        value_utils::{ValueTokenKind, extract_function_name_and_args, tokenize_value},
    },
};
use oxc_css_parser::ast::ComponentValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HueDegreeMode {
    Angle,
    Number,
}

pub struct HueDegreeNotation {
    pub mode: HueDegreeMode,
}

impl Default for HueDegreeNotation {
    fn default() -> Self {
        Self {
            mode: HueDegreeMode::Angle,
        }
    }
}

impl Rule for HueDegreeNotation {
    fn name(&self) -> &'static str {
        "hue-degree-notation"
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

        let hue_arg_index = match lower.as_str() {
            "hsl" | "hsla" | "hwb" => 0usize,
            "lch" | "oklch" => 2usize,
            _ => return,
        };

        let tokens = tokenize_value(ctx.source, args_span);
        if tokens
            .first()
            .is_some_and(|t| t.kind == ValueTokenKind::Ident && t.raw.eq_ignore_ascii_case("from"))
        {
            return;
        }

        let mut top_args = Vec::new();
        let mut depth = 0usize;
        for tok in &tokens {
            match tok.kind {
                ValueTokenKind::Function => depth += 1,
                ValueTokenKind::CloseParen => depth = depth.saturating_sub(1),
                ValueTokenKind::Comma | ValueTokenKind::Slash if depth == 0 => {}
                _ if depth == 0 => top_args.push(tok),
                _ => {}
            }
        }

        let Some(hue_tok) = top_args.get(hue_arg_index) else {
            return;
        };

        match self.mode {
            HueDegreeMode::Angle => {
                if hue_tok.kind == ValueTokenKind::Number {
                    let expected = format!("{}deg", hue_tok.raw);
                    ctx.report_with_fix(
                        self.name(),
                        hue_tok.span,
                        format!("Expected \"{}\" to be \"{}\"", hue_tok.raw, expected),
                        Some(Fix::single(hue_tok.span, expected)),
                    );
                }
            }
            HueDegreeMode::Number => {
                if let ValueTokenKind::Dimension { num_len } = hue_tok.kind {
                    let unit = &hue_tok.raw[num_len..];
                    if unit.eq_ignore_ascii_case("deg") {
                        let expected = &hue_tok.raw[..num_len];
                        ctx.report_with_fix(
                            self.name(),
                            hue_tok.span,
                            format!("Expected \"{}\" to be \"{}\"", hue_tok.raw, expected),
                            Some(Fix::single(hue_tok.span, expected.to_string())),
                        );
                    }
                }
            }
        }
    }
}
