use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::ComponentValue;

pub struct ColorNoInvalidHex;

impl Rule for ColorNoInvalidHex {
    fn name(&self) -> &'static str {
        "color-no-invalid-hex"
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        if let ComponentValue::HexColor(hex) = val {
            let len = hex.value.len();
            let valid_len = matches!(len, 3 | 4 | 6 | 8);
            let valid_chars = hex.value.bytes().all(|b| b.is_ascii_hexdigit());

            if !valid_len || !valid_chars {
                ctx.report(
                    self.name(),
                    hex.span,
                    format!("Unexpected invalid hex color \"#{}\"", hex.raw),
                );
            }
        }
    }
}
