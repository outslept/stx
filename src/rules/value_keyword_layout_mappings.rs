use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, decl_value_span, flow_relative_value_keywords_for_property,
        ident_name, is_standard_syntax_value, options_matches, parse_option_matchers,
        physical_to_flow_relative_value_keyword, tokenize_value,
    },
};
use oxc_css_parser::Span;

pub struct ValueKeywordLayoutMappings {
    physical: bool,
    ignore_properties: Vec<OptionMatcher>,
    ignore_keywords: Vec<OptionMatcher>,
}
impl ValueKeywordLayoutMappings {
    pub fn from_config(
        primary: &serde_json::Value,
        secondary: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        let physical = match primary.as_str()? {
            "physical" => true,
            "flow-relative" => false,
            _ => return None,
        };
        Some(Self {
            physical,
            ignore_properties: parse_option_matchers(secondary, "ignoreProperties"),
            ignore_keywords: parse_option_matchers(secondary, "ignoreKeywords"),
        })
    }
}
impl Rule for ValueKeywordLayoutMappings {
    fn name(&self) -> &'static str {
        "value-keyword-layout-mappings"
    }
    fn check_declaration<'a>(
        &self,
        decl: &'a oxc_css_parser::ast::Declaration<'a>,
        ctx: &mut LintContext<'a>,
    ) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if !ctx.is_standard_declaration(decl) {
            return;
        }
        if options_matches(&self.ignore_properties, prop) {
            return;
        }
        let prop_lower = prop.to_ascii_lowercase();
        let flow_list = flow_relative_value_keywords_for_property(&prop_lower);
        if flow_list.is_none() && !self.physical {
            return;
        }
        if flow_list.is_none() && self.physical {
            return;
        }
        let Some(vspan) = decl_value_span(ctx.source, decl) else {
            return;
        };
        let trans = if self.physical {
            Some("flow-relative")
        } else {
            Some("physical")
        };
        for token in tokenize_value(ctx.source, vspan) {
            if token.kind != ValueTokenKind::Ident {
                continue;
            }
            let keyword = token.raw;
            if !is_standard_syntax_value(keyword) {
                continue;
            }
            let lower = keyword.to_ascii_lowercase();
            let is_disallowed = if self.physical {
                flow_list.is_some_and(|list| list.iter().any(|k| *k == lower))
            } else {
                physical_to_flow_relative_value_keyword(&prop_lower, &lower).is_some()
            };
            if !is_disallowed {
                continue;
            }
            if options_matches(&self.ignore_keywords, keyword) {
                continue;
            }
            let kind = trans.unwrap_or("physical");
            ctx.report(
                self.name(),
                token.span,
                format!("Disallowed {kind} keyword \"{keyword}\""),
            );
        }
        let _ = Span { start: 0, end: 0 };
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;
    #[test]
    fn physical() {
        let pass = vec![
            "a { float: left; }",
            "a { text-align: right; }",
            "a { resize: horizontal; }",
            "a { caption-side: top; }",
            "a { caption-side: block-start; }",
        ];
        let fail = vec![
            reject(
                "a { float: inline-start; }",
                None,
                "Disallowed flow-relative keyword \"inline-start\"",
                (1, 12, 1, 24),
            ),
            reject(
                "a { clear: inline-end; }",
                None,
                "Disallowed flow-relative keyword \"inline-end\"",
                (1, 12, 1, 22),
            ),
            reject(
                "a { resize: inline; }",
                None,
                "Disallowed flow-relative keyword \"inline\"",
                (1, 13, 1, 19),
            ),
            reject(
                "a { text-align: start; }",
                None,
                "Disallowed flow-relative keyword \"start\"",
                (1, 17, 1, 22),
            ),
            reject(
                "a { text-align-last: end; }",
                None,
                "Disallowed flow-relative keyword \"end\"",
                (1, 22, 1, 25),
            ),
            reject_warnings(
                "a { offset-anchor: block-start inline-end; }",
                None,
                vec![
                    (
                        "Disallowed flow-relative keyword \"block-start\"".to_string(),
                        (1, 20, 1, 31),
                    ),
                    (
                        "Disallowed flow-relative keyword \"inline-end\"".to_string(),
                        (1, 32, 1, 42),
                    ),
                ],
            ),
        ];
        Tester::new(
            "value-keyword-layout-mappings",
            json!(["physical"]),
            &pass,
            &fail,
        )
        .test();
    }
    #[test]
    fn flow_relative() {
        let pass = vec![
            "a { float: inline-start; }",
            "a { resize: inline; }",
            "a { text-align: start; }",
            "a { float: var(--left); }",
            "a { caption-side: top; }",
            "a { caption-side: block-end; }",
            "a { text-align: -webkit-left; }",
            "a { margin-left: 0; }",
        ];
        let fail = vec![
            reject(
                "a { float: left; }",
                None,
                "Disallowed physical keyword \"left\"",
                (1, 12, 1, 16),
            ),
            reject(
                "a { text-align: right; }",
                None,
                "Disallowed physical keyword \"right\"",
                (1, 17, 1, 22),
            ),
        ];
        Tester::new(
            "value-keyword-layout-mappings",
            json!(["flow-relative"]),
            &pass,
            &fail,
        )
        .test();
    }
}
