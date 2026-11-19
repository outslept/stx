use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        value_utils::{
            ValueToken, ValueTokenKind, declaration_value_span, extract_function_name_and_args,
            format_clean_float, tokenize_value,
        },
    },
};
use oxc_css_parser::ast::{ComponentValue, Declaration, InterpolableIdent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaNotationMode {
    Percentage,
    Number,
}

pub struct AlphaValueNotation {
    pub mode: AlphaNotationMode,
}

impl Default for AlphaValueNotation {
    fn default() -> Self {
        Self {
            mode: AlphaNotationMode::Percentage,
        }
    }
}

const ALPHA_PROPERTIES: &[&str] = &[
    "opacity",
    "fill-opacity",
    "flood-opacity",
    "stop-opacity",
    "stroke-opacity",
    "shape-image-threshold",
];

const COLOR_FUNCTIONS: &[&str] = &[
    "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch", "color",
];

impl AlphaValueNotation {
    fn check_token(&self, token: &ValueToken<'_>, ctx: &mut LintContext<'_>) {
        match self.mode {
            AlphaNotationMode::Percentage => {
                if token.kind == ValueTokenKind::Number {
                    if let Ok(val) = token.raw.parse::<f64>() {
                        if (0.0..=1.0).contains(&val) {
                            let expected = format!("{}%", format_clean_float(val * 100.0));
                            ctx.report_with_fix(
                                self.name(),
                                token.span,
                                format!("Expected \"{}\" to be \"{}\"", token.raw, expected),
                                Some(Fix::single(token.span, expected)),
                            );
                        }
                    }
                }
            }
            AlphaNotationMode::Number => {
                if token.kind == ValueTokenKind::Percentage {
                    let num_part = &token.raw[..token.raw.len() - 1];
                    if let Ok(val) = num_part.parse::<f64>() {
                        let expected = format_clean_float(val / 100.0);
                        ctx.report_with_fix(
                            self.name(),
                            token.span,
                            format!("Expected \"{}\" to be \"{}\"", token.raw, expected),
                            Some(Fix::single(token.span, expected)),
                        );
                    }
                }
            }
        }
    }
}

impl Rule for AlphaValueNotation {
    fn name(&self) -> &'static str {
        "alpha-value-notation"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if !ALPHA_PROPERTIES
            .iter()
            .any(|p| p.eq_ignore_ascii_case(prop))
        {
            return;
        }

        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };

        let tokens = tokenize_value(ctx.source, val_span);
        if tokens.len() == 1 {
            self.check_token(&tokens[0], ctx);
        }
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        let ComponentValue::Function(func) = val else {
            return;
        };
        let Some((fn_name, _, args_span)) = extract_function_name_and_args(ctx.source, func.span)
        else {
            return;
        };
        if !COLOR_FUNCTIONS
            .iter()
            .any(|f| f.eq_ignore_ascii_case(fn_name))
        {
            return;
        }

        let tokens = tokenize_value(ctx.source, args_span);
        let mut depth = 0usize;
        let mut slash_idx = None;
        let mut comma_indices = Vec::new();

        for (idx, tok) in tokens.iter().enumerate() {
            match tok.kind {
                ValueTokenKind::Function => depth += 1,
                ValueTokenKind::CloseParen => depth = depth.saturating_sub(1),
                ValueTokenKind::Slash if depth == 0 => slash_idx = Some(idx),
                ValueTokenKind::Comma if depth == 0 => comma_indices.push(idx),
                _ => {}
            }
        }

        if let Some(s_idx) = slash_idx {
            let after = &tokens[s_idx + 1..];
            if after.len() == 1 {
                self.check_token(&after[0], ctx);
            }
        } else if comma_indices.len() == 3
            && (fn_name.eq_ignore_ascii_case("rgb")
                || fn_name.eq_ignore_ascii_case("rgba")
                || fn_name.eq_ignore_ascii_case("hsl")
                || fn_name.eq_ignore_ascii_case("hsla"))
        {
            let after = &tokens[comma_indices[2] + 1..];
            if after.len() == 1 {
                self.check_token(&after[0], ctx);
            }
        }
    }
}
