use oxc_css_parser::ast::Declaration;

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        LONGHAND_TIME_PROPERTIES, SHORTHAND_TIME_PROPERTIES, ValueTokenKind, decl_value_span,
        ident_name, secondary_has_flag, tokenize_value, unprefixed,
    },
};

const DELAY_PROPERTIES: &[&str] = &["animation-delay", "transition-delay"];

crate::define_stx_lint! {
    /// Reports positive animation and transition times below a minimum.
    ///
    /// Set to a number of milliseconds. Checks `ms` and `s` dimensions and exempts
    /// zero and negative values. Use `ignore: ["delay"]` to skip delay longhands and
    /// check only the first dimension of each comma-separated shorthand item.
    /// Does not fix violations.
    ///
    /// Example: `a { animation-duration: 80ms; }` is reported with a minimum of `100`.
    "time-min-milliseconds", restriction;
    pub struct TimeMinMilliseconds {
        minimum: f64,
        ignore_delay: bool,
    }
}

impl TimeMinMilliseconds {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        Some(Self {
            minimum: primary.as_f64()?,
            ignore_delay: secondary_has_flag(secondary, "ignore", "delay"),
        })
    }
}

fn expected_message(minimum: f64) -> String {
    format!("Too short duration, minimum {minimum} milliseconds")
}

fn is_acceptable(unit: &str, number: f64, minimum: f64) -> bool {
    if unit.is_empty() || number <= 0.0 {
        return true;
    }
    if unit.eq_ignore_ascii_case("ms") && number < minimum {
        return false;
    }
    if unit.eq_ignore_ascii_case("s") && number * 1000.0 < minimum {
        return false;
    }
    true
}

impl Rule for TimeMinMilliseconds {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        let property = unprefixed(&prop.to_ascii_lowercase()).to_ascii_lowercase();
        let is_longhand = LONGHAND_TIME_PROPERTIES.contains(&property.as_str());
        let is_shorthand = SHORTHAND_TIME_PROPERTIES.contains(&property.as_str());
        if !is_longhand && !is_shorthand {
            return;
        }
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let delay_ignored = self.ignore_delay && DELAY_PROPERTIES.contains(&property.as_str());

        let mut count = 0usize;
        for token in tokenize_value(ctx.source, value_span) {
            if token.kind == ValueTokenKind::Comma {
                count = 0;
                continue;
            }
            let ValueTokenKind::Dimension { num_len } = token.kind else {
                continue;
            };
            let unit = &token.raw[num_len..];
            let number = token.raw[..num_len].parse::<f64>().unwrap_or(0.0);
            let acceptable = is_acceptable(unit, number, self.minimum);

            if is_longhand && !delay_ignored && !acceptable {
                ctx.report(self.name(), value_span, expected_message(self.minimum));
            }

            if !is_shorthand {
                continue;
            }
            count += 1;
            if acceptable || (self.ignore_delay && count != 1) {
                continue;
            }
            ctx.report(self.name(), token.span, expected_message(self.minimum));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    const MIN_VALUE: u64 = 100;

    #[test]
    fn default_config() {
        let pass = vec![
            "a { animation: none; }",
            "a { animation: none, my-animation; }",
            "a { transition: opacity var(--x) var(--y) ease-out }",
            "a { transition-delay: 0.1s; }",
            "a { transition-delay: 0.2s; }",
            "a { -webkit-transition-delay: 0.2s; }",
            "a { tRaNsItIoN-dElAy: 0.2s; }",
            "a { TRANSITION-DELAY: 0.2s; }",
            "a { transition-delay: 3s; }",
            "a { transition-delay: 100ms; }",
            "a { transition-duration: 2.8s; }",
            "a { transition: foo 0.8s linear; }",
            "a { transition: foo 0.8s ease-in-out 200ms; }",
            "a { transition: foo 0.8s, bar 0.8s; }",
            "a { animation: foo 0.8s ease-in-out -20ms; }",
            "a { animation-delay: 0ms; }",
            "a { animation-delay: 0; }",
        ];

        let fail = vec![
            reject(
                "a { transition-delay: 0.006s; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 23, 1, 29),
            ),
            reject(
                "a { -webkit-transition-delay: 0.006s; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 31, 1, 37),
            ),
            reject(
                "a { transition-delay: 3ms; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 23, 1, 26),
            ),
            reject(
                "a { transition-duration: 0.009s; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 26, 1, 32),
            ),
            reject(
                "a { transition: foo 0.008s linear; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 21, 1, 27),
            ),
            reject(
                "a { transition: foo 0.8s ease-in-out 20ms; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 38, 1, 42),
            ),
            reject(
                "a { transition: foo 0.8s ease 200ms, bar 0.8s ease 20ms; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 52, 1, 56),
            ),
            reject(
                "a { animation: foo 0.8s ease, bar 20ms ease; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 35, 1, 39),
            ),
            reject(
                "a { animation-delay: 0.006s; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 22, 1, 28),
            ),
            reject(
                "a { animation-duration: 80ms; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 25, 1, 29),
            ),
            reject(
                "a { animation: foo 0.008s linear; }",
                None,
                expected_message(MIN_VALUE as f64),
                (1, 20, 1, 26),
            ),
        ];

        Tester::new("time-min-milliseconds", json!([MIN_VALUE]), &pass, &fail).test();
    }

    #[test]
    fn ignore_delay() {
        let pass = vec![
            "a { transition: foo 0.8s ease-in-out 20ms; }",
            "a { animation: foo 0.8s ease-in-out 20ms; }",
            "a { transition-delay: 20ms; }",
            "a { animation-delay: 20ms; }",
        ];
        let fail = vec![reject(
            "a { transition: foo 20ms ease-in-out 0.8s; }",
            None,
            expected_message(MIN_VALUE as f64),
            (1, 21, 1, 25),
        )];
        Tester::new(
            "time-min-milliseconds",
            json!([MIN_VALUE, {"ignore": ["delay"]}]),
            &pass,
            &fail,
        )
        .test();
    }
}
