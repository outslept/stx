use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        value_utils::{SelectorItemKind, scan_selector_items},
    },
};
use oxc_css_parser::{Span, ast::SelectorList};

pub struct SelectorNoVendorPrefix;

const PREFIXED_SELECTORS: &[&str] = &[
    "-webkit-autofill",
    "-webkit-any-link",
    "-moz-any-link",
    "-webkit-full-screen",
    "-moz-full-screen",
    "-ms-fullscreen",
    "-moz-placeholder",
    "-ms-input-placeholder",
    "-webkit-input-placeholder",
    "-moz-selection",
    "-moz-focus-inner",
    "-moz-ui-invalid",
    "-moz-ui-valid",
    "-moz-read-only",
    "-moz-read-write",
    "-webkit-file-upload-button",
    "-webkit-backdrop",
    "-ms-backdrop",
];

fn strip_prefix(s: &str) -> Option<&str> {
    let lower = s.to_ascii_lowercase();
    for p in ["-webkit-", "-moz-", "-ms-", "-o-"] {
        if lower.starts_with(p) {
            return Some(&s[p.len()..]);
        }
    }
    None
}

impl Rule for SelectorNoVendorPrefix {
    fn name(&self) -> &'static str {
        "selector-no-vendor-prefix"
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        let items = scan_selector_items(ctx.source, list.span);
        for item in items {
            let colon_end = match item.kind {
                SelectorItemKind::PseudoClass { colon_span } => colon_span.end,
                SelectorItemKind::PseudoElement { colon_span, .. } => colon_span.end,
                _ => continue,
            };

            if PREFIXED_SELECTORS
                .iter()
                .any(|s| s.eq_ignore_ascii_case(item.name))
            {
                if let Some(unprefixed) = strip_prefix(item.name) {
                    let raw = ctx.slice(item.span);
                    let name_span = Span {
                        start: colon_end,
                        end: item.span.end,
                    };
                    ctx.report_with_fix(
                        self.name(),
                        item.span,
                        format!("Unexpected vendor-prefix \"{}\"", raw),
                        Some(Fix::single(name_span, unprefixed.to_string())),
                    );
                }
            }
        }
    }
}
