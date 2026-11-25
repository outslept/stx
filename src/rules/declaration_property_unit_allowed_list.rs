use oxc_css_parser::ast::Declaration;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, decl_value_span, ident_name, secondary_has_flag,
        tokenize_value,
    },
};

crate::define_stx_lint! {
    /// Restricts which units can appear in declarations for configured properties.
    ///
    /// The primary option maps property names or `/regex/` patterns to a unit
    /// or an array of units. The first matching entry applies. Units are
    /// case-insensitive; use `"%"` for percentages. Unitless numbers and
    /// `url()` arguments are not checked. Set `ignore` to `"inside-function"`
    /// (or include it in an array) to skip units in other functions too.
    ///
    /// With `{"font-size": "px"}`:
    ///
    /// ```css
    /// a { font-size: 12px; } /* passes */
    /// a { font-size: 1.2rem; } /* reported */
    /// ```
    "declaration-property-unit-allowed-list", restriction;
    #[derive(Debug, Default)]
    pub struct DeclarationPropertyUnitAllowedList {
        entries: Vec<(OptionMatcher, Vec<String>)>,
        ignore_inside_function: bool,
    }
}

impl DeclarationPropertyUnitAllowedList {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        let object = primary.as_object()?;
        let mut entries = Vec::with_capacity(object.len());
        for (key, value) in object {
            let mut units = Vec::new();
            match value {
                serde_json::Value::String(s) => units.push(s.to_ascii_lowercase()),
                serde_json::Value::Array(items) => {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            units.push(s.to_ascii_lowercase());
                        }
                    }
                }
                _ => {}
            }
            entries.push((OptionMatcher::parse(key), units));
        }
        Some(Self {
            entries,
            ignore_inside_function: secondary_has_flag(secondary, "ignore", "inside-function"),
        })
    }
}

pub fn rejected_message(property: &str, unit: &str) -> String {
    format!("Disallowed unit \"{unit}\" for property \"{property}\"")
}

