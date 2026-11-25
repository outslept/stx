use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        CAMEL_CASE_FUNCTIONS, OptionMatcher, ValueTokenKind, decl_value_span,
        is_standard_syntax_function, is_standard_syntax_value, options_matches,
        parse_option_matchers, tokenize_value,
    },
};
use oxc_css_parser::ast::Declaration;
use serde_json::{Map, Value};

const NAME: &str = FunctionNameCase::NAME;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMode {
    Lower,
    Upper,
}

crate::define_stx_lint! {
    /// Enforces lowercase or uppercase CSS function names in declaration values.
    ///
    /// Set the primary option to `"lower"` or `"upper"`. In `"lower"` mode,
    /// known camel-case spellings such as `translateX` are preserved. Use
    /// `ignoreFunctions` with a literal name, a `/regex/`, or an array of
    /// either. Auto-fix rewrites the name without changing its arguments.
    ///
    /// ```css
    /// a { transform: TranslateX(0); } /* fixes to translateX(0) with "lower" */
    /// ```
    "function-name-case", style, fix = fix;
    pub struct FunctionNameCase {
        mode: CaseMode,
        ignore_functions: Vec<OptionMatcher>,
    }
}

impl FunctionNameCase {
    pub fn new(mode: CaseMode, secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            mode,
            ignore_functions: parse_option_matchers(secondary, "ignoreFunctions"),
        }
    }
}

impl Rule for FunctionNameCase {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let value = ctx.slice(value_span);
        if !value.contains('(') || !is_standard_syntax_value(value) {
            return;
        }
        let mut url_depth = 0usize;
        for token in tokenize_value(ctx.source, value_span) {
            if url_depth > 0 {
                if token.kind == ValueTokenKind::CloseParen {
                    url_depth -= 1;
                } else if token.kind == ValueTokenKind::Function || token.raw == "(" {
                    url_depth += 1;
                }
                continue;
            }
            if token.kind != ValueTokenKind::Function {
                continue;
            }
            let name = token.raw.strip_suffix('(').unwrap();
            url_depth = usize::from(name.eq_ignore_ascii_case("url"));
            if !is_standard_syntax_function(name) || options_matches(&self.ignore_functions, name) {
                continue;
            }
            let expected = match self.mode {
                CaseMode::Lower => CAMEL_CASE_FUNCTIONS
                    .iter()
                    .find(|known| known.eq_ignore_ascii_case(name))
                    .map_or_else(|| name.to_lowercase(), |known| (*known).to_owned()),
                CaseMode::Upper => name.to_uppercase(),
            };
            if name != expected {
                let mut span = token.span;
                span.end -= 1;
                ctx.report_with_fix(
                    NAME,
                    span,
                    format!("Expected \"{name}\" to be \"{expected}\""),
                    Fix::single(span, expected),
                );
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
    fn lower_keeps_canonical_camel_case_and_ignores_configured_names() {
        Tester::new(
            NAME,
            json!(["lower", {"ignoreFunctions": ["someFunction", "/^get/"]}]),
            ["a { transform: skewX(0); color: someFunction(); width: getWidth(); }"],
            [reject(
                "a { transform: TranslateX(0); }",
                "a { transform: translateX(0); }",
                "Expected \"TranslateX\" to be \"translateX\"",
                (1, 16, 1, 26),
            )],
        )
        .with_fix(true)
        .test();
    }

    #[test]
    fn ignores_scss_interpolation() {
        Tester::new(
            NAME,
            json!("upper"),
            ["a { margin: cAlC(5% - #{$size2}); }"],
            [],
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }
}
