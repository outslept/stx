use oxc_css_parser::{Span, ast::Declaration};
use serde_json::{Map, Value};

use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        COLOR_FUNCTIONS, COLOR_PROPERTIES, LENGTH_UNITS, NAMED_COLORS_KEYWORDS, OptionMatcher,
        ValueTokenKind, decl_value_span, ident_name, in_set_ignore_case, is_custom_property,
        is_standard_syntax_property, is_standard_syntax_value, options_matches,
        parse_option_matchers, parse_primary_option_matchers, tokenize_value,
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

struct TypedIgnores {
    all: Vec<OptionMatcher>,
    by_type: Vec<(OptionMatcher, Vec<OptionMatcher>)>,
}

impl TypedIgnores {
    fn parse(secondary: Option<&Map<String, Value>>, option: &str) -> Self {
        Self {
            all: parse_option_matchers(secondary, option),
            by_type: secondary
                .and_then(|options| options.get(option))
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .map(|(key, values)| {
                    (
                        OptionMatcher::parse(key),
                        parse_primary_option_matchers(values),
                    )
                })
                .collect(),
        }
    }

    fn matches(&self, value_type: &str, name: &str) -> bool {
        options_matches(&self.all, name)
            || self
                .by_type
                .iter()
                .any(|(key, values)| key.is_match(value_type) && options_matches(values, name))
    }
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn function_end(source: &str, open_paren: usize, end: usize) -> usize {
    let bytes = source.as_bytes();
    let mut depth = 1usize;
    let mut i = open_paren + 1;
    while i < end {
        match bytes[i] {
            b'\\' => i = (i + 2).min(end),
            b'/' if i + 1 < end && bytes[i + 1] == b'*' => {
                i = bytes[i + 2..end]
                    .windows(2)
                    .position(|pair| pair == b"*/")
                    .map_or(end, |offset| i + offset + 4);
            }
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

crate::define_stx_lint! {
    /// Requires custom property references for configured types of literal values.
    ///
    /// Set to a type or list of types: `"angle"`, `"color"`, `"length"`, `"number"`, or
    /// `"percentage"`. Color detection covers named colors and recognized color
    /// functions, not hex colors. `ignoreValues` matches value text; `ignoreFunctions`
    /// and `ignoreProperties` accept names or `/regex/` patterns, or maps from type
    /// names or patterns to those matchers. Skips custom property declarations and
    /// `@property` blocks. Does not fix violations.
    ///
    /// Example: With `"length"`, `a { --foo: 1px; width: calc(var(--foo) + 2rem); }`
    /// reports `2rem`, but not the declaration of `--foo` or its `var()` reference.
    "value-type-custom-property-required-list", restriction;
    pub struct ValueTypeCustomPropertyRequiredList {
        types: Vec<String>,
        ignore_functions: TypedIgnores,
        ignore_properties: TypedIgnores,
        ignore_values: Vec<OptionMatcher>,
    }
}

impl ValueTypeCustomPropertyRequiredList {
    pub fn from_config(primary: &Value, secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            types: strings(primary)
                .into_iter()
                .map(|value| value.to_ascii_lowercase())
                .collect(),
            ignore_functions: TypedIgnores::parse(secondary, "ignoreFunctions"),
            ignore_properties: TypedIgnores::parse(secondary, "ignoreProperties"),
            ignore_values: parse_option_matchers(secondary, "ignoreValues"),
        }
    }
}

impl Rule for ValueTypeCustomPropertyRequiredList {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(property) = ident_name(&decl.name) else {
            return;
        };
        if is_custom_property(property)
            || !ctx.is_standard_declaration(decl)
            || !is_standard_syntax_property(property)
            || self.types.iter().all(|kind| self.ignore_properties.matches(kind, property))
            || ctx.ancestors.iter().any(
                |ancestor| matches!(ancestor, AstParent::AtRule(at_rule) if at_rule.name.name.eq_ignore_ascii_case("property")),
            )
        {
            return;
        }
        let Some(value_span) = decl_value_span(ctx.source, decl) else {
            return;
        };
        if !is_standard_syntax_value(ctx.slice(value_span)) {
            return;
        }
        let color_only = in_set_ignore_case(COLOR_PROPERTIES, property);
        let mut functions = Vec::new();
        let mut fragments = Vec::new();
        let mut variables = Vec::new();
        let mut skip_until = 0;
        for token in tokenize_value(ctx.source, value_span) {
            if token.span.start < skip_until {
                continue;
            }
            while functions
                .last()
                .is_some_and(|&(_, end)| token.span.start >= end)
            {
                functions.pop();
            }
            let mut span = token.span;
            let value_type = match token.kind {
                ValueTokenKind::Dimension { num_len } => {
                    let unit = &token.raw[num_len..];
                    if in_set_ignore_case(&["deg", "grad", "rad", "turn"], unit) {
                        "angle"
                    } else if !unit.eq_ignore_ascii_case("fr")
                        && in_set_ignore_case(LENGTH_UNITS, unit)
                    {
                        "length"
                    } else {
                        continue;
                    }
                }
                ValueTokenKind::Number => "number",
                ValueTokenKind::Percentage => "percentage",
                ValueTokenKind::Function => {
                    let function = token.raw.strip_suffix('(').unwrap();
                    span.end =
                        function_end(ctx.source, span.end as usize - 1, value_span.end as usize)
                            as u32;
                    if function.eq_ignore_ascii_case("var") {
                        variables.push(span);
                        skip_until = span.end;
                        continue;
                    }
                    if function.eq_ignore_ascii_case("url") {
                        skip_until = span.end;
                        continue;
                    }
                    functions.push((function, span.end));
                    if !in_set_ignore_case(COLOR_FUNCTIONS, function) {
                        continue;
                    }
                    "color"
                }
                ValueTokenKind::Ident
                    if in_set_ignore_case(NAMED_COLORS_KEYWORDS, token.raw)
                        || in_set_ignore_case(&["transparent", "currentcolor"], token.raw) =>
                {
                    "color"
                }
                _ => continue,
            };
            if !self.types.iter().any(|kind| kind == value_type)
                || (color_only && functions.is_empty() && value_type != "color")
            {
                continue;
            }
            let ignored = self.ignore_properties.matches(value_type, property)
                || options_matches(&self.ignore_values, ctx.slice(span))
                || functions
                    .iter()
                    .any(|&(name, _)| self.ignore_functions.matches(value_type, name));
            fragments.push((value_type, span, ignored));
        }
        let ignored: Vec<_> = fragments.iter().copied().filter(|entry| entry.2).collect();
        for (value_type, span, _) in fragments {
            if variables.iter().any(|&var| contains(span, var))
                || ignored.iter().any(|&(kind, other, _)| {
                    contains(other, span) || (kind == value_type && contains(span, other))
                })
            {
                continue;
            }
            let display = ctx.slice(span);
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
    use crate::tester::{Tester, reject, reject_message, reject_warnings};
    use serde_json::json;

    fn expected(value: &str, kind: &str) -> String {
        format!("Expected \"{value}\" to be a custom property, as of type \"<{kind}>\"")
    }

    #[test]
    fn ignores_types_by_property_and_enclosing_function() {
        Tester::new(
            "value-type-custom-property-required-list",
            json!([["color", "length", "number", "angle", "percentage"], {
                "ignoreFunctions": {
                    "number": ["/.+/"], "color": ["color-mix"], "length": ["calc"],
                    "/^(angle|percentage)$/": ["/-gradient$/"]
                },
                "ignoreProperties": {"/.+/": ["transform"], "length": ["/^border/"]}
            }]),
            [
                "a { transform: translateX(1px); }",
                "a { border-width: 1px; }",
                "a { color: color-mix(in srgb, red, blue); }",
                "a { --foo: 0; color: rgb(var(--foo) 0 0); }",
                "a { color: 1px; }",
                "a { width: calc(1px /* ) */ + 2px); }",
            ],
            [
                reject(
                    "a { border: 1px solid red; }",
                    None,
                    expected("red", "color"),
                    (1, 23, 1, 26),
                ),
                reject(
                    "a { color: rgb(0 0 0); }",
                    None,
                    expected("rgb(0 0 0)", "color"),
                    (1, 12, 1, 22),
                ),
                reject_warnings(
                    "a { background: linear-gradient(45deg, red 0%, blue 100%); }",
                    None,
                    vec![
                        (expected("red", "color"), (1, 40, 1, 43)),
                        (expected("blue", "color"), (1, 48, 1, 52)),
                    ],
                ),
            ],
        )
        .test();
    }

    #[test]
    fn ignored_and_variable_fragments_do_not_hide_unrelated_values() {
        Tester::new(
            "value-type-custom-property-required-list",
            json!([["color", "number", "length", "angle"], {
                "ignoreValues": ["0", "transparent", "/^oklch\\(/"]
            }]),
            ["a { color: oklch(20% 0.1 0deg); }"],
            [
                reject(
                    "a { color: color-mix(in srgb, transparent, red); }",
                    None,
                    expected("red", "color"),
                    (1, 44, 1, 47),
                ),
                reject(
                    "a { --foo: 1px; width: calc(var(--foo) + 2rem); }",
                    None,
                    expected("2rem", "length"),
                    (1, 42, 1, 46),
                ),
                reject(
                    "a { transform: rotate(45deg); }",
                    None,
                    expected("45deg", "angle"),
                    (1, 23, 1, 28),
                ),
            ],
        )
        .test();
    }

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
