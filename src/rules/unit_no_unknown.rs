use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

pub struct UnitNoUnknown;

const KNOWN_UNITS: &[&str] = &[
    "%", "cap", "ch", "cm", "cqb", "cqh", "cqi", "cqmax", "cqmin", "cqw", "deg", "dpcm", "dpi",
    "dppx", "dvb", "dvh", "dvi", "dvmax", "dvmin", "dvw", "em", "ex", "fr", "grad", "hz", "ic",
    "in", "khz", "lh", "lvb", "lvh", "lvi", "lvmax", "lvmin", "lvw", "mm", "ms", "pc", "pt", "px",
    "q", "rad", "rcap", "rch", "rem", "rex", "ric", "rlh", "s", "svb", "svh", "svi", "svmax",
    "svmin", "svw", "turn", "vb", "vh", "vi", "vmax", "vmin", "vw", "x",
];

impl Rule for UnitNoUnknown {
    fn name(&self) -> &'static str {
        "unit-no-unknown"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let prop_end = if let InterpolableIdent::Literal(ident) = &decl.name {
            ident.span.end as usize
        } else {
            return;
        };

        let end = decl.span.end as usize;
        let src = ctx.source;
        if prop_end >= end || end > src.len() {
            return;
        }

        let bytes = src.as_bytes();
        let mut i = prop_end;

        while i < end && bytes[i] != b':' {
            i += 1;
        }
        if i < end && bytes[i] == b':' {
            i += 1;
        }

        while i < end {
            if bytes[i] == b'"' || bytes[i] == b'\'' {
                let quote = bytes[i];
                i += 1;
                while i < end {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }

            if i + 4 <= end && src[i..i + 4].eq_ignore_ascii_case("url(") {
                i += 4;
                let mut depth = 1usize;
                while i < end && depth > 0 {
                    if bytes[i] == b'(' {
                        depth += 1;
                    } else if bytes[i] == b')' {
                        depth -= 1;
                    }
                    i += 1;
                }
                continue;
            }

            if bytes[i] == b'#' {
                i += 1;
                while i < end && bytes[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                continue;
            }

            if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' || bytes[i] == b'-' {
                while i < end
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
                {
                    i += 1;
                }
                continue;
            }

            if bytes[i].is_ascii_digit() {
                while i < end && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i < end && bytes[i] == b'.' && i + 1 < end && bytes[i + 1].is_ascii_digit() {
                    i += 1;
                    while i < end && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                if i < end
                    && (bytes[i] == b'e' || bytes[i] == b'E')
                    && i + 1 < end
                    && (bytes[i + 1].is_ascii_digit()
                        || ((bytes[i + 1] == b'+' || bytes[i + 1] == b'-')
                            && i + 2 < end
                            && bytes[i + 2].is_ascii_digit()))
                {
                    i += 2;
                    while i < end && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }

                if i < end && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'%') {
                    let unit_start = i;
                    if bytes[i] == b'%' {
                        i += 1;
                    } else {
                        while i < end
                            && (bytes[i].is_ascii_alphanumeric()
                                || bytes[i] == b'-'
                                || bytes[i] == b'_')
                        {
                            i += 1;
                        }
                    }
                    let unit_raw = &src[unit_start..i];
                    let unit_lower = unit_raw.to_ascii_lowercase();
                    if !KNOWN_UNITS.contains(&unit_lower.as_str()) {
                        ctx.report(
                            self.name(),
                            Span {
                                start: unit_start as u32,
                                end: i as u32,
                            },
                            format!("Unexpected unknown unit \"{unit_raw}\""),
                        );
                    }
                }
                continue;
            }

            i += 1;
        }
    }
}
