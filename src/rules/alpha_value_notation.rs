use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        OptionMatcher, ValueToken, ValueTokenKind, decl_value_span, ident_name, in_set_ignore_case,
        is_standard_syntax_value, options_matches, parse_option_matchers, sass_variable_value_span,
        tokenize_value,
    },
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, SassVariableDeclaration},
};
use serde::Deserialize;
use serde_json::{Map, Value};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlphaMode {
    #[default]
    Number,
    Percentage,
}
crate::define_stx_lint! {
    /// Require numbers or percentages for literal alpha values.
    ///
    /// Set the primary option to `"number"` or `"percentage"`. The secondary
    /// `exceptProperties` list reverses that choice for matching property names
    /// or slash-delimited regexes. The rule checks opacity properties and alpha
    /// channels in recognized color functions, including in Sass variable values.
    /// It fixes numeric literals, not expressions such as `var(--alpha)`.
    ///
    /// With `"percentage"`:
    ///
    /// ```css
    /// a { opacity: 0.5; } /* reported */
    /// a { opacity: 50%; } /* accepted */
    /// ```
    "alpha-value-notation", style, fix = fix,
    short_description = "Specify percentage or number notation for alpha-values.";
    #[derive(Debug, Default)]
    pub struct AlphaValueNotation {
        pub mode: AlphaMode,
        pub except_properties: Vec<OptionMatcher>,
    }
}
const ALPHA_PROPS: &[&str] = &[
    "opacity",
    "shape-image-threshold",
    "fill-opacity",
    "flood-opacity",
    "stop-opacity",
    "stroke-opacity",
];
const ALPHA_FUNCTION_NAMES: &[&str] = &[
    "color", "hsl", "hsla", "rgb", "rgba", "hwb", "lab", "lch", "oklab", "oklch",
];
pub fn expected(unfixed: &str, fixed: &str) -> String {
    format!("Expected \"{unfixed}\" to be \"{fixed}\"")
}
impl AlphaValueNotation {
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Self {
        let mode = serde_json::from_value::<AlphaMode>(primary.clone()).unwrap_or_default();
        let except_properties = parse_option_matchers(secondary, "exceptProperties");
        Self {
            mode,
            except_properties,
        }
    }
    fn effective_mode(&self, prop_name: &str) -> AlphaMode {
        if options_matches(&self.except_properties, prop_name) {
            match self.mode {
                AlphaMode::Number => AlphaMode::Percentage,
                AlphaMode::Percentage => AlphaMode::Number,
            }
        } else {
            self.mode
        }
    }
    fn check_alpha_token<'a>(
        &self,
        token: &ValueToken<'a>,
        prop_name: &str,
        ctx: &mut LintContext<'a>,
    ) {
        if !is_standard_syntax_value(token.raw) {
            return;
        }
        if !matches!(
            token.kind,
            ValueTokenKind::Number | ValueTokenKind::Percentage
        ) {
            return;
        }
        let expectation = self.effective_mode(prop_name);
        match (expectation, &token.kind) {
            (AlphaMode::Number, ValueTokenKind::Number)
            | (AlphaMode::Percentage, ValueTokenKind::Percentage) => {}
            (AlphaMode::Number, ValueTokenKind::Percentage) => {
                if let Some(fixed) = as_number(token.raw) {
                    let msg = expected(token.raw, &fixed);
                    ctx.report_with_fix(
                        Self::NAME,
                        token.span,
                        msg,
                        Fix::single(token.span, fixed),
                    );
                }
            }
            (AlphaMode::Percentage, ValueTokenKind::Number) => {
                if let Some(fixed) = as_percentage(token.raw) {
                    let msg = expected(token.raw, &fixed);
                    ctx.report_with_fix(
                        Self::NAME,
                        token.span,
                        msg,
                        Fix::single(token.span, fixed),
                    );
                }
            }
            _ => {}
        }
    }
}
impl Rule for AlphaValueNotation {
    fn name(&self) -> &'static str {
        Self::NAME
    }
    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop_name) = ident_name(&decl.name) else {
            return;
        };
        let Some(val_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        self.check_value(prop_name, val_span, ctx);
    }

    fn check_sass_variable_declaration<'a>(
        &self,
        decl: &'a SassVariableDeclaration<'a>,
        ctx: &mut LintContext<'a>,
    ) {
        if let Some(span) = sass_variable_value_span(ctx.source, decl) {
            self.check_value(ctx.slice(decl.name.span), span, ctx);
        }
    }
}