impl Rule for DeclarationPropertyUnitAllowedList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };

        let mut allowed = None;
        for (matcher, units) in &self.entries {
            if matcher.is_match(prop) {
                allowed = Some(units);
                break;
            }
        }
        let Some(allowed) = allowed else {
            return;
        };

        let tokens = tokenize_value(ctx.source, value_span);
        let mut depth = 0usize;
        let mut skip_from: Option<usize> = None;

        for token in &tokens {
            match token.kind {
                ValueTokenKind::Function => {
                    depth += 1;
                    if skip_from.is_none() {
                        let name = token.raw.strip_suffix('(').unwrap_or(token.raw);
                        if name.eq_ignore_ascii_case("url") || self.ignore_inside_function {
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
                ValueTokenKind::Dimension { num_len } => {
                    let unit = &token.raw[num_len..];
                    if !allowed.iter().any(|u| u.eq_ignore_ascii_case(unit)) {
                        ctx.report(self.name(), token.span, rejected_message(prop, unit));
                    }
                }
                ValueTokenKind::Percentage if !allowed.iter().any(|u| u == "%") => {
                    ctx.report(self.name(), token.span, rejected_message(prop, "%"));
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;

    const RULE_NAME: &str = "declaration-property-unit-allowed-list";

    fn rejected(property: &str, unit: &str) -> String {
        format!("Disallowed unit \"{unit}\" for property \"{property}\"")
    }

    #[test]
    fn default_config() {
        let pass = [
            "a { color: pink; }",
            "a { top: 0; }",
            "a { color: #000; }",
            "a { margin: 0 0 0 0; }",
            "a { margin: 0 10em; }",
            "a { margin: 0 10eM; }",
            "a { margin: 0 10EM; }",
            "a { background-position: top right, 0 50%; }",
            "a { margin: calc(30em - 10em); }",
            "a { animation: animation-name 1s ease; }",
            "a { -webkit-animation: animation-name 1s ease; }",
            "a { line-height: 1; }",
            "a { font-size: /* 1.2rem */ 12px; }",
            "a::before { font-size: \"1.2rem\"}",
            "a { font-size: $fs1rem; }",
            "a { font-size: --some-fs-1rem; }",
        ];
        let fail = [
            reject(
                "a { font-size: 1.2rem; }",
                None,
                rejected("font-size", "rem"),
                (1, 16, 1, 22),
            ),
            reject(
                "a { font-size: 1.2rEm; }",
                None,
                rejected("font-size", "rEm"),
                (1, 16, 1, 22),
            ),
            reject(
                "a { font-size: 1.2REM; }",
                None,
                rejected("font-size", "REM"),
                (1, 16, 1, 22),
            ),
            reject(
                "a { margin: 10em 0 1rem; }",
                None,
                rejected("margin", "rem"),
                (1, 20, 1, 24),
            ),
            reject(
                "a { background-position: 0 10px; }",
                None,
                rejected("background-position", "px"),
                (1, 28, 1, 32),
            ),
            reject(
                "a { background-position: top right, 0 10px; }",
                None,
                rejected("background-position", "px"),
                (1, 39, 1, 43),
            ),
            reject(
                "a { margin: calc(10em - 10px); }",
                None,
                rejected("margin", "px"),
                (1, 25, 1, 29),
            ),
            reject(
                "a { animation: animation-name 300ms ease; }",
                None,
                rejected("animation", "ms"),
                (1, 31, 1, 36),
            ),
            reject(
                "a { -webkit-animation: animation-name 300ms ease; }",
                None,
                rejected("-webkit-animation", "ms"),
                (1, 39, 1, 44),
            ),
            reject(
                "a { line-height: 1.2em; }",
                None,
                rejected("line-height", "em"),
                (1, 18, 1, 23),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([{
                "font-size": ["px", "em"],
                "margin": "em",
                "background-position": ["%"],
                "/^(-webkit-)?animation$/": ["s"],
                "line-height": [],
            }]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn animation_units() {
        let pass = [
            "a { animation: animation-name 300ms ease; }",
            "a { -webkit-animation: animation-name 300ms ease; }",
            "a { animation-duration: 300ms; }",
            "a { -webkit-animation-duration: 300ms; }",
            "a { animation: 3ms cubic-bezier(.17,.67,.83,.67) 1ms infinite alternate none running slidein; }",
        ];
        let fail = [
            reject_message(
                "a { animation: animation-name 3s ease; }",
                None,
                rejected("animation", "s"),
            ),
            reject_message(
                "a { -webkit-animation: animation-name 3s ease; }",
                None,
                rejected("-webkit-animation", "s"),
            ),
            reject_message(
                "a { animation-duration: 3s; }",
                None,
                rejected("animation-duration", "s"),
            ),
            reject_message(
                "a { -webkit-animation-duration: 3s; }",
                None,
                rejected("-webkit-animation-duration", "s"),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([{
                "/^(-webkit-)?animation/": ["ms"],
            }]),
            pass,
            fail,
        )
        .test();
    }

    #[test]
    fn ignore_inside_function() {
        let pass = [
            "a { border-color: hsla(162deg, 51%, 35%, 0.8); }",
            "a { border: 1px solid hsla(162deg, 51%, 35%, 0.8); }",
            "a { background-image: linear-gradient(hsla(162deg, 51%, 35%, 0.8), hsla(62deg, 51%, 35%, 0.8)); }",
            "a { background-image: url(\"https://example.com/img.png\"); }",
            "a { background: center center / 50% 50% linear-gradient(hsla(162deg, 51%, 35%, 0.8), hsla(62deg, 51%, 35%, 0.8)); }",
            "a { background-color: var(--bg-color); }",
        ];
        let fail = [
            reject_message(
                "a { border: 1rem solid hsla(162deg, 51%, 35%, 0.8); }",
                None,
                rejected("border", "rem"),
            ),
            reject_message(
                "a { background-position: 5rem 45%; }",
                None,
                rejected("background-position", "rem"),
            ),
            reject_message(
                "a { background: center center / 50px 10% linear-gradient(hsla(162deg, 51%, 35%, 0.8), hsla(62deg, 51%, 35%, 0.8)); }",
                None,
                rejected("background", "px"),
            ),
        ];

        Tester::new(
            RULE_NAME,
            json!([
                {
                    "/^border/": ["px"],
                    "/^background/": ["%"],
                },
                {
                    "ignore": ["inside-function"],
                },
            ]),
            pass,
            fail,
        )
        .test();
    }
}
