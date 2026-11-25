use std::collections::HashSet;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, parse_option_matchers},
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

#[derive(Default)]
pub struct FontFamilyNoDuplicateNames {
    pub ignore_font_family_names: Vec<OptionMatcher>,
}

impl FontFamilyNoDuplicateNames {
    pub fn from_secondary(secondary: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        Self {
            ignore_font_family_names: parse_option_matchers(secondary, "ignoreFontFamilyNames"),
        }
    }
}

const GENERIC_FAMILIES: &[&str] = &[
    "inherit",
    "initial",
    "revert",
    "revert-layer",
    "unset",
    "serif",
    "sans-serif",
    "cursive",
    "fantasy",
    "monospace",
    "system-ui",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
    "math",
    "emoji",
    "fangsong",
];

const FONT_SHORTHAND_PREFIX_KEYWORDS: &[&str] = &[
    "normal",
    "italic",
    "oblique",
    "small-caps",
    "bold",
    "bolder",
    "lighter",
    "xx-small",
    "x-small",
    "small",
    "medium",
    "large",
    "x-large",
    "xx-large",
    "xxx-large",
    "larger",
    "smaller",
    "ultra-condensed",
    "extra-condensed",
    "condensed",
    "semi-condensed",
    "semi-expanded",
    "expanded",
    "extra-expanded",
    "ultra-expanded",
];

impl Rule for FontFamilyNoDuplicateNames {
    fn name(&self) -> &'static str {
        "font-family-no-duplicate-names"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let (is_shorthand, prop_end) = if let InterpolableIdent::Literal(ident) = &decl.name {
            let name: &str = ident.name;
            if name.eq_ignore_ascii_case("font-family") {
                (false, ident.span.end as usize)
            } else if name.eq_ignore_ascii_case("font") {
                (true, ident.span.end as usize)
            } else {
                return;
            }
        } else {
            return;
        };

        let end = decl.span.end as usize;
        let src = ctx.source;
        if prop_end >= end || end > src.len() {
            return;
        }
        let slice = &src[prop_end..end];
        let Some(colon_pos) = slice.find(':') else {
            return;
        };
        let val_start = prop_end + colon_pos + 1;
        let val_slice = &src[val_start..end];

        let mut keywords: HashSet<String> = HashSet::new();
        let mut family_names: HashSet<String> = HashSet::new();
        let mut offset = 0usize;

        for (part_idx, part) in val_slice.split(',').enumerate() {
            let part_start = val_start + offset;
            offset += part.len() + 1;

            let trimmed_end = part
                .trim_end()
                .trim_end_matches(';')
                .trim_end()
                .trim_end_matches("!important")
                .trim_end();
            let leading_ws = trimmed_end.len() - trimmed_end.trim_start().len();
            let mut item_slice = &trimmed_end[leading_ws..];
            let mut item_start = part_start + leading_ws;

            if item_slice.is_empty() {
                continue;
            }

            if is_shorthand && part_idx == 0 {
                let rel_offset = skip_font_shorthand_prefix(item_slice);
                if rel_offset >= item_slice.len() {
                    continue;
                }
                item_slice = &item_slice[rel_offset..];
                item_start += rel_offset;
            }

            if item_slice.is_empty() || item_slice.contains('(') {
                continue;
            }

            let is_quoted = (item_slice.starts_with('"') && item_slice.ends_with('"'))
                || (item_slice.starts_with('\'') && item_slice.ends_with('\''));

            let family = if is_quoted {
                item_slice[1..item_slice.len() - 1].trim().to_string()
            } else {
                item_slice.split_whitespace().collect::<Vec<_>>().join(" ")
            };

            if family.is_empty() {
                continue;
            }

            if self
                .ignore_font_family_names
                .iter()
                .any(|m| m.is_match(&family))
            {
                continue;
            }

            let raw_len = if is_quoted {
                family.len() + 2
            } else {
                family.len()
            };

            let span = Span {
                start: item_start as u32,
                end: (item_start + raw_len) as u32,
            };

            let family_lower = family.to_ascii_lowercase();
            let is_keyword = !is_quoted && GENERIC_FAMILIES.contains(&family_lower.as_str());

            if is_keyword {
                if keywords.contains(&family_lower) {
                    ctx.report(
                        self.name(),
                        span,
                        format!("Duplicate font-family name \"{family}\""),
                    );
                    continue;
                }
                keywords.insert(family_lower);
                continue;
            }

            if family_names.contains(&family) {
                ctx.report(
                    self.name(),
                    span,
                    format!("Duplicate font-family name \"{family}\""),
                );
                continue;
            }
            family_names.insert(family);
        }
    }
}

