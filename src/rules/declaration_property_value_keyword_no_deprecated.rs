use oxc_css_parser::ast::Declaration;

use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        COLOR_FUNCTIONS, OptionMatcher, SINGLE_VALUE_COLOR_PROPERTIES, ValueTokenKind,
        decl_value_span, ident_name, is_standard_syntax_declaration, is_standard_syntax_value,
        options_matches, parse_option_matchers, tokenize_value,
    },
};

const DEPRECATED_COLORS: &[(&str, &str)] = &[
    ("activecaption", "canvas"),
    ("appworkspace", "canvas"),
    ("background", "canvas"),
    ("inactivecaption", "canvas"),
    ("infobackground", "canvas"),
    ("menu", "canvas"),
    ("scrollbar", "canvas"),
    ("window", "canvas"),
    ("activeborder", "ButtonBorder"),
    ("inactiveborder", "ButtonBorder"),
    ("threeddarkshadow", "ButtonBorder"),
    ("threedhighlight", "ButtonBorder"),
    ("threedlightshadow", "ButtonBorder"),
    ("threedshadow", "ButtonBorder"),
    ("windowframe", "ButtonBorder"),
    ("captiontext", "CanvasText"),
    ("infotext", "CanvasText"),
    ("menutext", "CanvasText"),
    ("windowtext", "CanvasText"),
    ("buttonhighlight", "ButtonFace"),
    ("buttonshadow", "ButtonFace"),
    ("threedface", "ButtonFace"),
    ("inactivecaptiontext", "GrayText"),
];

const APPEARANCE: &[(&str, &str)] = &[
    ("button", "auto"),
    ("checkbox", "auto"),
    ("listbox", "auto"),
    ("menulist", "auto"),
    ("meter", "auto"),
    ("progress-bar", "auto"),
    ("push-button", "auto"),
    ("radio", "auto"),
    ("searchfield", "auto"),
    ("slider-horizontal", "auto"),
    ("square-button", "auto"),
    ("textarea", "auto"),
];

const IMAGE_RENDERING: &[(&str, &str)] = &[
    ("optimizequality", "smooth"),
    ("optimizespeed", "pixelated"),
];

const OVERFLOW: &[(&str, &str)] = &[("overlay", "auto")];
const TEXT_JUSTIFY: &[(&str, &str)] = &[("distribute", "inter-character")];
const TEXT_ORIENTATION: &[(&str, &str)] = &[("sideways-right", "sideways")];
const USER_SELECT: &[(&str, &str)] = &[("element", "contain")];
const ZOOM: &[(&str, &str)] = &[("reset", "1")];

const BLINK: &[&str] = &["blink"];
const PADDING_BOX: &[&str] = &["padding-box"];
const FLIP: &[&str] = &["flip"];
const INTRINSIC: &[&str] = &["intrinsic", "min-intrinsic"];
const BREAK_WORD: &[&str] = &["break-word"];

#[derive(Clone, Copy)]
enum Keywords {
    Replace(&'static [(&'static str, &'static str)]),
    Reject(&'static [&'static str]),
}

fn keywords_for(prop_lower: &str) -> Option<Keywords> {
    Some(match prop_lower {
        "appearance" => Keywords::Replace(APPEARANCE),
        "image-rendering" => Keywords::Replace(IMAGE_RENDERING),
        "overflow" | "overflow-x" | "overflow-y" => Keywords::Replace(OVERFLOW),
        "text-justify" => Keywords::Replace(TEXT_JUSTIFY),
        "text-orientation" => Keywords::Replace(TEXT_ORIENTATION),
        "user-select" => Keywords::Replace(USER_SELECT),
        "zoom" => Keywords::Replace(ZOOM),
        "border-color" | "scrollbar-color" => Keywords::Replace(DEPRECATED_COLORS),
        "text-decoration" | "text-decoration-line" => Keywords::Reject(BLINK),
        "box-sizing" => Keywords::Reject(PADDING_BOX),
        "image-orientation" => Keywords::Reject(FLIP),
        "min-height" | "min-width" | "max-height" | "max-width" | "height" | "width" => {
            Keywords::Reject(INTRINSIC)
        }
        "word-break" => Keywords::Reject(BREAK_WORD),
        _ if SINGLE_VALUE_COLOR_PROPERTIES.contains(&prop_lower) => {
            Keywords::Replace(DEPRECATED_COLORS)
        }
        _ => return None,
    })
}

pub fn expected_message(actual: &str, expected: &str) -> String {
    format!("Expected \"{actual}\" to be \"{expected}\"")
}

pub fn rejected_message(property: &str, keyword: &str) -> String {
    format!("Deprecated keyword \"{keyword}\" for property \"{property}\"")
}

#[derive(Debug, Default)]
pub struct DeclarationPropertyValueKeywordNoDeprecated {
    ignore_keywords: Vec<OptionMatcher>,
}

impl DeclarationPropertyValueKeywordNoDeprecated {
    pub fn new(secondary: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        Self {
            ignore_keywords: parse_option_matchers(secondary, "ignoreKeywords"),
        }
    }
}

impl Rule for DeclarationPropertyValueKeywordNoDeprecated {
    fn name(&self) -> &'static str {
        "declaration-property-value-keyword-no-deprecated"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if !is_standard_syntax_declaration(decl) {
            return;
        }
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let value = ctx.slice(value_span);
        if options_matches(&self.ignore_keywords, value) {
            return;
        }

        let Some(keywords) = keywords_for(&prop.to_ascii_lowercase()) else {
            return;
        };

