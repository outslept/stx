use std::collections::HashSet;

use crate::{
    context::LintContext,
    rules::{Rule, value_utils::PatternMatcher},
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

#[derive(Default)]
pub struct FontFamilyNoDuplicateNames {
    pub ignore_font_family_names: Vec<PatternMatcher>,
}

impl FontFamilyNoDuplicateNames {
    pub fn from_secondary(secondary: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        let mut ignore_font_family_names = Vec::new();
        if let Some(arr) = secondary
            .and_then(|m| m.get("ignoreFontFamilyNames"))
            .and_then(|v| v.as_array())
        {
            for item in arr {
                if let Some(s) = item.as_str() {
                    let pattern = if let Some(inner) = s.strip_prefix('/') {
                        if let Some(pat) = inner.strip_suffix("/i") {
                            format!("(?i){pat}")
                        } else if let Some(pat) = inner.strip_suffix('/') {
                            pat.to_string()
                        } else {
                            s.to_string()
                        }
                    } else {
                        format!("^{}$", regex::escape(s))
                    };
                    ignore_font_family_names.push(PatternMatcher::new(Some(&pattern)));
                }
            }
        }
        Self {
            ignore_font_family_names,
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
