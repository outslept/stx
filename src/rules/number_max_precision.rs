use crate::{
    context::LintContext,
    rules::{
        Rule,
        value_utils::{ValueTokenKind, declaration_value_span, tokenize_value},
    },
};
use oxc_css_parser::ast::{Declaration, InterpolableIdent};

pub struct NumberMaxPrecision {
    pub max: usize,
}

impl Default for NumberMaxPrecision {
    fn default() -> Self {
        Self { max: 4 }
    }
}

impl NumberMaxPrecision {
    fn check_number_str(&self, num_str: &str) -> Option<String> {
        let mantissa = match num_str.find(['e', 'E']) {
            Some(idx) => &num_str[..idx],
            None => num_str,
        };
        let dot_idx = mantissa.find('.')?;
        let decimals = &mantissa[dot_idx + 1..];
        if decimals.len() <= self.max {
            return None;
        }
        let val = num_str.parse::<f64>().ok()?;
        let factor = 10f64.powi(self.max as i32);
        let rounded = (val * factor).round() / factor;
        let formatted = format!("{:.prec$}", rounded, prec = self.max);
        let cleaned = if self.max > 0 {
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            formatted
        };
        Some(cleaned)
    }
}

impl Rule for NumberMaxPrecision {
    fn name(&self) -> &'static str {
        "number-max-precision"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };

        let tokens = tokenize_value(ctx.source, val_span);
        for tok in tokens {
            let num_str = match tok.kind {
                ValueTokenKind::Number => tok.raw,
                ValueTokenKind::Percentage => &tok.raw[..tok.raw.len() - 1],
                ValueTokenKind::Dimension { num_len } => &tok.raw[..num_len],
                _ => continue,
            };

            if let Some(expected) = self.check_number_str(num_str) {
                let raw_clean = num_str.trim_start_matches('+');
                ctx.report(
                    self.name(),
                    tok.span,
                    format!("Expected \"{}\" to be \"{}\"", raw_clean, expected),
                );
            }
        }
    }
}
