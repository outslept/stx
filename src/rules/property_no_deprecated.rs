use oxc_css_parser::{Span, ast::Declaration};

use crate::{
    context::LintContext,
    fix::{Edit, Fix},
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, decl_value_span, ident_name, options_matches,
        parse_option_matchers, tokenize_value,
    },
};

const DEPRECATED_PROPS: &[(&str, Option<&str>)] = &[
    ("-khtml-box-align", Some("align-items")),
    ("-khtml-box-direction", None),
    ("-khtml-box-flex", Some("flex-grow")),
    ("-khtml-box-lines", None),
    ("-khtml-box-ordinal-group", Some("order")),
    ("-khtml-box-orient", None),
    ("-khtml-box-pack", None),
    ("-khtml-user-modify", None),
    ("-moz-box-align", Some("align-items")),
    ("-moz-box-direction", None),
    ("-moz-box-flex", Some("flex-grow")),
    ("-moz-box-lines", None),
    ("-moz-box-ordinal-group", Some("order")),
    ("-moz-box-orient", None),
    ("-moz-box-pack", None),
    ("-moz-user-modify", None),
    ("-ms-box-align", Some("align-items")),
    ("-ms-box-direction", None),
    ("-ms-box-flex", Some("flex-grow")),
    ("-ms-box-lines", None),
    ("-ms-box-ordinal-group", Some("order")),
    ("-ms-box-orient", None),
    ("-ms-box-pack", None),
    ("-webkit-box-align", Some("align-items")),
    ("-webkit-box-direction", None),
    ("-webkit-box-flex", Some("flex-grow")),
    ("-webkit-box-lines", None),
    ("-webkit-box-ordinal-group", Some("order")),
    ("-webkit-box-orient", None),
    ("-webkit-box-pack", None),
    ("-webkit-user-modify", None),
    ("grid-column-gap", Some("column-gap")),
    ("grid-gap", Some("gap")),
    ("grid-row-gap", Some("row-gap")),
    ("ime-mode", None),
    ("page-break-after", Some("break-after")),
    ("page-break-before", Some("break-before")),
    ("page-break-inside", Some("break-inside")),
    ("position-try-options", Some("position-try-fallbacks")),
    ("scroll-snap-coordinate", None),
    ("scroll-snap-destination", None),
    ("scroll-snap-margin-bottom", Some("scroll-margin-bottom")),
    ("scroll-snap-margin-left", Some("scroll-margin-left")),
    ("scroll-snap-margin-right", Some("scroll-margin-right")),
    ("scroll-snap-margin-top", Some("scroll-margin-top")),
    ("scroll-snap-margin", Some("scroll-margin")),
    ("scroll-snap-points-x", None),
    ("scroll-snap-points-y", None),
    ("scroll-snap-type-x", None),
    ("scroll-snap-type-y", None),
    ("word-wrap", Some("overflow-wrap")),
    ("clip", None),
];

crate::define_stx_lint! {
    /// Report deprecated properties and replace those with a known equivalent.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/property-no-deprecated>
    ///
    /// Enable with `true`. `ignoreProperties` accepts exact names or
    /// slash-delimited regexes. Properties without a mapped replacement, such
    /// as `clip`, are reported without a fix. `-webkit-box-orient: vertical`
    /// is exempt. The fix also changes `always` to `page` for `page-break-before`
    /// and `page-break-after`.
    ///
    /// ```css
    /// .grid { grid-gap: 1rem; } /* Fixes to gap */
    /// ```
    "property-no-deprecated", correctness, fix = conditional;
    #[derive(Debug, Default)]
    pub struct PropertyNoDeprecated {
        ignore: Vec<OptionMatcher>,
    }
}

impl PropertyNoDeprecated {
    pub fn new(secondary: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        Self {
            ignore: parse_option_matchers(secondary, "ignoreProperties"),
        }
    }
}

