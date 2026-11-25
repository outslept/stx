use oxc_css_parser::{
    Span,
    ast::{Declaration, Statement},
};

use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, decl_value_span, ident_name, is_custom_property,
        options_matches, parse_option_matchers, parse_primary_option_matchers, tokenize_value,
    },
};

crate::define_stx_lint! {
    /// Restricts function calls in declaration values to an allow list.
    ///
    /// The primary option accepts a function name, a `/regex/`, or an array
    /// of either. Literal names are case-sensitive; omit the parentheses.
    /// `exceptWithoutPropertyFallback` takes the same kind of matchers and
    /// requires a prior declaration of the same property in the same block
    /// for matching allowed functions. Custom properties cannot satisfy this
    /// fallback requirement. To use both a list and secondary options, use
    /// `[["rgb", "calc"], {"exceptWithoutPropertyFallback": "calc"}]`.
    ///
    /// ```css
    /// a { width: 8px; width: calc(4px + 4px); } /* passes with that setting */
    /// a { width: calc(4px + 4px); } /* reported: no fallback */
    /// ```
    "function-allowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct FunctionAllowedList {
        allowed: Vec<OptionMatcher>,
        except_without_property_fallback: Vec<OptionMatcher>,
    }
}

impl FunctionAllowedList {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        Self {
            allowed: parse_primary_option_matchers(primary),
            except_without_property_fallback: parse_option_matchers(
                secondary,
                "exceptWithoutPropertyFallback",
            ),
        }
    }
}

pub fn rejected_message(name: &str) -> String {
    format!("Disallowed function \"{name}\"")
}

impl Rule for FunctionAllowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        if !ctx.slice(value_span).contains('(') {
            return;
        }

        let has_fallback = if self.except_without_property_fallback.is_empty() {
            false
        } else {
            has_prev_property_declaration(decl, ctx)
        };

        for token in tokenize_value(ctx.source, value_span) {
            if token.kind != ValueTokenKind::Function {
                continue;
            }
            let name = token.raw.strip_suffix('(').unwrap_or(token.raw);
            if options_matches(&self.allowed, name)
                && (!options_matches(&self.except_without_property_fallback, name) || has_fallback)
            {
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

fn has_prev_property_declaration(decl: &Declaration<'_>, ctx: &LintContext<'_>) -> bool {
    let Some(prop) = ident_name(&decl.name) else {
        return false;
    };
    if is_custom_property(prop) {
        return false;
    }

    let statements = ctx.ancestors.iter().rev().find_map(|parent| match parent {
        AstParent::QualifiedRule(rule) => Some(&rule.block.statements),
        AstParent::AtRule(at_rule) => at_rule.block.as_ref().map(|block| &block.statements),
        AstParent::KeyframeBlock(block) => Some(&block.block.statements),
    });
    let Some(statements) = statements else {
        return false;
    };

    let position = statements.iter().position(|statement| {
        matches!(statement, Statement::Declaration(candidate) if candidate.span.start == decl.span.start && candidate.span.end == decl.span.end)
    });
    let Some(position) = position else {
        return false;
    };

    statements[..position].iter().rev().any(|statement| {
        matches!(statement, Statement::Declaration(candidate) if ident_name(&candidate.name).is_some_and(|name| name.eq_ignore_ascii_case(prop)))
    })
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
            "a { transform: rotate(7deg) }",
            "a { background: -webkit-radial-gradient(red, green, blue); }",
            "a { color: color(rgb(0, 0, 0) lightness(50%)); }",
            "@media (max-width: 10px) { a { color: color(rgb(0, 0, 0) lightness(50%)); } }",
            "$list: (value, value2)",
        ];

        let fail = vec![
            reject(
                "a { transform: rOtAtE(7deg) }",
                None,
                rejected_message("rOtAtE"),
                (1, 16, 1, 22),
            ),
            reject(
                "a { transform: ROTATE(7deg) }",
                None,
                rejected_message("ROTATE"),
                (1, 16, 1, 22),
            ),
            reject(
                "a { transform: scale(1); }",
                None,
                rejected_message("scale"),
                (1, 16, 1, 21),
            ),
            reject(
                "a { transform: sCaLe(1); }",
                None,
                rejected_message("sCaLe"),
                (1, 16, 1, 21),
            ),
            reject(
                "a { transform: SCALE(1); }",
                None,
                rejected_message("SCALE"),
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
            "function-allowed-list",
            json!([
                "rotate",
                "rgb",
                "/^(-webkit-)?radial-gradient$/",
                "lightness",
                "color"
            ]),
            &pass,
            &fail,
        )
        .test();
    }

    #[test]
    fn single_string_config() {
        let fail = vec![reject(
            "a { transform: scale(4); }",
            None,
            rejected_message("scale"),
            (1, 16, 1, 21),
        )];

        Tester::new(
            "function-allowed-list",
            json!(["translate"]),
            ["a { transform: translate(1px); }"],
            &fail,
        )
        .test();

        let fail = vec![reject(
            "a { color: hsl(208, 100%, 97%); }",
            None,
            rejected_message("hsl"),
            (1, 12, 1, 15),
        )];
        let pass = vec![
            "a { color: rgb(0, 0, 0); }",
            "a { color: rgba(0, 0, 0, 0); }",
        ];
        Tester::new("function-allowed-list", json!("/rgb/"), &pass, &fail).test();
        Tester::new("function-allowed-list", json!(["/rgb/"]), &pass, &fail).test();
    }

    #[test]
    fn except_without_property_fallback() {
        let pass = vec![
            "a { transform: scale(1); }",
            "a { width: 10px; width: min(50%, 10px); }",
            "a { height: 10px; height: max(10px, 50%); }",
            "a { width: 10px; height: 10px; width: min(50%, 10px); }",
        ];

        let fail = vec![
            reject(
                "a { width: min(50%, 10px); }",
                None,
                rejected_message("min"),
                (1, 12, 1, 15),
            ),
            reject(
                "a { width: max(10px, 50%); }",
                None,
                rejected_message("max"),
                (1, 12, 1, 15),
            ),
            reject(
                "a { width: min(50%, 10px); width: 10px; }",
                None,
                rejected_message("min"),
                (1, 12, 1, 15),
            ),
            reject(
                "a { width: 1px; } b { width: min(50%, 10px); }",
                None,
                rejected_message("min"),
                (1, 30, 1, 33),
            ),
            reject(
                "a { --foo: min(50%, 10px); }",
                None,
                rejected_message("min"),
                (1, 12, 1, 15),
            ),
            reject(
                "a { --foo: 10px; --foo: min(50%, 10px); }",
                None,
                rejected_message("min"),
                (1, 25, 1, 28),
            ),
            reject(
                ":root { --foo: 10px; } a { --foo: min(50%, 10px); }",
                None,
                rejected_message("min"),
                (1, 35, 1, 38),
            ),
        ];

        Tester::new(
            "function-allowed-list",
            json!([
                ["scale", "min", "max"],
                {"exceptWithoutPropertyFallback": ["/min/", "max"]}
            ]),
            &pass,
            &fail,
        )
        .test();
    }
}
