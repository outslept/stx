use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        value_utils::{ValueTokenKind, declaration_value_span, tokenize_value},
    },
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

pub struct LengthZeroNoUnit;

const LENGTH_UNITS: &[&str] = &[
    "px", "em", "rem", "ex", "ch", "cap", "ic", "lh", "rlh", "vw", "vh", "vi", "vb", "vmin",
    "vmax", "svw", "svh", "svi", "svb", "svmin", "svmax", "lvw", "lvh", "lvi", "lvb", "lvmin",
    "lvmax", "dvw", "dvh", "dvi", "dvb", "dvmin", "dvmax", "cqw", "cqh", "cqi", "cqb", "cqmin",
    "cqmax", "cm", "mm", "q", "in", "pc", "pt",
];

impl Rule for LengthZeroNoUnit {
    fn name(&self) -> &'static str {
        "length-zero-no-unit"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if prop.starts_with("--") {
            return;
        }

        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };

        let tokens = tokenize_value(ctx.source, val_span);
        for tok in tokens {
            if let ValueTokenKind::Dimension { num_len } = tok.kind {
                let num_str = &tok.raw[..num_len];
                let unit_str = &tok.raw[num_len..];
                if !LENGTH_UNITS
                    .iter()
                    .any(|u| u.eq_ignore_ascii_case(unit_str))
                {
                    continue;
                }
                if let Ok(val) = num_str.parse::<f64>() {
                    if val == 0.0 {
                        let unit_span = Span {
                            start: tok.span.start + num_len as u32,
                            end: tok.span.end,
                        };
                        ctx.report_with_fix(
                            self.name(),
                            unit_span,
                            "Unexpected unit".to_string(),
                            Some(Fix::single(tok.span, "0".to_string())),
                        );
                    }
                }
            }
        }
    }
}
