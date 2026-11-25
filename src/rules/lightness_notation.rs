use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{ValueTokenKind, extract_function_name_and_args, format_clean_float, tokenize_value},
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

fn after_function(tokens: &[crate::utils::ValueToken<'_>], open: usize) -> usize {
    let mut depth = 1usize;
    let mut i = open + 1;
    while i < tokens.len() {
        if tokens[i].kind == ValueTokenKind::Function {
            depth += 1;
        } else if tokens[i].kind == ValueTokenKind::CloseParen {
            depth -= 1;
            if depth == 0 {
                return i + 1;
            }
        }
        i += 1;
    }
    tokens.len()
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
        let relative = tokens
            .first()
            .is_some_and(|t| t.kind == ValueTokenKind::Ident && t.raw.eq_ignore_ascii_case("from"));
        let first_index = if relative {
            if tokens
                .get(1)
                .is_some_and(|t| t.kind == ValueTokenKind::Function)
            {
                after_function(&tokens, 1)
            } else {
                2
            }
        } else {
            0
        };
        let Some(first) = tokens.get(first_index) else {
            return;
        };

        match self.mode {
            LightnessMode::Percentage => {
                if first.kind == ValueTokenKind::Number
                    && let Ok(num) = first.raw.parse::<f64>()
                {
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

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn checks_lightness_in_relative_color_functions() {
        Tester::new(
            "lightness-notation",
            json!("percentage"),
            [],
            [reject(
                "a { color: lab(from var(--base) 50 0 0); }",
                Some("a { color: lab(from var(--base) 50% 0 0); }"),
                "Expected \"50\" to be \"50%\"",
                (1, 33, 1, 35),
            )],
        )
        .with_fix(true)
        .test();
        Tester::new(
            "lightness-notation",
            json!("number"),
            [],
            [reject(
                "a { color: oklab(from red 50% 0 0); }",
                Some("a { color: oklab(from red 0.5 0 0); }"),
                "Expected \"50%\" to be \"0.5\"",
                (1, 27, 1, 30),
            )],
        )
        .with_fix(true)
        .test();
    }
}
