use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, function_name_case::CaseMode},
    utils::{
        ANIMATION_NAME_KEYWORDS, ANIMATION_SHORTHAND_KEYWORDS, CAMEL_CASE_KEYWORDS,
        COUNTER_INCREMENT_KEYWORDS, COUNTER_RESET_KEYWORDS, FONT_FAMILY_KEYWORDS,
        FONT_SHORTHAND_KEYWORDS, GRID_AREA_KEYWORDS, GRID_COLUMN_KEYWORDS, GRID_ROW_KEYWORDS,
        LIST_STYLE_SHORTHAND_KEYWORDS, LIST_STYLE_TYPE_KEYWORDS, OptionMatcher,
        SYSTEM_COLORS_KEYWORDS, ValueTokenKind, declaration_value_span, in_set_ignore_case,
        is_standard_syntax_value, options_matches, parse_option_matchers, secondary_bool,
        tokenize_value,
    },
};
use oxc_css_parser::ast::{Declaration, InterpolableIdent};
use serde_json::{Map, Value};

const NAME: &str = "value-keyword-case";
const IGNORED_FUNCTIONS: &[&str] = &["url", "var", "counter", "counters", "attr"];

pub struct ValueKeywordCase {
    mode: CaseMode,
    camel_case_svg_keywords: bool,
    ignore_properties: Vec<OptionMatcher>,
    ignore_keywords: Vec<OptionMatcher>,
    ignore_functions: Vec<OptionMatcher>,
}

impl ValueKeywordCase {
    pub fn new(mode: CaseMode, secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            mode,
            camel_case_svg_keywords: secondary_bool(secondary, "camelCaseSvgKeywords")
                .unwrap_or(false),
            ignore_properties: parse_option_matchers(secondary, "ignoreProperties"),
            ignore_keywords: parse_option_matchers(secondary, "ignoreKeywords"),
            ignore_functions: parse_option_matchers(secondary, "ignoreFunctions"),
        }
    }
}

fn is_keyword(property: &str, word: &str) -> bool {
    let contains = |keywords| in_set_ignore_case(keywords, word);
    match property {
        "animation" => contains(ANIMATION_SHORTHAND_KEYWORDS) || contains(ANIMATION_NAME_KEYWORDS),
        "animation-name" => contains(ANIMATION_NAME_KEYWORDS),
        "font" => contains(FONT_SHORTHAND_KEYWORDS) || contains(FONT_FAMILY_KEYWORDS),
        "font-family" => contains(FONT_FAMILY_KEYWORDS),
        "counter-increment" => contains(COUNTER_INCREMENT_KEYWORDS),
        "counter-reset" => contains(COUNTER_RESET_KEYWORDS),
        "grid-row" | "grid-row-start" | "grid-row-end" => contains(GRID_ROW_KEYWORDS),
        "grid-column" | "grid-column-start" | "grid-column-end" => contains(GRID_COLUMN_KEYWORDS),
        "grid-area" => contains(GRID_AREA_KEYWORDS),
        "list-style" => {
            contains(LIST_STYLE_SHORTHAND_KEYWORDS) || contains(LIST_STYLE_TYPE_KEYWORDS)
        }
        "list-style-type" => contains(LIST_STYLE_TYPE_KEYWORDS),
        "counter-set" | "container" | "container-name" | "view-transition-name" => false,
        _ => true,
    }
}

impl Rule for ValueKeywordCase {
    fn name(&self) -> &'static str {
        NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        if options_matches(&self.ignore_properties, ident.name) {
            return;
        }
        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(value_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };
        let value = ctx.slice(value_span);
        if value.contains('#') || !is_standard_syntax_value(value) {
            return;
        }
        let property = ident.name.to_ascii_lowercase();
        let mut depth = 0usize;
        let mut skipped_at = None;
        for token in tokenize_value(ctx.source, value_span) {
            match token.kind {
                ValueTokenKind::Function => {
                    depth += 1;
                    let name = token.raw.trim_end_matches('(');
                    if skipped_at.is_none()
                        && (in_set_ignore_case(IGNORED_FUNCTIONS, name)
                            || options_matches(&self.ignore_functions, name))
                    {
                        skipped_at = Some(depth);
                    }
                }
                ValueTokenKind::Other if token.raw == "(" => depth += 1,
                ValueTokenKind::CloseParen => {
                    if skipped_at == Some(depth) {
                        skipped_at = None;
                    }
                    depth = depth.saturating_sub(1);
                }
                ValueTokenKind::Ident if skipped_at.is_none() => {
                    let mut span = token.span;
                    if span.start > value_span.start
                        && ctx.source.as_bytes()[span.start as usize - 1] == b'!'
                    {
                        span.start -= 1;
                    }
                    let word = ctx.slice(span);
                    if matches!(word, "-")
                        || !is_standard_syntax_value(word)
                        || in_set_ignore_case(SYSTEM_COLORS_KEYWORDS, word)
                        || !is_keyword(&property, word)
                        || options_matches(&self.ignore_keywords, word)
                    {
                        continue;
                    }
                    let expected = match self.mode {
                        CaseMode::Lower if self.camel_case_svg_keywords => CAMEL_CASE_KEYWORDS
                            .iter()
                            .find(|keyword| keyword.eq_ignore_ascii_case(word))
                            .map_or_else(|| word.to_lowercase(), |keyword| (*keyword).to_owned()),
                        CaseMode::Lower => word.to_lowercase(),
                        CaseMode::Upper => word.to_uppercase(),
                    };
                    if word != expected {
                        ctx.report_with_fix(
                            NAME,
                            span,
                            format!("Expected \"{word}\" to be \"{expected}\""),
                            Fix::single(span, expected),
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NAME;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn lower_preserves_hashes_and_custom_identifiers() {
        let pass = [
            "a { color: #FFF; }",
            "a { color: WindowText; }",
            "a { animation: Fancy 1s; font: 14px SomeFont, serif; }",
            "a { display: var(--Size, BLOCK); }",
        ];
        let fail = [
            reject(
                "a { animation: INHERIT; }",
                "a { animation: inherit; }",
                "Expected \"INHERIT\" to be \"inherit\"",
                (1, 16, 1, 23),
            ),
            reject(
                "a { display: block !Import; }",
                "a { display: block !import; }",
                "Expected \"!Import\" to be \"!import\"",
                (1, 20, 1, 27),
            ),
        ];
        Tester::new(NAME, json!("lower"), pass, fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn ignores_and_camel_case_follow_mode() {
        Tester::new(
            NAME,
            json!(["lower", {"ignoreFunctions": ["t"], "ignoreProperties": ["BACKGROUND"], "ignoreKeywords": ["/^Flex$/"]}]),
            ["a { display: t(BLOCK) Flex; BACKGROUND: Red; }"],
            [reject(
                "a { color: RED; }",
                "a { color: red; }",
                "Expected \"RED\" to be \"red\"",
                (1, 12, 1, 15),
            )],
        )
        .with_fix(true)
        .test();
        Tester::new(
            NAME,
            json!(["upper", {"camelCaseSvgKeywords": true}]),
            ["a { color: CURRENTCOLOR; }"],
            [reject(
                "a { color: currentColor; }",
                "a { color: CURRENTCOLOR; }",
                "Expected \"currentColor\" to be \"CURRENTCOLOR\"",
                (1, 12, 1, 24),
            )],
        )
        .with_fix(true)
        .test();
    }
}