impl AlphaValueNotation {
    fn check_value(&self, prop_name: &str, val_span: Span, ctx: &mut LintContext<'_>) {
        let val_raw = ctx.slice_range(val_span.start, val_span.end);
        if !val_raw.bytes().any(|b| b.is_ascii_digit()) {
            return;
        }
        let is_alpha_prop = in_set_ignore_case(ALPHA_PROPS, prop_name);
        let tokens = tokenize_value(ctx.source, val_span);
        for (i, tok) in tokens.iter().enumerate() {
            if is_alpha_prop
                && matches!(
                    tok.kind,
                    ValueTokenKind::Number | ValueTokenKind::Percentage
                )
            {
                self.check_alpha_token(tok, prop_name, ctx);
            } else if tok.kind == ValueTokenKind::Function
                && in_set_ignore_case(
                    ALPHA_FUNCTION_NAMES,
                    tok.raw.strip_suffix('(').unwrap_or(tok.raw),
                )
            {
                let close_idx = find_closing_paren(&tokens, i);
                if let Some(alpha_tok) = find_alpha_in_function(&tokens[i + 1..close_idx]) {
                    self.check_alpha_token(alpha_tok, prop_name, ctx);
                }
            }
        }
    }
}
#[inline]
fn is_open_paren(tok: &ValueToken<'_>) -> bool {
    tok.kind == ValueTokenKind::Function || tok.raw == "("
}
fn find_closing_paren(tokens: &[ValueToken<'_>], open_idx: usize) -> usize {
    let mut depth = 1usize;
    let mut i = open_idx + 1;
    while i < tokens.len() {
        if is_open_paren(&tokens[i]) {
            depth += 1;
        } else if tokens[i].kind == ValueTokenKind::CloseParen {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
        i += 1;
    }
    tokens.len()
}
fn skip_parenthesized(tokens: &[ValueToken<'_>], open_idx: usize) -> usize {
    let close_idx = find_closing_paren(tokens, open_idx);
    (close_idx + 1).min(tokens.len())
}
fn has_top_level_comma(args: &[ValueToken<'_>]) -> bool {
    let mut depth = 0usize;
    for tok in args {
        if is_open_paren(tok) {
            depth += 1;
        } else if tok.kind == ValueTokenKind::CloseParen {
            depth = depth.saturating_sub(1);
        } else if tok.kind == ValueTokenKind::Comma && depth == 0 {
            return true;
        }
    }
    false
}
fn is_word_boundary(tok: &ValueToken<'_>) -> bool {
    is_open_paren(tok)
        || matches!(
            tok.kind,
            ValueTokenKind::Comma
                | ValueTokenKind::Slash
                | ValueTokenKind::String
                | ValueTokenKind::CloseParen
        )
}
fn find_alpha_in_function<'t, 'a>(args: &'t [ValueToken<'a>]) -> Option<&'t ValueToken<'a>> {
    if has_top_level_comma(args) {
        let mut count = 0usize;
        let mut fourth_tok: Option<&'t ValueToken<'a>> = None;
        let mut i = 0usize;
        while i < args.len() {
            if matches!(
                args[i].kind,
                ValueTokenKind::Comma | ValueTokenKind::Slash | ValueTokenKind::String
            ) {
                i += 1;
            } else if is_open_paren(&args[i]) {
                count += 1;
                i = skip_parenthesized(args, i);
            } else {
                let start = i;
                i += 1;
                while i < args.len()
                    && !is_word_boundary(&args[i])
                    && args[i - 1].span.end == args[i].span.start
                {
                    i += 1;
                }
                count += 1;
                if count == 4 && i == start + 1 {
                    fourth_tok = Some(&args[start]);
                }
            }
        }
        return if count == 4 { fourth_tok } else { None };
    }
    let mut i = 0usize;
    let mut found_slash = false;
    while i < args.len() {
        if is_open_paren(&args[i]) {
            i = skip_parenthesized(args, i);
        } else if args[i].kind == ValueTokenKind::Slash {
            found_slash = true;
            i += 1;
            break;
        } else {
            i += 1;
        }
    }
    if !found_slash {
        return None;
    }
    while i < args.len() {
        if matches!(
            args[i].kind,
            ValueTokenKind::Comma | ValueTokenKind::Slash | ValueTokenKind::String
        ) {
            i += 1;
        } else if is_open_paren(&args[i]) {
            i = skip_parenthesized(args, i);
        } else {
            let start = i;
            i += 1;
            while i < args.len()
                && !is_word_boundary(&args[i])
                && args[i - 1].span.end == args[i].span.start
            {
                i += 1;
            }
            return (i == start + 1).then_some(&args[start]);
        }
    }
    None
}
fn as_percentage(value: &str) -> Option<String> {
    let number: f64 = value.parse().ok()?;
    Some(format!("{}%", format_float(number * 100.0)))
}
fn as_number(value: &str) -> Option<String> {
    let num_str = value.strip_suffix('%')?;
    let number: f64 = num_str.parse().ok()?;
    Some(format_float(number / 100.0))
}
fn format_float(n: f64) -> String {
    let clean = if n == 0.0 || !n.is_finite() { 0.0 } else { n };
    let formatted = format!("{clean:.6}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{FailCase, Tester, reject};
    use serde_json::json;
    fn reject_case<'a>(
        code: &'a str,
        fixed: &'a str,
        message: String,
        (line, column, end_line, end_column): (usize, usize, usize, usize),
    ) -> FailCase<'a> {
        reject(
            code,
            Some(fixed),
            message,
            (line, column, end_line, end_column),
        )
    }
    #[test]
    fn test_number_mode_default() {
        let pass = vec![
            "a { opacity: 0 }",
            "a { opacity: 0.5 }",
            "a { opacity: 1 }",
            "a { fill-opacity: 0.25 }",
            "a { shape-image-threshold: 0.7 }",
            "a { flood-opacity: 0.8 }",
            "a { stop-opacity: 0.9 }",
            "a { stroke-opacity: 0.1 }",
            "a { color: rgb(0 0 0 / 0.5) }",
            "a { color: rgba(0, 0, 0, 0.5) }",
            "a { color: hsl(0 0% 0% / 0.5) }",
            "a { color: hsla(0, 0%, 0%, 0.5) }",
            "a { color: hwb(0 0% 0% / 0.5) }",
            "a { color: lab(0% 0 0 / 0.5) }",
            "a { color: lch(0% 0 0 / 0.5) }",
            "a { color: oklab(0% 0 0 / 0.5) }",
            "a { color: oklch(0% 0 0 / 0.5) }",
            "a { color: color(srgb 0 0 0 / 0.5) }",
            "a { color: rgb(0 0 0) }",
            "a { color: hsl(0, 0%, 50%) }",
            "a { opacity: var(--alpha) }",
            "a { opacity: var(--alpha, 0.5) }",
            "a { color: rgba(0, 0, 0, var(--alpha)) }",
            "a { color: rgb(0 0 0 / var(--alpha)) }",
            "a { color: rgb(0, 0, 0, var(--alpha, 50%)) }",
            "a { width: 50% }",
            "a { background: linear-gradient(50%, red, blue) }",
            "a { color: rgb(0, 0, 0, 50%, 50%) }",
        ];
        let fail = vec![
            reject_case(
                "a { opacity: 10% }",
                "a { opacity: 0.1 }",
                expected("10%", "0.1"),
                (1, 14, 1, 17),
            ),
            reject_case(
                "a { opacity: 50% }",
                "a { opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 14, 1, 17),
            ),
            reject_case(
                "a { shape-image-threshold: 50% }",
                "a { shape-image-threshold: 0.5 }",
                expected("50%", "0.5"),
                (1, 28, 1, 31),
            ),
            reject_case(
                "a { fill-opacity: 50% }",
                "a { fill-opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 19, 1, 22),
            ),
            reject_case(
                "a { flood-opacity: 50% }",
                "a { flood-opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 20, 1, 23),
            ),
            reject_case(
                "a { stop-opacity: 50% }",
                "a { stop-opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 19, 1, 22),
            ),
            reject_case(
                "a { stroke-opacity: 50% }",
                "a { stroke-opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 21, 1, 24),
            ),
            reject_case(
                "a { opacity: 14.3% }",
                "a { opacity: 0.143 }",
                expected("14.3%", "0.143"),
                (1, 14, 1, 19),
            ),
            reject_case(
                "a { opacity: 14.36% }",
                "a { opacity: 0.1436 }",
                expected("14.36%", "0.1436"),
                (1, 14, 1, 20),
            ),
            reject_case(
                "a { color: rgb(0 0 0 / 50%) }",
                "a { color: rgb(0 0 0 / 0.5) }",
                expected("50%", "0.5"),
                (1, 24, 1, 27),
            ),
            reject_case(
                "a { color: rgba(0, 0, 0, 50%) }",
                "a { color: rgba(0, 0, 0, 0.5) }",
                expected("50%", "0.5"),
                (1, 26, 1, 29),
            ),
            reject_case(
                "a { color: hsl(0 0% 0% / 50%) }",
                "a { color: hsl(0 0% 0% / 0.5) }",
                expected("50%", "0.5"),
                (1, 26, 1, 29),
            ),
            reject_case(
                "a { opacity: var(--alpha, 50%) }",
                "a { opacity: var(--alpha, 0.5) }",
                expected("50%", "0.5"),
                (1, 27, 1, 30),
            ),
            reject_case(
                "a { opacity: +50% }",
                "a { opacity: 0.5 }",
                expected("+50%", "0.5"),
                (1, 14, 1, 18),
            ),
            reject_case(
                "a { OPACITY: 50% }",
                "a { OPACITY: 0.5 }",
                expected("50%", "0.5"),
                (1, 14, 1, 17),
            ),
            reject_case(
                "a { COLOR: RGB(0 0 0 / 50%) }",
                "a { COLOR: RGB(0 0 0 / 0.5) }",
                expected("50%", "0.5"),
                (1, 24, 1, 27),
            ),
        ];
        Tester::new("alpha-value-notation", Some(json!("number")), pass, fail).test();
    }
    #[test]
    fn test_percentage_mode() {
        let pass = vec![
            "a { opacity: 0% }",
            "a { opacity: 50% }",
            "a { opacity: 100% }",
            "a { color: rgb(0 0 0 / 50%) }",
            "a { color: rgba(0, 0, 0, 50%) }",
            "a { color: hsl(0 0% 0% / 50%) }",
            "a { line-height: 1.5 }",
            "a { z-index: 1 }",
        ];
        let fail = vec![
            reject_case(
                "a { opacity: 0.1 }",
                "a { opacity: 10% }",
                expected("0.1", "10%"),
                (1, 14, 1, 17),
            ),
            reject_case(
                "a { opacity: 0.5 }",
                "a { opacity: 50% }",
                expected("0.5", "50%"),
                (1, 14, 1, 17),
            ),
            reject_case(
                "a { opacity: .5 }",
                "a { opacity: 50% }",
                expected(".5", "50%"),
                (1, 14, 1, 16),
            ),
            reject_case(
                "a { opacity: 0 }",
                "a { opacity: 0% }",
                expected("0", "0%"),
                (1, 14, 1, 15),
            ),
            reject_case(
                "a { opacity: 1 }",
                "a { opacity: 100% }",
                expected("1", "100%"),
                (1, 14, 1, 15),
            ),
            reject_case(
                "a { color: rgb(0 0 0 / 0.5) }",
                "a { color: rgb(0 0 0 / 50%) }",
                expected("0.5", "50%"),
                (1, 24, 1, 27),
            ),
            reject_case(
                "a { color: rgba(0, 0, 0, 0.25) }",
                "a { color: rgba(0, 0, 0, 25%) }",
                expected("0.25", "25%"),
                (1, 26, 1, 30),
            ),
        ];
        Tester::new(
            "alpha-value-notation",
            Some(json!("percentage")),
            pass,
            fail,
        )
        .test();
    }
    #[test]
    fn test_except_properties_string_and_regex() {
        let config = json!(["number", { "exceptProperties": ["opacity", "/^fill-/"] }]);
        let pass = vec![
            "a { opacity: 50% }",
            "a { fill-opacity: 50% }",
            "a { stop-opacity: 0.5 }",
            "a { color: rgb(0 0 0 / 0.5) }",
        ];
        let fail = vec![
            reject_case(
                "a { opacity: 0.5 }",
                "a { opacity: 50% }",
                expected("0.5", "50%"),
                (1, 14, 1, 17),
            ),
            reject_case(
                "a { fill-opacity: 0.5 }",
                "a { fill-opacity: 50% }",
                expected("0.5", "50%"),
                (1, 19, 1, 22),
            ),
            reject_case(
                "a { stop-opacity: 50% }",
                "a { stop-opacity: 0.5 }",
                expected("50%", "0.5"),
                (1, 19, 1, 22),
            ),
        ];
        Tester::new("alpha-value-notation", Some(config), pass, fail).test();
    }
    #[test]
    fn test_relative_color_alpha() {
        Tester::new(
            "alpha-value-notation",
            Some(json!("number")),
            [],
            [
                reject(
                    "a { color: hsl(from var(--base) h s l / 50%) }",
                    Some("a { color: hsl(from var(--base) h s l / 0.5) }"),
                    expected("50%", "0.5"),
                    (1, 41, 1, 44),
                ),
                reject(
                    "a { color: color(from red srgb r g b / 25%) }",
                    Some("a { color: color(from red srgb r g b / 0.25) }"),
                    expected("25%", "0.25"),
                    (1, 40, 1, 43),
                ),
            ],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn test_preprocessor_ignores() {
        let pass = vec![
            "a { opacity: $alpha }",
            "a { opacity: @alpha }",
            "a { color: rgba(0, 0, 0, #{$alpha}) }",
            "a { color: rgba(0, 0, 0, $alpha) }",
        ];
        let fail = vec![];
        Tester::new("alpha-value-notation", Some(json!("number")), pass, fail).test();
    }

    #[test]
    fn checks_scss_variables() {
        let fail = [reject(
            "$a: rgb(0 0 0 / 50%);",
            "$a: rgb(0 0 0 / 0.5);",
            "Expected \"50%\" to be \"0.5\"",
            (1, 17, 1, 20),
        )];
        Tester::new("alpha-value-notation", json!("number"), [], fail)
            .with_custom_syntax("postcss-scss")
            .with_fix(true)
            .test();
    }
}
