use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::ComplexSelector};

pub struct SelectorPseudoElementNoUnknown;

const KNOWN_PSEUDO_ELEMENTS: &[&str] = &[
    "after",
    "backdrop",
    "before",
    "cue",
    "cue-region",
    "details-content",
    "file-selector-button",
    "first-letter",
    "first-line",
    "grammar-error",
    "highlight",
    "marker",
    "part",
    "placeholder",
    "selection",
    "slotted",
    "spelling-error",
    "target-text",
    "view-transition",
    "view-transition-group",
    "view-transition-image-pair",
    "view-transition-new",
    "view-transition-old",
    "v-deep",
    "ng-deep",
    "deep",
];

impl Rule for SelectorPseudoElementNoUnknown {
    fn name(&self) -> &'static str {
        "selector-pseudo-element-no-unknown"
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        let start = sel.span.start as usize;
        let end = sel.span.end as usize;
        let src = ctx.source;
        if start >= end || end > src.len() {
            return;
        }

        let bytes = src.as_bytes();
        let mut i = start;

        while i < end {
            if bytes[i] == b'[' {
                i += 1;
                while i < end && bytes[i] != b']' {
                    if bytes[i] == b'"' || bytes[i] == b'\'' {
                        let q = bytes[i];
                        i += 1;
                        while i < end && bytes[i] != q {
                            if bytes[i] == b'\\' {
                                i += 2;
                                continue;
                            }
                            i += 1;
                        }
                    }
                    i += 1;
                }
                i += 1;
                continue;
            }

            if i + 1 < end && bytes[i] == b':' && bytes[i + 1] == b':' {
                let colons_pos = i;
                i += 2;
                let name_start = i;
                while i < end
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
                {
                    i += 1;
                }
                if i > name_start {
                    let raw_name = &src[name_start..i];
                    if !raw_name.starts_with('-') {
                        let lower = raw_name.to_ascii_lowercase();
                        if !KNOWN_PSEUDO_ELEMENTS.contains(&lower.as_str()) {
                            ctx.report(
                                self.name(),
                                Span {
                                    start: colons_pos as u32,
                                    end: i as u32,
                                },
                                format!(
                                    "Unexpected unknown pseudo-element selector \"::{raw_name}\""
                                ),
                            );
                        }
                    }
                }
                continue;
            }

            i += 1;
        }
    }
}
