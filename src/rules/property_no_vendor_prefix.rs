use crate::{context::LintContext, fix::Fix, rules::Rule};
use oxc_css_parser::ast::{Declaration, InterpolableIdent};

pub struct PropertyNoVendorPrefix;

const PREFIXED_PROPERTIES: &[&str] = &[
    "-webkit-align-content",
    "-ms-flex-line-pack",
    "-webkit-align-items",
    "-webkit-box-align",
    "-ms-flex-align",
    "-webkit-align-self",
    "-ms-flex-item-align",
    "-webkit-animation",
    "-moz-animation",
    "-o-animation",
    "-webkit-animation-delay",
    "-moz-animation-delay",
    "-o-animation-delay",
    "-webkit-animation-direction",
    "-moz-animation-direction",
    "-o-animation-direction",
    "-webkit-animation-duration",
    "-moz-animation-duration",
    "-o-animation-duration",
    "-webkit-animation-fill-mode",
    "-moz-animation-fill-mode",
    "-o-animation-fill-mode",
    "-webkit-animation-iteration-count",
    "-moz-animation-iteration-count",
    "-o-animation-iteration-count",
    "-webkit-animation-name",
    "-moz-animation-name",
    "-o-animation-name",
    "-webkit-animation-play-state",
    "-moz-animation-play-state",
    "-o-animation-play-state",
    "-webkit-animation-timing-function",
    "-moz-animation-timing-function",
    "-o-animation-timing-function",
    "-webkit-appearance",
    "-moz-appearance",
    "-webkit-backdrop-filter",
    "-webkit-backface-visibility",
    "-moz-backface-visibility",
    "-webkit-background-clip",
    "-moz-background-clip",
    "-webkit-background-origin",
    "-moz-background-origin",
    "-webkit-background-size",
    "-moz-background-size",
    "-o-background-size",
    "-webkit-box-Decoration-break",
    "-webkit-box-shadow",
    "-moz-box-shadow",
    "-webkit-box-sizing",
    "-moz-box-sizing",
    "-webkit-clip-path",
    "-webkit-column-count",
    "-moz-column-count",
    "-webkit-column-fill",
    "-moz-column-fill",
    "-webkit-column-gap",
    "-moz-column-gap",
    "-webkit-column-rule",
    "-moz-column-rule",
    "-webkit-column-rule-color",
    "-moz-column-rule-color",
    "-webkit-column-rule-style",
    "-moz-column-rule-style",
    "-webkit-column-rule-width",
    "-moz-column-rule-width",
    "-webkit-column-span",
    "-moz-column-span",
    "-webkit-column-width",
    "-moz-column-width",
    "-webkit-columns",
    "-moz-columns",
    "-webkit-filter",
    "-webkit-flex",
    "-ms-flex",
    "-webkit-flex-basis",
    "-ms-flex-preferred-size",
    "-webkit-flex-direction",
    "-ms-flex-direction",
    "-webkit-flex-flow",
    "-ms-flex-flow",
    "-webkit-flex-grow",
    "-ms-flex-positive",
    "-webkit-flex-shrink",
    "-ms-flex-negative",
    "-webkit-flex-wrap",
    "-ms-flex-wrap",
    "-webkit-font-feature-settings",
    "-moz-font-feature-settings",
    "-webkit-hyphens",
    "-moz-hyphens",
    "-ms-hyphens",
    "-webkit-justify-content",
    "-webkit-mask",
    "-webkit-mask-clip",
    "-webkit-mask-composite",
    "-webkit-mask-image",
    "-webkit-mask-origin",
    "-webkit-mask-position",
    "-webkit-mask-repeat",
    "-webkit-mask-size",
    "-webkit-order",
    "-ms-flex-order",
    "-webkit-perspective",
    "-moz-perspective",
    "-webkit-perspective-origin",
    "-moz-perspective-origin",
    "-webkit-tab-size",
    "-moz-tab-size",
    "-o-tab-size",
    "-webkit-text-decoration",
    "-moz-text-decoration",
    "-webkit-text-decoration-color",
    "-moz-text-decoration-color",
    "-webkit-text-decoration-line",
    "-moz-text-decoration-line",
    "-webkit-text-decoration-skip-ink",
    "-webkit-text-decoration-style",
    "-moz-text-decoration-style",
    "-webkit-text-emphasis",
    "-webkit-text-emphasis-color",
    "-webkit-text-emphasis-position",
    "-webkit-text-emphasis-style",
    "-webkit-text-size-adjust",
    "-moz-text-size-adjust",
    "-ms-text-size-adjust",
    "-webkit-transform",
    "-moz-transform",
    "-ms-transform",
    "-o-transform",
    "-webkit-transform-origin",
    "-moz-transform-origin",
    "-ms-transform-origin",
    "-o-transform-origin",
    "-webkit-transform-style",
    "-moz-transform-style",
    "-webkit-transition",
    "-moz-transition",
    "-o-transition",
    "-webkit-transition-delay",
    "-moz-transition-delay",
    "-o-transition-delay",
    "-webkit-transition-duration",
    "-moz-transition-duration",
    "-o-transition-duration",
    "-webkit-transition-property",
    "-moz-transition-property",
    "-o-transition-property",
    "-webkit-transition-timing-function",
    "-moz-transition-timing-function",
    "-o-transition-timing-function",
    "-webkit-user-select",
    "-moz-user-select",
    "-ms-user-select",
];

fn strip_vendor_prefix(s: &str) -> Option<&str> {
    let lower = s.to_ascii_lowercase();
    for p in ["-webkit-", "-moz-", "-ms-", "-o-"] {
        if lower.starts_with(p) {
            return Some(&s[p.len()..]);
        }
    }
    None
}

impl Rule for PropertyNoVendorPrefix {
    fn name(&self) -> &'static str {
        "property-no-vendor-prefix"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if PREFIXED_PROPERTIES
            .iter()
            .any(|p| p.eq_ignore_ascii_case(prop))
        {
            if let Some(unprefixed) = strip_vendor_prefix(prop) {
                ctx.report_with_fix(
                    self.name(),
                    ident.span,
                    format!("Unexpected vendor-prefix \"{}\"", prop),
                    Some(Fix::single(ident.span, unprefixed.to_string())),
                );
            }
        }
    }
}
