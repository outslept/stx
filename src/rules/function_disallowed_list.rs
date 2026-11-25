use oxc_css_parser::Span;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, decl_value_span, options_matches,
        parse_primary_option_matchers, tokenize_value,
    },
};

crate::define_stx_lint! {
    /// Disallows specified function calls in declaration values.
    ///
    /// The primary option accepts a function name, a `/regex/`, or an array
    /// of either. Literal names are case-sensitive; omit the parentheses.
    ///
    /// With `"scale"`:
    ///
    /// ```css
    /// a { transform: rotate(7deg); } /* passes */
    /// a { transform: scale(1); } /* reported */
    /// ```
    "function-disallowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct FunctionDisallowedList {
        disallowed: Vec<OptionMatcher>,
    }
}

impl FunctionDisallowedList {
    pub fn from_config(primary: &serde_json::Value) -> Self {
        Self {
            disallowed: parse_primary_option_matchers(primary),
        }
    }
}

pub fn rejected_message(name: &str) -> String {
    format!("Disallowed function \"{name}\"")
}

impl Rule for FunctionDisallowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(
        &self,
        decl: &'a oxc_css_parser::ast::Declaration<'a>,
        ctx: &mut LintContext<'a>,
    ) {
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        if !ctx.slice(value_span).contains('(') {
            return;
        }

        for token in tokenize_value(ctx.source, value_span) {
            if token.kind != ValueTokenKind::Function {
                continue;
            }
            let name = token.raw.strip_suffix('(').unwrap_or(token.raw);
            if !options_matches(&self.disallowed, name) {
                continue;
            }
            let span = Span {
                start: token.span.start,
                end: token.span.start + name.len() as u32,
            };
            ctx.report(self.name(), span, rejected_message(name));
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
        let pass = vec![
            "a { color: pink; }",
            "a { transform: SCALE(1); }",
            "a { transform: sCaLe(1); }",
            "a { transform: rotate(7deg) }",
            "a { transform: rOtAtE(7deg) }",
            "a { transform: ROTATE(7deg) }",
            "a { background: -webkit-radial-gradient(red, green, blue); }",
            "a { color: color(rgb(0, 0, 0) lightness(50%)); }",
            "@media (max-width: 10px) { a { color: color(rgb(0, 0, 0) lightness(50%)); } }",
            "$scale: (value, value2)",
        ];

        let fail = vec![
            reject(
                "a { transform: scale(1); }",
                None,
                rejected_message("scale"),
                (1, 16, 1, 21),
            ),
            reject(
                "a { transform : scale(1); }",
                None,
                rejected_message("scale"),
                (1, 17, 1, 22),
            ),
            reject(
                "a\n{ transform: scale(1); }",
                None,
                rejected_message("scale"),
                (2, 14, 2, 19),
            ),
            reject(
                "a { transform:    scale(1); }",
                None,
                rejected_message("scale"),
                (1, 19, 1, 24),
            ),
            reject(
                "  a { transform: scale(1); }",
                None,
                rejected_message("scale"),
                (1, 18, 1, 23),
            ),
            reject(
                "a { color: rgba(0, 0, 0, 0) }",
                None,
                rejected_message("rgba"),
                (1, 12, 1, 16),
            ),
            reject(
                "a { color: color(rgba(0, 0, 0, 0) lightness(50%)); }",
                None,
                rejected_message("rgba"),
                (1, 18, 1, 22),
            ),
            reject(
                "a { background: red, -moz-linear-gradient(45deg, blue, red); }",
                None,
                rejected_message("-moz-linear-gradient"),
                (1, 22, 1, 42),
            ),
            reject(
                "@media (max-width: 10px) { a { color: color(rgba(0, 0, 0) lightness(50%)); } }",
                None,
                rejected_message("rgba"),
                (1, 45, 1, 49),
            ),
        ];

        Tester::new(
            "function-disallowed-list",
            json!(["rgba", "scale", "/^(-moz-)?linear-gradient$/"]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn regex_config() {
        let pass = vec!["a { color: hsl(208, 100%, 97%); }"];
        let fail = vec![
            reject(
                "a { color: rgb(0, 0, 0); }",
                None,
                rejected_message("rgb"),
                (1, 12, 1, 15),
            ),
            reject(
                "a { color: rgba(0, 0, 0); }",
                None,
                rejected_message("rgba"),
                (1, 12, 1, 16),
            ),
        ];
        Tester::new("function-disallowed-list", json!("/rgb/"), &pass, &fail).test();
        Tester::new("function-disallowed-list", json!(["/rgb/"]), &pass, &fail).test();
    }

    #[test]
    fn case_sensitive_and_flags() {
        let pass = vec![
            "a { transform: stewX(10deg); }",
            "a { transform: translateY(5px); }",
            "a { transform: scaleX(1); }",
            "a { transform: matrix3d(a1); }",
        ];

        let fail = vec![
            reject(
                "a { transform: skewx(10deg); }",
                None,
                rejected_message("skewx"),
                (1, 16, 1, 21),
            ),
            reject(
                "a { transform: translateX(5px); }",
                None,
                rejected_message("translateX"),
                (1, 16, 1, 26),
            ),
            reject(
                "a { transform: SCALEX(1); }",
                None,
                rejected_message("SCALEX"),
                (1, 16, 1, 22),
            ),
            reject(
                "a { transform: rotatex(60deg); }",
                None,
                rejected_message("rotatex"),
                (1, 16, 1, 23),
            ),
            reject(
                "a { transform: rotateX(60deg); }",
                None,
                rejected_message("rotateX"),
                (1, 16, 1, 23),
            ),
            reject(
                "a { transform: ROTATEX(60deg); }",
                None,
                rejected_message("ROTATEX"),
                (1, 16, 1, 23),
            ),
            reject(
                "a { transform: MATRIX3d(a1); }",
                None,
                rejected_message("MATRIX3d"),
                (1, 16, 1, 24),
            ),
        ];

        Tester::new(
            "function-disallowed-list",
            json!(["skewx", "translateX", "SCALEX", "/rotate/i", "/MATRIX/"]),
            &pass,
            &fail,
        )
        .test();
    }
}
