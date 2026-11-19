use crate::{context::LintContext, fix::Fix, rules::Rule};
use oxc_css_parser::ast::ComponentValue;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexLengthMode {
    Short,
    Long,
}

pub struct ColorHexLength {
    pub mode: HexLengthMode,
}

impl ColorHexLength {
    fn can_shorten(bytes: &[u8]) -> bool {
        match bytes.len() {
            6 => {
                bytes[0].eq_ignore_ascii_case(&bytes[1])
                    && bytes[2].eq_ignore_ascii_case(&bytes[3])
                    && bytes[4].eq_ignore_ascii_case(&bytes[5])
            }
            8 => {
                bytes[0].eq_ignore_ascii_case(&bytes[1])
                    && bytes[2].eq_ignore_ascii_case(&bytes[3])
                    && bytes[4].eq_ignore_ascii_case(&bytes[5])
                    && bytes[6].eq_ignore_ascii_case(&bytes[7])
            }
            _ => false,
        }
    }

    fn shorten(raw: &str) -> String {
        let b = raw.as_bytes();
        let mut out = String::with_capacity(5);
        out.push('#');
        out.push(b[0] as char);
        out.push(b[2] as char);
        out.push(b[4] as char);
        if b.len() == 8 {
            out.push(b[6] as char);
        }
        out
    }

    fn lengthen(raw: &str) -> String {
        let b = raw.as_bytes();
        let mut out = String::with_capacity(9);
        out.push('#');
        for &ch in b {
            out.push(ch as char);
            out.push(ch as char);
        }
        out
    }
}

impl Rule for ColorHexLength {
    fn name(&self) -> &'static str {
        "color-hex-length"
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        let ComponentValue::HexColor(hex) = val else {
            return;
        };

        let bytes = hex.raw.as_bytes();
        if !bytes.iter().all(|b| b.is_ascii_hexdigit()) {
            return;
        }

        match self.mode {
            HexLengthMode::Short => {
                if Self::can_shorten(bytes) {
                    let expected = Self::shorten(hex.raw);
                    ctx.report_with_fix(
                        self.name(),
                        hex.span,
                        format!("Expected \"#{}\" to be \"{}\"", hex.raw, expected),
                        Some(Fix::single(hex.span, expected)),
                    );
                }
            }
            HexLengthMode::Long => {
                if matches!(bytes.len(), 3 | 4) {
                    let expected = Self::lengthen(hex.raw);
                    ctx.report_with_fix(
                        self.name(),
                        hex.span,
                        format!("Expected \"#{}\" to be \"{}\"", hex.raw, expected),
                        Some(Fix::single(hex.span, expected)),
                    );
                }
            }
        }
    }
}
