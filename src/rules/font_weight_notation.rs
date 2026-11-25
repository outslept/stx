use oxc_css_parser::ast::Declaration;

use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        FONT_WEIGHT_NON_NUMERIC_KEYWORDS, FONT_WEIGHT_RELATIVE_KEYWORDS, ValueTokenKind,
        decl_value_span, ident_name, is_standard_syntax_value, secondary_has_flag, tokenize_value,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeightMode {
    Numeric,
    NamedWherePossible,
}

pub struct FontWeightNotation {
    mode: FontWeightMode,
    ignore_relative: bool,
}

impl FontWeightNotation {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        let mode = match primary.as_str()? {
            "numeric" => FontWeightMode::Numeric,
            "named-where-possible" => FontWeightMode::NamedWherePossible,
            _ => return None,
        };
        Some(Self {
            mode,
            ignore_relative: secondary_has_flag(secondary, "ignore", "relative"),
        })
    }
}

pub fn expected_message(kind: &str) -> String {
    format!("Expected {kind} font-weight notation")
}

pub fn expected_with_actual_message(actual: &str, expected: &str) -> String {
    format!("Expected \"{actual}\" to be \"{expected}\"")
}

fn is_numbery(value: &str) -> bool {
    !value.trim().is_empty() && value.trim().parse::<f64>().is_ok()
}

fn is_non_numeric_keyword(lower: &str) -> bool {
    FONT_WEIGHT_NON_NUMERIC_KEYWORDS
        .iter()
        .any(|keyword| *keyword == lower)
}

fn is_relative_keyword(lower: &str) -> bool {
    FONT_WEIGHT_RELATIVE_KEYWORDS
        .iter()
        .any(|keyword| *keyword == lower)
}

fn is_div(kind: &ValueTokenKind) -> bool {
    matches!(
        kind,
        ValueTokenKind::Slash | ValueTokenKind::Comma | ValueTokenKind::Colon
    )
}

impl Rule for FontWeightNotation {
    fn name(&self) -> &'static str {
        "font-weight-notation"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        let is_shorthand = prop.eq_ignore_ascii_case("font");
        if !is_shorthand && !prop.eq_ignore_ascii_case("font-weight") {
            return;
        }
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };

        let tokens = tokenize_value(ctx.source, value_span);
        let candidates: Vec<usize> = tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| {
                matches!(token.kind, ValueTokenKind::Ident | ValueTokenKind::Number)
            })
            .map(|(index, _)| index)
            .collect();

        let is_candidate = |index: usize| -> bool {
            let prev_div = index > 0 && is_div(&tokens[index - 1].kind);
            let next_div = index + 1 < tokens.len() && is_div(&tokens[index + 1].kind);
            !prev_div && !next_div
        };

        let has_other = candidates.iter().any(|&index| {
            is_candidate(index) && {
                let raw = tokens[index].raw;
                let lower = raw.to_ascii_lowercase();
                lower != "normal" && (is_numbery(&lower) || is_non_numeric_keyword(&lower))
            }
        });

        for &index in &candidates {
            if !is_candidate(index) {
                continue;
            }
            let token = &tokens[index];
            if is_shorthand {
                if has_other && token.raw.eq_ignore_ascii_case("normal") {
                    continue;
                }
                if self.check_weight(token, ctx) {
                    break;
                }
            }
            self.check_weight(token, ctx);
        }
    }
}

