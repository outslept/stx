use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        FLOW_RELATIVE_PROPERTIES, OptionMatcher, ident_name, is_standard_syntax_declaration,
        options_matches, parse_option_matchers, physical_to_flow_relative_property,
    },
};
use oxc_css_parser::{Span, ast::Declaration};
pub struct PropertyLayoutMappings {
    physical: bool,
    ignore: Vec<OptionMatcher>,
}
impl PropertyLayoutMappings {
    pub fn from_config(
        p: &serde_json::Value,
        s: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Option<Self> {
        let physical = match p.as_str()? {
            "physical" => true,
            "flow-relative" => false,
            _ => return None,
        };
        Some(Self {
            physical,
            ignore: parse_option_matchers(s, "ignoreProperties"),
        })
    }
}
impl Rule for PropertyLayoutMappings {
    fn name(&self) -> &'static str {
        "property-layout-mappings"
    }
    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let Some(prop) = ident_name(&decl.name) else {
            return;
        };
        if !is_standard_syntax_declaration(decl) || options_matches(&self.ignore, prop) {
            return;
        }
        let lower = prop.to_ascii_lowercase();
        let disallowed = if self.physical {
            FLOW_RELATIVE_PROPERTIES.contains(&lower.as_str())
        } else {
            physical_to_flow_relative_property(&lower).is_some()
        };
        if !disallowed {
            return;
        }
        let kind = if self.physical {
            "flow-relative"
        } else {
            "physical"
        };
        ctx.report(
            self.name(),
            Span {
                start: decl.span.start,
                end: decl.span.start + prop.len() as u32,
            },
            format!("Disallowed {kind} property \"{prop}\""),
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject_message};
    use serde_json::json;
    #[test]
    fn physical() {
        let pass = vec!["a { margin-left: 0; }", "a { width: 0; }"];
        let fail = vec![reject_message(
            "a { margin-inline-start: 0; }",
            None,
            "Disallowed flow-relative property \"margin-inline-start\"",
        )];
        Tester::new(
            "property-layout-mappings",
            json!(["physical"]),
            &pass,
            &fail,
        )
        .test();
    }
    #[test]
    fn flow_relative() {
        let pass = vec!["a { margin-inline-start: 0; }", "a { inline-size: 0; }"];
        let fail = vec![reject_message(
            "a { margin-left: 0; }",
            None,
            "Disallowed physical property \"margin-left\"",
        )];
        Tester::new(
            "property-layout-mappings",
            json!(["flow-relative"]),
            &pass,
            &fail,
        )
        .test();
    }
}
