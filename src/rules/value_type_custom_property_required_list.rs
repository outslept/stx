use oxc_css_parser::{Span, ast::Declaration};
use serde_json::{Map, Value};

use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        COLOR_FUNCTIONS, OptionMatcher, ValueTokenKind, decl_value_span, ident_name,
        is_custom_property, is_standard_syntax_property, options_matches, parse_option_matchers,
        tokenize_value,
    },
};

fn strings(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) => vec![value.clone()],
        Value::Array(values) => values
            .iter()
            .filter_map(|value| value.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

fn is_length_unit(unit: &str) -> bool {
    matches!(
        unit,
        "px" | "em"
            | "rem"
            | "ex"
            | "ch"
            | "vh"
            | "vw"
            | "vmin"
            | "vmax"
            | "vi"
            | "vb"
            | "cm"
            | "mm"
            | "in"
            | "pt"
            | "pc"
            | "q"
    )
}

fn is_color_keyword(value: &str) -> bool {
    matches!(
        value,
        "red"
            | "green"
            | "blue"
            | "black"
            | "white"
            | "pink"
            | "gray"
            | "grey"
            | "yellow"
            | "orange"
            | "purple"
            | "transparent"
            | "currentcolor"
    )
}

fn function_end(source: &str, open_paren: usize, end: usize) -> usize {
    let bytes = source.as_bytes();
    let mut depth = 1usize;
    let mut i = open_paren + 1;
    while i < end {
        match bytes[i] {
            b'\\' => i = (i + 2).min(end),
            b'\'' | b'"' => {
                let quote = bytes[i];
                i += 1;
                while i < end {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(end);
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
            }
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => i += 1,
        }
    }
    end
}

pub struct ValueTypeCustomPropertyRequiredList {
    types: Vec<String>,
    ignore_functions: Vec<OptionMatcher>,
    ignore_properties: Vec<OptionMatcher>,
    ignore_values: Vec<OptionMatcher>,
}

impl ValueTypeCustomPropertyRequiredList {
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            types: strings(primary)
                .into_iter()
                .map(|value| value.to_ascii_lowercase())
                .collect(),
            ignore_functions: parse_option_matchers(secondary, "ignoreFunctions"),
            ignore_properties: parse_option_matchers(secondary, "ignoreProperties"),
            ignore_values: parse_option_matchers(secondary, "ignoreValues"),
        }
    }
}

impl Rule for ValueTypeCustomPropertyRequiredList {
    fn name(&self) -> &'static str {
        "value-type-custom-property-required-list"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(property) = ident_name(&decl.name) else {
            return;
        };
        if is_custom_property(property)
            || !ctx.is_standard_declaration(decl)
            || !is_standard_syntax_property(property)
            || options_matches(&self.ignore_properties, property)
            || ctx.ancestors.iter().any(
                |ancestor| matches!(ancestor, AstParent::AtRule(at_rule) if at_rule.name.name.eq_ignore_ascii_case("property")),
            )
        {
            return;
        }
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let mut skip_until = 0usize;
        for token in tokenize_value(ctx.source, value_span) {
            if (token.span.start as usize) < skip_until {
                continue;
            }
            let (value_type, span) = match token.kind {
                ValueTokenKind::Dimension { num_len }
                    if self.types.iter().any(|value| value == "length")
                        && is_length_unit(&token.raw[num_len..].to_ascii_lowercase()) =>
                {
                    ("length", token.span)
                }
                ValueTokenKind::Number if self.types.iter().any(|value| value == "number") => {
                    ("number", token.span)
                }
                ValueTokenKind::Percentage
                    if self.types.iter().any(|value| value == "percentage") =>
                {
                    ("percentage", token.span)
                }
                ValueTokenKind::Function => {
                    let function = token.raw.strip_suffix('(').unwrap_or(token.raw);
                    let end = function_end(
                        ctx.source,
                        token.span.end as usize - 1,
                        value_span.end as usize,
                    );
                    if options_matches(&self.ignore_functions, function) {
                        skip_until = end;
                        continue;
                    }
                    if self.types.iter().any(|value| value == "color")
                        && COLOR_FUNCTIONS
                            .iter()
                            .any(|candidate| function.eq_ignore_ascii_case(candidate))
                    {
                        (
                            "color",
                            Span {
                                start: token.span.start,
                                end: end as u32,
                            },
                        )
                    } else {
                        continue;
                    }
                }
                ValueTokenKind::Ident
                    if self.types.iter().any(|value| value == "color")
                        && is_color_keyword(&token.raw.to_ascii_lowercase()) =>
                {
                    ("color", token.span)
                }
                _ => continue,
            };
            let display = ctx.slice(span);
            if options_matches(&self.ignore_values, display) {
                continue;
            }
            ctx.report(
                self.name(),
                span,
                format!(
                    "Expected \"{display}\" to be a custom property, as of type \"<{value_type}>\""
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_message};
    use serde_json::json;

    #[test]
    fn reports_configured_types() {
        let pass = ["a { color: var(--x); }", "a { --x: red; }"];
        let fail = [reject_message(
            "a { color: red; }",
            None,
            "Expected \"red\" to be a custom property, as of type \"<color>\"",
        )];
        Tester::new(
            "value-type-custom-property-required-list",
            json!(["color"]),
            pass,
            &fail,
        )
        .test();
        let fail = [reject_message(
            "a { color: rgb(1 2 3); }",
            None,
            "Expected \"rgb(1 2 3)\" to be a custom property, as of type \"<color>\"",
        )];
        Tester::new(
            "value-type-custom-property-required-list",
            json!(["color"]),
            [],
            &fail,
        )
        .test();
        let fail = [reject_message(
            "a { margin: 1px; }",
            None,
            "Expected \"1px\" to be a custom property, as of type \"<length>\"",
        )];
        Tester::new(
            "value-type-custom-property-required-list",
            json!(["length"]),
            ["a { margin: var(--m); }"],
            &fail,
        )
        .test();
    }

    #[test]
    fn supports_ignores_and_registered_property_values() {
        Tester::new(
            "value-type-custom-property-required-list",
            json!(["color", {
                "ignoreFunctions": ["rgb"],
                "ignoreProperties": ["background"],
                "ignoreValues": ["red", "/^blue$/"]
            }]),
            [
                "a { color: rgb(1 2 3); color: red; color: blue; background: green; }",
                "@property --x { syntax: \"<length>\"; initial-value: 1px; }",
            ],
            &[],
        )
        .test();
    }
}