impl FontWeightNotation {
    fn check_weight<'a>(
        &self,
        token: &crate::utils::ValueToken<'a>,
        ctx: &mut LintContext<'a>,
    ) -> bool {
        let weight = token.raw;
        if !is_standard_syntax_value(weight) || weight.to_ascii_lowercase().starts_with("var(") {
            return false;
        }
        let lower = weight.to_ascii_lowercase();
        if self.ignore_relative && is_relative_keyword(&lower) {
            return false;
        }

        match self.mode {
            FontWeightMode::Numeric => {
                if !is_numbery(&lower) && is_non_numeric_keyword(&lower) {
                    if lower == "normal" || lower == "bold" {
                        let numeric = if lower == "normal" { "400" } else { "700" };
                        ctx.report_with_fix(
                            self.name(),
                            token.span,
                            expected_with_actual_message(weight, numeric),
                            Fix::single(token.span, numeric.to_string()),
                        );
                    } else {
                        ctx.report(self.name(), token.span, expected_message("numeric"));
                    }
                    return true;
                }
            }
            FontWeightMode::NamedWherePossible => {
                if is_numbery(&lower) && (lower == "400" || lower == "700") {
                    let named = if lower == "400" { "normal" } else { "bold" };
                    ctx.report_with_fix(
                        self.name(),
                        token.span,
                        expected_with_actual_message(weight, named),
                        Fix::single(token.span, named.to_string()),
                    );
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn numeric() {
        let pass = vec![
            "a { font-weight: $sassy-font-weight; }",
            "a { font: italic @less-666 20px; }",
            "a { font-weight: var(--ff1); }",
            "a { font-weight: 100; }",
            "a { fOnT-wEiGhT: 100; }",
            "a { FONT-WEIGHT: 100; }",
            "a { font-weight: 700; }",
            "a { font-weight: 850; }",
            "a { font-weight: 900; }",
            "a { font: italic small-caps 400 16px/3 cursive; }",
            "a { fOnT: italic small-caps 400 16px/3 cursive; }",
            "a { FONT: italic small-caps 400 16px/3 cursive; }",
            "a { font: italic small-caps 400 16px/500 cursive; }",
            "a { font: italic small-caps 400 16px/500 \"bold font name\"; }",
            "a { font: italic small-caps 400 16px/500 boldfontname; }",
            "a { font: normal 400 normal 16px serif; }",
            "a { font: 400 normal 16px serif; }",
            "a { font: 400 16px serif; }",
            "a { font-weight: inherit; }",
            "a { font-weight: initial; }",
            "a { font-weight: iNiTiAl; }",
            "a { font-weight: INITIAL; }",
            "a { font-weight: /* bold */ 400; }",
            "@font-face { font-weight: 400; }",
            "@font-face { font-weight: 400.5; }",
            "@font-face { font-weight: 400 700; }",
            "@font-face { font-weight: 100.5 400; }",
            "@font-face { font-weight: 400.5 700.5; }",
            "@font-face { font-weight: 400.5 /* 700 */ 700.5; }",
            "@font-face { font-weight: 400.5 /* 700 */700.5; }",
            "@font-face { font-weight: 400 /*700*/700; }",
            "@font-face { font-weight: 400.5/*700*/ 700.5; }",
        ];

        let fail = vec![
            reject(
                "a { font-weight: normal; }",
                "a { font-weight: 400; }",
                expected_with_actual_message("normal", "400"),
                (1, 18, 1, 24),
            ),
            reject(
                "a { fOnT-wEiGhT: normal; }",
                "a { fOnT-wEiGhT: 400; }",
                expected_with_actual_message("normal", "400"),
                (1, 18, 1, 24),
            ),
            reject(
                "a { FONT-WEIGHT: normal; }",
                "a { FONT-WEIGHT: 400; }",
                expected_with_actual_message("normal", "400"),
                (1, 18, 1, 24),
            ),
            reject(
                "a { font-weight: nOrMaL; }",
                "a { font-weight: 400; }",
                expected_with_actual_message("nOrMaL", "400"),
                (1, 18, 1, 24),
            ),
            reject(
                "a { font-weight: NORMAL; }",
                "a { font-weight: 400; }",
                expected_with_actual_message("NORMAL", "400"),
                (1, 18, 1, 24),
            ),
            reject(
                "a { font-weight: /* bold */ normal; }",
                "a { font-weight: /* bold */ 400; }",
                expected_with_actual_message("normal", "400"),
                (1, 29, 1, 35),
            ),
            reject(
                "a { font: italic small-caps bolder 16px/3 cursive; }",
                None,
                expected_message("numeric"),
                (1, 29, 1, 35),
            ),
            reject(
                "a { font: normal 16px/3 cursive; }",
                "a { font: 400 16px/3 cursive; }",
                expected_with_actual_message("normal", "400"),
                (1, 11, 1, 17),
            ),
            reject(
                "a { font: normal normal 16px/3 cursive; }",
                "a { font: 400 normal 16px/3 cursive; }",
                expected_with_actual_message("normal", "400"),
                (1, 11, 1, 17),
            ),
            reject(
                "a { font: normal normal normal 16px/3 cursive; }",
                "a { font: 400 normal normal 16px/3 cursive; }",
                expected_with_actual_message("normal", "400"),
                (1, 11, 1, 17),
            ),
            reject(
                "a { font: normal bold 16px/3 cursive; }",
                "a { font: normal 700 16px/3 cursive; }",
                expected_with_actual_message("bold", "700"),
                (1, 18, 1, 22),
            ),
            reject_warnings(
                "@font-face { font-weight: normal bold; }",
                "@font-face { font-weight: 400 700; }",
                vec![
                    (
                        expected_with_actual_message("normal", "400"),
                        (1, 27, 1, 33),
                    ),
                    (expected_with_actual_message("bold", "700"), (1, 34, 1, 38)),
                ],
            ),
            reject(
                "@font-face { font-weight: 400 bold; }",
                "@font-face { font-weight: 400 700; }",
                expected_with_actual_message("bold", "700"),
                (1, 31, 1, 35),
            ),
            reject(
                "@font-face { font-weight: normal 700; }",
                "@font-face { font-weight: 400 700; }",
                expected_with_actual_message("normal", "400"),
                (1, 27, 1, 33),
            ),
            reject(
                "@font-face { font-weight: /* 400 */ normal 700; }",
                "@font-face { font-weight: /* 400 */ 400 700; }",
                expected_with_actual_message("normal", "400"),
                (1, 37, 1, 43),
            ),
            reject(
                "@font-face { font-weight: normal /* 400 */700; }",
                "@font-face { font-weight: 400 /* 400 */700; }",
                expected_with_actual_message("normal", "400"),
                (1, 27, 1, 33),
            ),
            reject(
                "@font-face { font-weight: normal/* 400 */ 700; }",
                "@font-face { font-weight: 400/* 400 */ 700; }",
                expected_with_actual_message("normal", "400"),
                (1, 27, 1, 33),
            ),
        ];

        Tester::new("font-weight-notation", json!(["numeric"]), &pass, &fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn numeric_ignore_relative() {
        let pass = vec![
            "a { font-weight: 700; }",
            "a { font-weight: bolder; }",
            "a { font-weight: bOlDeR; }",
            "a { font-weight: BOLDER; }",
            "a { font-weight: lighter; }",
            "a { font: italic small-caps lighter 16px/3 cursive; }",
            "a { font: normal lighter 16px/3 cursive; }",
        ];

        let fail = vec![reject(
            "a { font-weight: normal; }",
            "a { font-weight: 400; }",
            expected_with_actual_message("normal", "400"),
            (1, 18, 1, 24),
        )];

        Tester::new(
            "font-weight-notation",
            json!(["numeric", {"ignore": ["relative"]}]),
            &pass,
            &fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn named_where_possible() {
        let pass = vec![
            "a { font-weight: $sassy-font-weight; }",
            "a { font: italic @less-666 20px; }",
            "a { font-weight: var(--ff1); }",
            "a { font-weight: bold; }",
            "a { font-weight: BoLd; }",
            "a { font-weight: BOLD; }",
            "a { font-weight: bolder; }",
            "a { font-weight: normal; }",
            "a { font-weight: lighter; }",
            "a { font: italic small-caps bold 16px/500 cursive; }",
            "a { font: italic small-caps bold 16px/500 \"cursive 100 font\"; }",
            "a { font: italic small-caps bold 16px/500 100cursivefont; }",
            "a { font: normal 16px/3 cursive; }",
            "a { font: normal normal 16px/3 cursive; }",
            "a { font: normal normal normal 16px/3 cursive; }",
            "a { font-weight: 300; }",
            "a { font: italic small-caps 600 16px/3 cursive; }",
            "a { font-weight: inherit; }",
            "a { font-weight: iNhErIt; }",
            "a { font-weight: INHERIT; }",
        ];

        let fail = vec![
            reject(
                "a { font-weight: 400; }",
                "a { font-weight: normal; }",
                expected_with_actual_message("400", "normal"),
                (1, 18, 1, 21),
            ),
            reject(
                "a { font: italic small-caps 700 16px/3 cursive; }",
                "a { font: italic small-caps bold 16px/3 cursive; }",
                expected_with_actual_message("700", "bold"),
                (1, 29, 1, 32),
            ),
            reject(
                "a { font: normal 400 normal 16px serif; }",
                "a { font: normal normal normal 16px serif; }",
                expected_with_actual_message("400", "normal"),
                (1, 18, 1, 21),
            ),
            reject(
                "a { font: 400 normal 16px serif; }",
                "a { font: normal normal 16px serif; }",
                expected_with_actual_message("400", "normal"),
                (1, 11, 1, 14),
            ),
            reject(
                "a { font: 400 16px serif; }",
                "a { font: normal 16px serif; }",
                expected_with_actual_message("400", "normal"),
                (1, 11, 1, 14),
            ),
        ];

        Tester::new(
            "font-weight-notation",
            json!(["named-where-possible"]),
            &pass,
            &fail,
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn scss_syntax() {
        let pass = vec![
            "@mixin font-face($weight) { a { font-weight: #{$weight}; } }",
            "a { font-weight: $foo }",
            "a { font-weight: map-deep-get($theme, typography, weight, semibold); }",
            "a { font-weight: /*comment*/ map-deep-get($theme, typography, weight, semibold); }",
        ];

        Tester::new("font-weight-notation", json!(["numeric"]), &pass, [])
            .with_custom_syntax("postcss-scss")
            .test();
    }
}
