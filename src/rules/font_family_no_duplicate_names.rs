use std::collections::HashSet;

use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

pub struct FontFamilyNoDuplicateNames;

const GENERIC_FAMILIES: &[&str] = &[
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

impl Rule for FontFamilyNoDuplicateNames {
    fn name(&self) -> &'static str {
        "font-family-no-duplicate-names"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let prop_end = if let InterpolableIdent::Literal(ident) = &decl.name {
            let name: &str = ident.name;
            if !name.eq_ignore_ascii_case("font-family") && !name.eq_ignore_ascii_case("font") {
                return;
            }
            ident.span.end as usize
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

        let mut seen_keywords = HashSet::new();
        let mut seen_quoted = HashSet::new();
        let mut offset = 0usize;

        for part in val_slice.split(',') {
            let part_start = val_start + offset;
            offset += part.len() + 1;

            let trimmed = part
                .trim()
                .trim_end_matches(';')
                .trim_end_matches("!important")
                .trim();
            if trimmed.is_empty() {
                continue;
            }

            let leading_ws = part.len() - part.trim_start().len();
            let item_start = (part_start + leading_ws) as u32;
            let item_end = item_start + trimmed.len() as u32;

            let is_quoted = (trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\''));

            let unquoted = trimmed.trim_matches(|c| c == '"' || c == '\'').trim();
            if unquoted.is_empty() || unquoted.contains('(') || unquoted.starts_with("var(") {
                continue;
            }

            let normalized = unquoted
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase();

            if is_quoted {
                if GENERIC_FAMILIES.contains(&normalized.as_str()) {
                    if !seen_quoted.insert(normalized) {
                        ctx.report(
                            self.name(),
                            Span {
                                start: item_start,
                                end: item_end,
                            },
                            format!("Unexpected duplicate font-family name {trimmed}"),
                        );
                    }
                } else if !seen_quoted.insert(normalized.clone())
                    || seen_keywords.contains(&normalized)
                {
                    ctx.report(
                        self.name(),
                        Span {
                            start: item_start,
                            end: item_end,
                        },
                        format!("Unexpected duplicate font-family name {trimmed}"),
                    );
                }
            } else {
                let family_token = if normalized.contains('/') {
                    let Some(after_slash) = normalized.split('/').nth(1) else {
                        continue;
                    };
                    let tokens: Vec<&str> = after_slash.split_whitespace().collect();
                    if tokens.len() < 2 {
                        continue;
                    }
                    tokens[1..].join(" ")
                } else {
                    normalized
                };

                if !seen_keywords.insert(family_token.clone())
                    || (!GENERIC_FAMILIES.contains(&family_token.as_str())
                        && seen_quoted.contains(&family_token))
                {
                    ctx.report(
                        self.name(),
                        Span {
                            start: item_start,
                            end: item_end,
                        },
                        format!("Unexpected duplicate font-family name {trimmed}"),
                    );
                }
            }
        }
    }
}
