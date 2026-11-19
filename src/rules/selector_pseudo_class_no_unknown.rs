use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::ComplexSelector};

pub struct SelectorPseudoClassNoUnknown;

const KNOWN_PSEUDO_CLASSES: &[&str] = &[
    "active",
    "any-link",
    "autofill",
    "blank",
    "buffering",
    "checked",
    "current",
    "default",
    "defined",
    "dir",
    "disabled",
    "empty",
    "enabled",
    "first",
    "first-child",
    "first-of-type",
    "focus",
    "focus-visible",
    "focus-within",
    "fullscreen",
    "future",
    "has",
    "host",
    "host-context",
    "hover",
    "in-range",
    "indeterminate",
    "invalid",
    "is",
    "lang",
    "last-child",
    "last-of-type",
    "left",
    "link",
    "local-link",
    "matches",
    "modal",
    "muted",
    "not",
    "nth-child",
    "nth-col",
    "nth-last-child",
    "nth-last-col",
    "nth-last-of-type",
    "nth-of-type",
    "only-child",
    "only-of-type",
    "open",
    "optional",
    "out-of-range",
    "past",
    "paused",
    "picture-in-picture",
    "placeholder-shown",
    "playing",
    "popover-open",
    "read-only",
    "read-write",
    "recto",
    "required",
    "right",
    "root",
    "scope",
    "seeking",
    "stalled",
    "state",
    "target",
    "target-within",
    "user-invalid",
    "user-valid",
    "valid",
    "verso",
    "visited",
    "volume-locked",
    "where",
    "window-inactive",
    "after",
    "before",
    "first-letter",
    "first-line",
    "global",
    "local",
    "export",
    "import",
    "deep",
    "slotted",
    "horizontal",
    "vertical",
    "decrement",
    "increment",
    "start",
    "end",
    "double-button",
    "single-button",
    "no-button",
    "corner-present",
];

impl Rule for SelectorPseudoClassNoUnknown {
    fn name(&self) -> &'static str {
        "selector-pseudo-class-no-unknown"
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

            if bytes[i] == b':' {
                if i + 1 < end && bytes[i + 1] == b':' {
                    i += 2;
                    while i < end
                        && (bytes[i].is_ascii_alphanumeric()
                            || bytes[i] == b'-'
                            || bytes[i] == b'_')
                    {
                        i += 1;
                    }
                    continue;
                }

                let colon_pos = i;
                i += 1;
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
                        if !KNOWN_PSEUDO_CLASSES.contains(&lower.as_str()) {
                            ctx.report(
                                self.name(),
                                Span {
                                    start: colon_pos as u32,
                                    end: i as u32,
                                },
                                format!("Unexpected unknown pseudo-class selector \":{raw_name}\""),
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
