use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::AtRule};

pub struct MediaFeatureNameNoUnknown;

const KNOWN_MEDIA_FEATURES: &[&str] = &[
    "any-hover",
    "any-pointer",
    "aspect-ratio",
    "color",
    "color-gamut",
    "color-index",
    "device-aspect-ratio",
    "device-height",
    "device-width",
    "display-mode",
    "dynamic-range",
    "environment-blending",
    "forced-colors",
    "grid",
    "height",
    "horizontal-viewport-segments",
    "hover",
    "inverted-colors",
    "max-aspect-ratio",
    "max-color",
    "max-color-index",
    "max-device-aspect-ratio",
    "max-device-height",
    "max-device-width",
    "max-height",
    "max-monochrome",
    "max-resolution",
    "max-width",
    "min-aspect-ratio",
    "min-color",
    "min-color-index",
    "min-device-aspect-ratio",
    "min-device-height",
    "min-device-width",
    "min-height",
    "min-monochrome",
    "min-resolution",
    "min-width",
    "monochrome",
    "nav-controls",
    "orientation",
    "overflow-block",
    "overflow-inline",
    "pointer",
    "prefers-color-scheme",
    "prefers-contrast",
    "prefers-reduced-data",
    "prefers-reduced-motion",
    "prefers-reduced-transparency",
    "resolution",
    "scan",
    "scripting",
    "update",
    "vertical-viewport-segments",
    "video-color-gamut",
    "video-dynamic-range",
    "width",
];

impl Rule for MediaFeatureNameNoUnknown {
    fn name(&self) -> &'static str {
        "media-feature-name-no-unknown"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("media") {
            return;
        }

        let start = at_rule.name.span.end as usize;
        let end = at_rule
            .block
            .as_ref()
            .map(|b| b.span.start as usize)
            .unwrap_or(at_rule.span.end as usize);

        let src = ctx.source;
        if start >= end || end > src.len() {
            return;
        }

        let bytes = src.as_bytes();
        let mut i = start;

        while i < end {
            if bytes[i] == b'(' {
                i += 1;
                while i < end && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i < end && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'-') {
                    let ident_start = i;
                    while i < end
                        && (bytes[i].is_ascii_alphanumeric()
                            || bytes[i] == b'-'
                            || bytes[i] == b'_')
                    {
                        i += 1;
                    }
                    let ident_end = i;
                    let ident = &src[ident_start..ident_end];
                    let lower = ident.to_ascii_lowercase();

                    if lower == "not" || lower == "only" || lower == "and" || lower == "or" {
                        continue;
                    }

                    while i < end && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }

                    if i < end && matches!(bytes[i], b':' | b')' | b'<' | b'>' | b'=') {
                        if !ident.starts_with('-')
                            && !ident.starts_with("--")
                            && !KNOWN_MEDIA_FEATURES.contains(&lower.as_str())
                        {
                            ctx.report(
                                self.name(),
                                Span {
                                    start: ident_start as u32,
                                    end: ident_end as u32,
                                },
                                format!("Unexpected unknown media feature name \"{ident}\""),
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
