use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, declaration_value_span, options_matches,
        parse_option_matchers, tokenize_value,
    },
};
use oxc_css_parser::ast::{Declaration, InterpolableIdent};
use serde_json::{Map, Value};

pub struct NumberMaxPrecision {
    pub max: usize,
    ignore_properties: Vec<OptionMatcher>,
    ignore_units: Vec<OptionMatcher>,
    inside_functions: Vec<(OptionMatcher, usize)>,
}

impl Default for NumberMaxPrecision {
    fn default() -> Self {
        Self {
            max: 4,
            ignore_properties: Vec::new(),
            ignore_units: Vec::new(),
            inside_functions: Vec::new(),
        }
    }
}

impl NumberMaxPrecision {
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Self {
        let inside_functions = secondary
            .and_then(|map| map.get("insideFunctions"))
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(|map| map.iter())
            .filter_map(|(name, precision)| {
                Some((OptionMatcher::parse(name), precision.as_u64()? as usize))
            })
            .collect();
        Self {
            max: primary.as_u64().unwrap_or(4) as usize,
            ignore_properties: parse_option_matchers(secondary, "ignoreProperties"),
            ignore_units: parse_option_matchers(secondary, "ignoreUnits"),
            inside_functions,
        }
    }

    fn check_number_str(&self, num_str: &str, max: usize) -> Option<String> {
        let mantissa = match num_str.find(['e', 'E']) {
            Some(idx) => &num_str[..idx],
            None => num_str,
        };
        let dot_idx = mantissa.find('.')?;
        let decimals = &mantissa[dot_idx + 1..];
        if decimals.len() <= max {
            return None;
        }
        let val = num_str.parse::<f64>().ok()?;
        let factor = 10f64.powi(max as i32);
        let rounded = (val * factor).round() / factor;
        let formatted = format!("{:.prec$}", rounded, prec = max);
        let cleaned = if max > 0 {
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            formatted
        };
        Some(cleaned)
    }
}

impl Rule for NumberMaxPrecision {
    fn name(&self) -> &'static str {
        "number-max-precision"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };

        if options_matches(&self.ignore_properties, ident.name) {
            return;
        }
        let tokens = tokenize_value(ctx.source, val_span);
        let mut functions: Vec<(bool, usize)> = Vec::new();
        for tok in tokens {
            match tok.kind {
                ValueTokenKind::Function => {
                    let name = tok.raw[..tok.raw.len() - 1].trim_start_matches(['$', '@']);
                    let inherited = functions.last().copied().unwrap_or((false, self.max));
                    let precision = self
                        .inside_functions
                        .iter()
                        .find(|(matcher, _)| matcher.is_match(name))
                        .map_or(inherited.1, |(_, precision)| *precision);
                    functions.push((inherited.0 || name.eq_ignore_ascii_case("url"), precision));
                    continue;
                }
                ValueTokenKind::CloseParen => {
                    functions.pop();
                    continue;
                }
                _ => {}
            }
            let (ignored, precision) = functions.last().copied().unwrap_or((false, self.max));
            if ignored {
                continue;
            }
            let (num_str, num_len) = match tok.kind {
                ValueTokenKind::Number => (tok.raw, tok.raw.len()),
                ValueTokenKind::Percentage => {
                    if options_matches(&self.ignore_units, "%") {
                        continue;
                    }
                    (&tok.raw[..tok.raw.len() - 1], tok.raw.len() - 1)
                }
                ValueTokenKind::Dimension { num_len } => {
                    if options_matches(&self.ignore_units, &tok.raw[num_len..]) {
                        continue;
                    }
                    (&tok.raw[..num_len], num_len)
                }
                _ => continue,
            };

            if let Some(expected) = self.check_number_str(num_str, precision) {
                let raw_clean = num_str.trim_start_matches('+');
                ctx.report(
                    self.name(),
                    oxc_css_parser::Span {
                        start: tok.span.start,
                        end: tok.span.start + num_len as u32,
                    },
                    format!("Expected \"{}\" to be \"{}\"", raw_clean, expected),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn reports_number_spans_without_units() {
        let fail = vec![
            reject(
                "a { width: 1.23456px; }",
                None,
                "Expected \"1.23456\" to be \"1.2346\"",
                (1, 12, 1, 19),
            ),
            reject(
                "a { opacity: 0.12345%; }",
                None,
                "Expected \"0.12345\" to be \"0.1235\"",
                (1, 14, 1, 21),
            ),
        ];
        Tester::new("number-max-precision", json!(4), [], &fail).test();
    }

    #[test]
    fn preserves_ignore_units() {
        Tester::new(
            "number-max-precision",
            json!([4, { "ignoreUnits": ["px"] }]),
            ["a { width: 1.23456px; }"],
            &[],
        )
        .test();
    }
}
