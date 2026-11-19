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

pub struct ValueNoVendorPrefix;

const PREFIXED_VALUES: &[&str] = &[
    "-webkit-box",
    "-moz-box",
    "-webkit-inline-box",
    "-moz-inline-box",
    "-webkit-flex",
    "-ms-flexbox",
    "-webkit-inline-flex",
    "-ms-inline-flexbox",
    "-webkit-grab",
    "-moz-grab",
    "-webkit-grabbing",
    "-moz-grabbing",
    "-webkit-sticky",
    "-webkit-fit-content",
    "-moz-fit-content",
    "-webkit-max-content",
    "-moz-max-content",
    "-webkit-min-content",
    "-moz-min-content",
    "-webkit-fill-available",
    "-moz-available",
    "-webkit-isolate",
    "-moz-isolate",
    "-webkit-plaintext",
    "-moz-plaintext",
];

const PREFIXED_FUNCTIONS: &[&str] = &[
    "-webkit-linear-gradient",
    "-moz-linear-gradient",
    "-o-linear-gradient",
    "-ms-linear-gradient",
    "-webkit-radial-gradient",
    "-moz-radial-gradient",
    "-o-radial-gradient",
    "-ms-radial-gradient",
    "-webkit-repeating-linear-gradient",
    "-moz-repeating-linear-gradient",
    "-o-repeating-linear-gradient",
    "-ms-repeating-linear-gradient",
    "-webkit-repeating-radial-gradient",
    "-moz-repeating-radial-gradient",
    "-o-repeating-radial-gradient",
    "-ms-repeating-radial-gradient",
    "-webkit-calc",
    "-moz-calc",
    "-webkit-image-set",
];

fn strip_vendor_prefix(s: &str) -> Option<&str> {
    let lower = s.to_ascii_lowercase();
    for prefix in ["-webkit-", "-moz-", "-ms-", "-o-"] {
        if lower.starts_with(prefix) {
            return Some(&s[prefix.len()..]);
        }
    }
    None
}

impl Rule for ValueNoVendorPrefix {
    fn name(&self) -> &'static str {
        "value-no-vendor-prefix"
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
            match tok.kind {
                ValueTokenKind::Ident => {
                    if PREFIXED_VALUES
                        .iter()
                        .any(|v| v.eq_ignore_ascii_case(tok.raw))
                    {
                        if let Some(unprefixed) = strip_vendor_prefix(tok.raw) {
                            ctx.report_with_fix(
                                self.name(),
                                tok.span,
                                format!("Unexpected vendor-prefix \"{}\"", tok.raw),
                                Some(Fix::single(tok.span, unprefixed.to_string())),
                            );
                        }
                    }
                }
                ValueTokenKind::Function => {
                    let fn_name = &tok.raw[..tok.raw.len() - 1];
                    if PREFIXED_FUNCTIONS
                        .iter()
                        .any(|f| f.eq_ignore_ascii_case(fn_name))
                    {
                        if let Some(unprefixed) = strip_vendor_prefix(fn_name) {
                            let fn_span = Span {
                                start: tok.span.start,
                                end: tok.span.end - 1,
                            };
                            ctx.report_with_fix(
                                self.name(),
                                fn_span,
                                format!("Unexpected vendor-prefix \"{}\"", fn_name),
                                Some(Fix::single(fn_span, unprefixed.to_string())),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
