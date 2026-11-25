use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        FLOW_RELATIVE_UNITS, OptionMatcher, ValueTokenKind, decl_value_span, ident_name,
        is_standard_syntax_declaration, options_matches, parse_option_matchers,
        physical_to_flow_relative_unit, tokenize_value,
    },
};
use oxc_css_parser::Span;

#[derive(Debug)]
pub enum LayoutMode {
    Physical,
    FlowRelative,
}

pub struct UnitLayoutMappings {
    mode: LayoutMode,
    ignore_units: Vec<OptionMatcher>,
}
impl UnitLayoutMappings {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        let mode = match primary.as_str()? {
            "physical" => LayoutMode::Physical,
            "flow-relative" => LayoutMode::FlowRelative,
            _ => return None,
        };
        Some(Self {
            mode,
            ignore_units: parse_option_matchers(secondary, "ignoreUnits"),
        })
    }
}
impl Rule for UnitLayoutMappings {
    fn name(&self) -> &'static str {
        "unit-layout-mappings"
    }
    fn check_declaration<'a>(
        &self,
        decl: &'a oxc_css_parser::ast::Declaration<'a>,
        ctx: &mut LintContext<'a>,
    ) {
        if !is_standard_syntax_declaration(decl) {
            return;
        }
        let _ = ident_name(&decl.name);
        let Some(vspan) = decl_value_span(ctx.source, decl) else {
            return;
        };
        for token in tokenize_value(ctx.source, vspan) {
            let ValueTokenKind::Dimension { num_len } = token.kind else {
                continue;
            };
            let unit = &token.raw[num_len..];
            if options_matches(&self.ignore_units, unit) {
                continue;
            }
            let lower = unit.to_ascii_lowercase();
            let disallowed = match self.mode {
                LayoutMode::Physical => {
                    if FLOW_RELATIVE_UNITS.contains(&lower.as_str()) {
                        Some("flow-relative")
                    } else {
                        None
                    }
                }
                LayoutMode::FlowRelative => {
                    if physical_to_flow_relative_unit(&lower).is_some() {
                        Some("physical")
                    } else {
                        None
                    }
                }
            };
            if let Some(kind) = disallowed {
                ctx.report(
                    self.name(),
                    Span {
                        start: token.span.start + num_len as u32,
                        end: token.span.end,
                    },
                    format!("Disallowed {kind} unit \"{unit}\""),
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;
    #[test]
    fn physical() {
        let pass = vec!["a { margin-top: 0vh; }", "a { width: 0vmin; }"];
        let fail = vec![
            reject(
                "a { margin-top: 0vb; }",
                None,
                "Disallowed flow-relative unit \"vb\"",
                (1, 18, 1, 20),
            ),
            reject(
                "a { margin-top: 0vi; }",
                None,
                "Disallowed flow-relative unit \"vi\"",
                (1, 18, 1, 20),
            ),
            reject(
                "a { margin-top: 0VB; }",
                None,
                "Disallowed flow-relative unit \"VB\"",
                (1, 18, 1, 20),
            ),
        ];
        Tester::new("unit-layout-mappings", json!(["physical"]), &pass, &fail).test();
    }
    #[test]
    fn flow_relative() {
        let pass = vec![
            "a { margin-inline-start: 0vb; }",
            "a { width: 0vmin; }",
            "a { margin-inline-start: 0px; }",
            "a { width: 0cqmin; }",
        ];
        let fail = vec![
            reject(
                "a { margin-inline-start: 0vh; }",
                None,
                "Disallowed physical unit \"vh\"",
                (1, 27, 1, 29),
            ),
            reject(
                "a { margin-inline-start: calc(100vw - 10px); }",
                None,
                "Disallowed physical unit \"vw\"",
                (1, 34, 1, 36),
            ),
            reject(
                "a { margin-inline-start: 0VH; }",
                None,
                "Disallowed physical unit \"VH\"",
                (1, 27, 1, 29),
            ),
            reject_warnings(
                "a { padding-inline: 0vw 0vh; }",
                None,
                vec![
                    (
                        "Disallowed physical unit \"vw\"".to_string(),
                        (1, 22, 1, 24),
                    ),
                    (
                        "Disallowed physical unit \"vh\"".to_string(),
                        (1, 26, 1, 28),
                    ),
                ],
            ),
        ];
        Tester::new(
            "unit-layout-mappings",
            json!(["flow-relative"]),
            &pass,
            &fail,
        )
        .test();
    }
}