pub fn expected_message(unfixed: &str, fixed: &str) -> String {
    format!("Expected \"{unfixed}\" to be \"{fixed}\"")
}

pub fn rejected_message(property: &str) -> String {
    format!("Deprecated property \"{property}\"")
}

impl Rule for PropertyNoDeprecated {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if !ctx.is_standard_declaration(decl) || options_matches(&self.ignore, prop) {
            return;
        }
        let normalized = prop.to_ascii_lowercase();
        let Some(&(_, mapped)) = DEPRECATED_PROPS
            .iter()
            .find(|(name, _)| *name == normalized)
        else {
            return;
        };

        if normalized == "-webkit-box-orient"
            && let Some(value_span) = decl_value_span(ctx.source, decl)
            && ctx.slice(value_span).eq_ignore_ascii_case("vertical")
        {
            return;
        }

        let prop_span = Span {
            start: decl.span.start,
            end: decl.span.start + prop.len() as u32,
        };

        let Some(replacement) = mapped else {
            ctx.report(self.name(), prop_span, rejected_message(prop));
            return;
        };

        let mut edits = vec![Edit {
            span: prop_span,
            replacement: replacement.to_string(),
        }];
        if (normalized == "page-break-before" || normalized == "page-break-after")
            && let Some(value_span) = decl_value_span(ctx.source, decl)
        {
            for token in tokenize_value(ctx.source, value_span) {
                if token.kind == ValueTokenKind::Ident && token.raw.eq_ignore_ascii_case("always") {
                    edits.push(Edit {
                        span: token.span,
                        replacement: "page".to_string(),
                    });
                }
            }
        }

        ctx.report_with_fix(
            self.name(),
            prop_span,
            expected_message(prop, replacement),
            Fix { edits },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "a { clip-path: rect(0 0 0 0); }",
            "display: -webkit-box;\n-webkit-box-orient: vertical;\n-webkit-line-clamp: 2;",
        ];

        let fail = vec![
            reject(
                "a { clip: rect(0, 0, 0, 0); }",
                None,
                rejected_message("clip"),
                (1, 5, 1, 9),
            ),
            reject(
                "a { /* comment */ clip: rect(0, 0, 0, 0); }",
                None,
                rejected_message("clip"),
                (1, 19, 1, 23),
            ),
            reject(
                "a { ime-mode: active; }",
                None,
                rejected_message("ime-mode"),
                (1, 5, 1, 13),
            ),
            reject(
                "a { page-break-before: always; }",
                "a { break-before: page; }",
                expected_message("page-break-before", "break-before"),
                (1, 5, 1, 22),
            ),
            reject(
                "a { page-break-before: always /* foo */; }",
                "a { break-before: page /* foo */; }",
                expected_message("page-break-before", "break-before"),
                (1, 5, 1, 22),
            ),
            reject_warnings(
                "a {\n\t-moz-box-flex: revert;\n\t-moz-box-pack: start;\n}",
                "a {\n\tflex-grow: revert;\n\t-moz-box-pack: start;\n}",
                vec![
                    (
                        expected_message("-moz-box-flex", "flex-grow"),
                        (2, 2, 2, 15),
                    ),
                    (rejected_message("-moz-box-pack"), (3, 2, 3, 15)),
                ],
            ),
        ];

        Tester::new("property-no-deprecated", json!(true), &pass, &fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn ignore_properties() {
        let pass = vec![
            "a { grid-column-gap: 1px; }",
            "a { grid-gap: 1px; }",
            "a { grid-row-gap: 1px; }",
            "a { word-wrap: break-word; }",
        ];
        let fail = vec![reject(
            "a { -webkit-user-modify: read-only; }",
            None,
            rejected_message("-webkit-user-modify"),
            (1, 5, 1, 24),
        )];
        Tester::new(
            "property-no-deprecated",
            json!([true, {"ignoreProperties": ["/^grid-/", "word-wrap"]}]),
            &pass,
            &fail,
        )
        .with_fix(true)
        .test();
    }
}
