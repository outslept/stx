use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{ValueTokenKind, extract_function_name_and_args, tokenize_value},
};
use oxc_css_parser::ast::ComponentValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HueDegreeMode {
    Angle,
    Number,
}

crate::define_stx_lint! {
    /// Enforces unitless or degree notation for hue values in CSS color functions.
    ///
    /// Set the primary option to `"angle"` to add `deg` to a bare hue number,
    /// or `"number"` to remove `deg`. Auto-fix leaves other angle units alone.
    /// The rule checks `hsl()`, `hsla()`, `hwb()`, `lch()`, and `oklch()`,
    /// including relative color forms.
    ///
    /// ```css
    /// a { color: hsl(180 50% 50%); } /* fixes to hsl(180deg 50% 50%) with "angle" */
    /// ```
    "hue-degree-notation", style, fix = fix;
    pub struct HueDegreeNotation {
        pub mode: HueDegreeMode,
    }
}

impl Default for HueDegreeNotation {
    fn default() -> Self {
        Self {
            mode: HueDegreeMode::Angle,
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

impl Rule for HueDegreeNotation {
    fn name(&self) -> &'static str {
        Self::NAME
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
        let relative = tokens
            .first()
            .is_some_and(|t| t.kind == ValueTokenKind::Ident && t.raw.eq_ignore_ascii_case("from"));
        let start = if relative {
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

        let mut top_args = Vec::new();
        let mut depth = 0usize;
        for tok in tokens.iter().skip(start) {
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

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn checks_hues_in_relative_color_functions() {
        Tester::new(
            "hue-degree-notation",
            json!("angle"),
            [],
            [
                reject(
                    "a { color: hsl(from var(--base) 180 50% 50%); }",
                    Some("a { color: hsl(from var(--base) 180deg 50% 50%); }"),
                    "Expected \"180\" to be \"180deg\"",
                    (1, 33, 1, 36),
                ),
                reject(
                    "a { color: oklch(from red 50% .2 30); }",
                    Some("a { color: oklch(from red 50% .2 30deg); }"),
                    "Expected \"30\" to be \"30deg\"",
                    (1, 34, 1, 36),
                ),
            ],
        )
        .with_fix(true)
        .test();
    }
}
