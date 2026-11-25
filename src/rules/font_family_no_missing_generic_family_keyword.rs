use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, font_size_end, options_matches, parse_option_matchers},
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};

pub struct FontFamilyNoMissingGenericFamilyKeyword {
    ignore_font_families: Vec<OptionMatcher>,
}

impl FontFamilyNoMissingGenericFamilyKeyword {
    pub fn from_config(secondary: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        Self {
            ignore_font_families: parse_option_matchers(secondary, "ignoreFontFamilies"),
        }
    }
}

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
        if ctx.ancestors.iter().any(|p| {
            matches!(p, crate::context::AstParent::AtRule(at) if matches!(at.name.name.to_ascii_lowercase().as_str(), "font-face" | "font-palette-values"))
        })
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
            || raw_val.contains('@')
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

        let family_start = raw_val.rfind(',').map_or(0, |comma| comma + 1);
        let family_start = if is_shorthand && family_start == 0 {
            font_size_end(raw_val).unwrap_or(0)
        } else {
            family_start
        };
        let family = raw_val[family_start..].trim();
        if family.is_empty()
            || options_matches(&self.ignore_font_families, family.trim_matches(['"', '\'']))
        {
            return;
        }
        let value_offset = slice[colon_pos + 1..].find(raw_val).unwrap_or(0);
        let start = prop_end + colon_pos + 1 + value_offset + raw_val.rfind(family).unwrap_or(0);
        ctx.report(
            self.name(),
            Span {
                start: start as u32,
                end: (start + family.len()) as u32,
            },
            "Missing generic font family".to_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn family_spans_and_ignored_contexts() {
        let pass = [
            "@font-palette-values --foo { font-family: bar; }",
            "a { font: @font-family; }",
            "a { font: \"font\", @font-family; }",
        ];
        let fail = [
            reject(
                "a { font-family: Arial; }",
                None,
                "Missing generic font family",
                (1, 18, 1, 23),
            ),
            reject(
                "a { font: 1em \"ruby\"; }",
                None,
                "Missing generic font family",
                (1, 15, 1, 21),
            ),
        ];
        Tester::new(
            "font-family-no-missing-generic-family-keyword",
            json!(true),
            pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn ignores_configured_font_families() {
        Tester::new(
            "font-family-no-missing-generic-family-keyword",
            json!([true, {"ignoreFontFamilies": ["custom-font", "/foo/", "/baz[0-9]/"]}]),
            [
                "a { font-family: custom-font; }",
                "a { font: 1em Lucida Grande, Arial, custom-font }",
                "a { font: foo-font }",
                "a { font: baz7 }",
            ],
            [],
        )
        .test();
    }
}