fn skip_font_shorthand_prefix(s: &str) -> usize {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut after_slash = false;

    while i < len {
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        if bytes[i] == b'"' || bytes[i] == b'\'' {
            return i;
        }

        let token_start = i;
        while i < len && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let token = &s[token_start..i];

        if after_slash {
            after_slash = false;
            continue;
        }

        if token == "/" || token.ends_with('/') {
            after_slash = true;
            continue;
        }

        if token.contains('/') || is_number_or_dimension(token) {
            continue;
        }

        let lower = token.to_ascii_lowercase();
        if FONT_SHORTHAND_PREFIX_KEYWORDS.contains(&lower.as_str()) {
            continue;
        }

        return token_start;
    }

    len
}

fn is_number_or_dimension(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut i = 0;
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
        if i >= bytes.len() {
            return false;
        }
    }
    if bytes[i] == b'.' {
        i += 1;
        if i >= bytes.len() {
            return false;
        }
    }
    bytes[i].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    const RULE_NAME: &str = "font-family-no-duplicate-names";

    fn rejected(name: &str) -> String {
        format!("Duplicate font-family name \"{name}\"")
    }

    #[test]
    fn default_config() {
        let pass = [
            "a { font-family: \"Lucida Grande\", \"Arial\", sans-serif; }",
            "a { font: 1em \"Lucida Grande\", 'Arial', sans-serif; }",
            "a { font: 1em \"Lucida Grande\", 'Arial', \"sans-serif\", sans-serif; }",
            "a { font-family: Times, serif; }",
            "b { font: normal 14px/32px -apple-system, BlinkMacSystemFont, sans-serif; }",
        ];
        let fail = [
            reject(
                "a { font-family: \"Lucida Grande\", 'Arial', sans-serif, sans-serif; }",
                None,
                rejected("sans-serif"),
                (1, 56, 1, 66),
            ),
            reject(
                "a { font-family: 'Arial', \"Lucida Grande\", Arial, sans-serif; }",
                None,
                rejected("Arial"),
                (1, 44, 1, 49),
            ),
            reject(
                "a { fOnT-fAmIlY: '  Lucida Grande ', \"Lucida Grande\", sans-serif; }",
                None,
                rejected("Lucida Grande"),
                (1, 38, 1, 53),
            ),
            reject(
                "a { font-family: 'Times', Times, \"serif\", serif; }",
                None,
                rejected("Times"),
                (1, 27, 1, 32),
            ),
            reject(
                "a { FONT: italic 300 16px/30px Arial, \" Arial\", serif; }",
                None,
                rejected("Arial"),
                (1, 39, 1, 46),
            ),
            reject(
                "b { font: normal 14px/32px -apple-system, BlinkMacSystemFont, sans-serif, sans-serif; }",
                None,
                rejected("sans-serif"),
                (1, 75, 1, 85),
            ),
        ];

        Tester::new(RULE_NAME, json!([true]), pass, fail).test();
    }

    #[test]
    fn ignore_font_family_names_strings_and_regexes() {
        let pass = [
            "pre { font-family: monospace, monospace }",
            "pre { font: 1em monospace, monospace}",
            "pre { font-family: monospace, \"Roberto Mono\", monospace}",
            "pre { font-family: \"my-font\", \"my-font\", sans-serif}",
            "pre { font-family: \"your-font\", \"YOUR-font\", sans-serif}",
        ];
        let fail = [
            reject(
                "pre { font-family: \"Roberto Mono\", \"Roberto Mono\", monospace}",
                None,
                rejected("Roberto Mono"),
                (1, 36, 1, 50),
            ),
            reject(
                "pre { font-family: My-font, \"My-font\", sans-serif}",
                None,
                rejected("My-font"),
                (1, 29, 1, 38),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignoreFontFamilyNames": ["monospace", "/my-/", "/^YOUR-/i"] }]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn ignore_font_family_names_regex() {
        let pass = ["pre { font-family: \"my-font\", \"my-font\", sans-serif}"];
        let fail = [reject(
            "pre { font-family: \"Roberto Mono\", \"Roberto Mono\", monospace}",
            None,
            rejected("Roberto Mono"),
            (1, 36, 1, 50),
        )];

        Tester::new(
            RULE_NAME,
            json!([true, { "ignoreFontFamilyNames": ["/my-/"] }]),
            pass,
            fail,
        )
        .test();
    }
}
