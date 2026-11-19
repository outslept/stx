use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::{Declaration, InterpolableIdent};

pub struct FontFamilyNoMissingGenericFamilyKeyword;

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

const GLOBAL_KEYWORDS: &[&str] = &["inherit", "initial", "unset", "revert", "revert-layer"];

impl Rule for FontFamilyNoMissingGenericFamilyKeyword {
    fn name(&self) -> &'static str {
        "font-family-no-missing-generic-family-keyword"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if ctx
            .ancestry
            .iter()
            .any(|p| matches!(p, crate::context::AstParent::AtRule(at) if at.name.name.eq_ignore_ascii_case("font-face")))
        {
            return;
        }

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
        let raw_val = slice[colon_pos + 1..]
            .trim()
            .trim_end_matches(';')
            .trim_end_matches("!important")
            .trim();

        if raw_val.is_empty()
            || raw_val.contains("var(")
            || raw_val.contains("env(")
            || raw_val.contains('$')
        {
            return;
        }

        let lower_val = raw_val.to_ascii_lowercase();
        if GLOBAL_KEYWORDS.contains(&lower_val.as_str()) {
            return;
        }

        if is_shorthand
            && matches!(
                lower_val.as_str(),
                "caption" | "icon" | "menu" | "message-box" | "small-caption" | "status-bar"
            )
        {
            return;
        }

        for part in raw_val.split(',') {
            let item = part.trim();
            if item.starts_with('"') || item.starts_with('\'') {
                continue;
            }
            for token in item.split_whitespace() {
                let lower = token.to_ascii_lowercase();
                if GENERIC_FAMILIES.contains(&lower.as_str()) {
                    return;
                }
            }
        }

        ctx.report(
            self.name(),
            decl.span,
            "Unexpected missing generic font family".to_string(),
        );
    }
}
