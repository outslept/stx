use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

pub struct CustomPropertyNoMissingVarFunction;

impl Rule for CustomPropertyNoMissingVarFunction {
    fn name(&self) -> &'static str {
        "custom-property-no-missing-var-function"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let prop_Span_end = if let InterpolableIdent::Literal(ident) = &decl.name {
            let prop_name: &str = ident.name;
            if prop_name.eq_ignore_ascii_case("transition-property")
                || prop_name.eq_ignore_ascii_case("will-change")
            {
                return;
            }
            ident.span.end as usize
        } else {
            return;
        };

        let end = decl.span.end as usize;
        let src = ctx.source;
        if prop_Span_end >= end || end > src.len() {
            return;
        }

        let bytes = src.as_bytes();
        let mut i = prop_Span_end;

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

            if i + 4 <= end
                && (src[i..i + 4].eq_ignore_ascii_case("var(")
                    || src[i..i + 4].eq_ignore_ascii_case("env(")
                    || src[i..i + 4].eq_ignore_ascii_case("url("))
            {
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

            if i + 2 < end
                && bytes[i] == b'-'
                && bytes[i + 1] == b'-'
                && (bytes[i + 2].is_ascii_alphabetic() || bytes[i + 2] == b'_')
            {
                let start = i;
                i += 2;
                while i < end
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
                {
                    i += 1;
                }
                if i < end && bytes[i] == b'(' {
                    continue;
                }
                let ident_text = &src[start..i];
                ctx.report(
                    self.name(),
                    Span {
                        start: start as u32,
                        end: i as u32,
                    },
                    format!("Unexpected missing var function for \"{ident_text}\""),
                );
                continue;
            }

            i += 1;
        }
    }
}