        let tokens = tokenize_value(ctx.source, value_span);
        for token in &tokens {
            let raw = if token.kind == ValueTokenKind::Function {
                token.raw.strip_suffix('(').unwrap_or(token.raw)
            } else {
                token.raw
            };
            if !is_standard_syntax_value(raw) {
                return;
            }
        }

        let mut depth = 0usize;
        let mut skip_from: Option<usize> = None;
        for token in &tokens {
            match token.kind {
                ValueTokenKind::Function => {
                    depth += 1;
                    if skip_from.is_none() {
                        let name = token.raw.strip_suffix('(').unwrap_or(token.raw);
                        let is_color = COLOR_FUNCTIONS
                            .iter()
                            .any(|function| function.eq_ignore_ascii_case(name));
                        if !is_color {
                            skip_from = Some(depth);
                        }
                    }
                }
                ValueTokenKind::CloseParen => {
                    depth = depth.saturating_sub(1);
                    if skip_from.is_some_and(|from| depth < from) {
                        skip_from = None;
                    }
                }
                _ if skip_from.is_some() => {}
                ValueTokenKind::Ident => {
                    let lower = token.raw.to_ascii_lowercase();
                    match keywords {
                        Keywords::Replace(map) => {
                            if let Some(&(_, expected)) =
                                map.iter().find(|(deprecated, _)| *deprecated == lower)
                            {
                                ctx.report_with_fix(
                                    self.name(),
                                    token.span,
                                    expected_message(token.raw, expected),
                                    Fix::single(token.span, expected.to_string()),
                                );
                            }
                        }
                        Keywords::Reject(list) => {
                            if list.contains(&lower.as_str()) {
                                ctx.report(
                                    self.name(),
                                    token.span,
                                    rejected_message(prop, token.raw),
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec!["a { color: red; }", "a { color: --foo(background); }"];

        let fail = vec![
            reject(
                "a { appearance: searchfield; }",
                "a { appearance: auto; }",
                expected_message("searchfield", "auto"),
                (1, 17, 1, 28),
            ),
            reject(
                "a { color: InactiveCaptionText; }",
                "a { color: GrayText; }",
                expected_message("InactiveCaptionText", "GrayText"),
                (1, 12, 1, 31),
            ),
            reject(
                "a { color: CoLoR(from InactiveCaptionText srgb r g b / 0.5); }",
                "a { color: CoLoR(from GrayText srgb r g b / 0.5); }",
                expected_message("InactiveCaptionText", "GrayText"),
                (1, 23, 1, 42),
            ),
            reject(
                "a { scrollbar-color: --foo(background, bar) menu; }",
                "a { scrollbar-color: --foo(background, bar) canvas; }",
                expected_message("menu", "canvas"),
                (1, 45, 1, 49),
            ),
            reject(
                "a { zOom: /*qux*/reset/*baz*/; }",
                "a { zOom: /*qux*/1/*baz*/; }",
                expected_message("reset", "1"),
                (1, 18, 1, 23),
            ),
            reject(
                "a { overflow-x: overlay; }",
                "a { overflow-x: auto; }",
                expected_message("overlay", "auto"),
                (1, 17, 1, 24),
            ),
            reject(
                "a { border-color: green yellow WindowFrame blue; }",
                "a { border-color: green yellow ButtonBorder blue; }",
                expected_message("WindowFrame", "ButtonBorder"),
                (1, 32, 1, 43),
            ),
            reject(
                "a { scrollbar-color: red Background; }",
                "a { scrollbar-color: red canvas; }",
                expected_message("Background", "canvas"),
                (1, 26, 1, 36),
            ),
            reject(
                "a { overflow: overlay; }",
                "a { overflow: auto; }",
                expected_message("overlay", "auto"),
                (1, 15, 1, 22),
            ),
            reject(
                "a { overflow: hidden overlay; }",
                "a { overflow: hidden auto; }",
                expected_message("overlay", "auto"),
                (1, 22, 1, 29),
            ),
            reject(
                "a { overflow: hidden /* comment */ overlay; }",
                "a { overflow: hidden /* comment */ auto; }",
                expected_message("overlay", "auto"),
                (1, 36, 1, 43),
            ),
            reject(
                "a { text-decoration-line: /* foo */blink/* bar */; }",
                None,
                rejected_message("text-decoration-line", "blink"),
                (1, 36, 1, 41),
            ),
            reject(
                "a { text-decoration: foo blink bar qux baz; }",
                None,
                rejected_message("text-decoration", "blink"),
                (1, 26, 1, 31),
            ),
        ];

        Tester::new(
            "declaration-property-value-keyword-no-deprecated",
            json!(true),
            &pass,
            &fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn ignore_keywords() {
        let pass = vec![
            "a { box-sizing: padding-box; }",
            "a { width: min-intrinsic; }",
            "a { appearance: textfield; }",
        ];

        let fail = vec![reject(
            "a { box-sizing: PADDING-BOX; }",
            None,
            rejected_message("box-sizing", "PADDING-BOX"),
            (1, 17, 1, 28),
        )];

        Tester::new(
            "declaration-property-value-keyword-no-deprecated",
            json!([true, {"ignoreKeywords": ["/intrinsic$/", "padding-box"]}]),
            &pass,
            &fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn scss_syntax() {
        let pass = vec![
            "$prefix: info;\na { background-color: $prefix + background; }",
            "$prefix: info;\na { background-color: /* a */ $prefix /* b */ + /* c */ background /* d */; }",
            "$postfix: text;\na { background-color: menu + $postfix; }",
        ];

        Tester::new(
            "declaration-property-value-keyword-no-deprecated",
            json!(true),
            &pass,
            [],
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
