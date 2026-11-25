use oxc_css_parser::{Span, ast::AtRule};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        DEPRECATED_MEDIA_TYPES_NAMES, OptionMatcher, at_rule_prelude_span, options_matches,
        parse_option_matchers,
    },
};

crate::define_stx_lint! {
    /// Disallow deprecated media types in `@media` queries.
    ///
    /// Enable with `true`. `ignoreMediaTypes` accepts a name or array of names
    /// and `/regex/` patterns. Deprecation is checked case-insensitively, but
    /// the ignore list matches the original spelling. This rule checks known
    /// deprecated types, including `speech`, not unknown media types.
    ///
    /// Example: `@media handheld {}` is reported; `@media screen {}` passes.
    "media-type-no-deprecated", restriction;
    #[derive(Debug, Default)]
    pub struct MediaTypeNoDeprecated {
        ignore: Vec<OptionMatcher>,
    }
}

impl MediaTypeNoDeprecated {
    pub fn new(secondary: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        Self {
            ignore: parse_option_matchers(secondary, "ignoreMediaTypes"),
        }
    }
}

pub fn rejected_message(media_type: &str) -> String {
    format!("Deprecated media type \"{media_type}\"")
}

impl Rule for MediaTypeNoDeprecated {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if !at_rule.name.name.eq_ignore_ascii_case("media") {
            return;
        }
        let Some(prelude) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let params = ctx.slice(prelude);

        let bytes = params.as_bytes();
        let len = bytes.len();
        let mut segment_start = 0usize;
        let mut depth = 0usize;
        let mut i = 0usize;
        loop {
            let at_end = i == len;
            if at_end || (bytes[i] == b',' && depth == 0) {
                let segment = &params[segment_start..i];
                if let Some((offset, media_type)) = media_type(segment)
                    && DEPRECATED_MEDIA_TYPES_NAMES
                        .iter()
                        .any(|name| name.eq_ignore_ascii_case(media_type))
                    && !options_matches(&self.ignore, media_type)
                {
                    let start = prelude.start + segment_start as u32 + offset as u32;
                    ctx.report(
                        self.name(),
                        Span {
                            start,
                            end: start + media_type.len() as u32,
                        },
                        rejected_message(media_type),
                    );
                }
                if at_end {
                    break;
                }
                segment_start = i + 1;
            } else if bytes[i] == b'(' {
                depth += 1;
            } else if bytes[i] == b')' {
                depth = depth.saturating_sub(1);
            }
            i += 1;
        }
    }
}

fn media_type(segment: &str) -> Option<(usize, &str)> {
    let bytes = segment.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= len || bytes[i] == b'(' {
        return None;
    }
    let (start, word) = read_ident(segment, i)?;
    if word.eq_ignore_ascii_case("not") || word.eq_ignore_ascii_case("only") {
        let mut j = start + word.len();
        while j < len && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        return read_ident(segment, j);
    }
    Some((start, word))
}

fn read_ident(segment: &str, start: usize) -> Option<(usize, &str)> {
    let bytes = segment.as_bytes();
    let len = bytes.len();
    let mut i = start;
    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_') {
        i += 1;
    }
    if i == start {
        return None;
    }
    Some((start, &segment[start..i]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "@media all {}",
            "@media screen {}",
            "@media print {}",
            "@media {}",
            "@media screen and (min-width: 600px) {}",
            "@media all and (color: 8) {} ",
            "@media screen and (min-width: 600px), print and (max-width: 800px) {}",
        ];

        let fail = vec![
            reject(
                "@media tty {}",
                None,
                rejected_message("tty"),
                (1, 8, 1, 11),
            ),
            reject("@media tv {}", None, rejected_message("tv"), (1, 8, 1, 10)),
            reject_warnings(
                "@media handheld and (max-width: 800px), projection and (min-width: 1200px) {}",
                None,
                vec![
                    (rejected_message("handheld"), (1, 8, 1, 16)),
                    (rejected_message("projection"), (1, 41, 1, 51)),
                ],
            ),
            reject(
                "@media braille {}",
                None,
                rejected_message("braille"),
                (1, 8, 1, 15),
            ),
            reject(
                "@media embossed {}",
                None,
                rejected_message("embossed"),
                (1, 8, 1, 16),
            ),
            reject(
                "@media aural {}",
                None,
                rejected_message("aural"),
                (1, 8, 1, 13),
            ),
            reject(
                "@media speech {}",
                None,
                rejected_message("speech"),
                (1, 8, 1, 14),
            ),
            reject(
                "@media screen, print, tty {}",
                None,
                rejected_message("tty"),
                (1, 23, 1, 26),
            ),
            reject_warnings(
                "@media screen, tty, print, tv {}",
                None,
                vec![
                    (rejected_message("tty"), (1, 16, 1, 19)),
                    (rejected_message("tv"), (1, 28, 1, 30)),
                ],
            ),
            reject_warnings(
                "@media tty, all, tv {}",
                None,
                vec![
                    (rejected_message("tty"), (1, 8, 1, 11)),
                    (rejected_message("tv"), (1, 18, 1, 20)),
                ],
            ),
            reject_warnings(
                "@media tty, all, tty {}",
                None,
                vec![
                    (rejected_message("tty"), (1, 8, 1, 11)),
                    (rejected_message("tty"), (1, 18, 1, 21)),
                ],
            ),
        ];

        Tester::new("media-type-no-deprecated", json!(true), &pass, &fail).test();
    }

    #[test]
    fn ignore_media_types() {
        let pass = vec!["@media speech {}", "@media tty {}", "@media tv {}"];
        let fail = vec![reject(
            "@media aural {}",
            None,
            rejected_message("aural"),
            (1, 8, 1, 13),
        )];
        Tester::new(
            "media-type-no-deprecated",
            json!([true, {"ignoreMediaTypes": ["speech", "/^t/"]}]),
            &pass,
            &fail,
        )
        .test();
    }
}
