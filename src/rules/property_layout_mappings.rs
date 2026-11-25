use crate::{
    context::{AstParent, LintContext},
    rules::Rule,
    utils::{
        FLOW_RELATIVE_PROPERTIES, OptionMatcher, ident_name, options_matches,
        parse_option_matchers, physical_shorthand_to_flow_relative_by_axis,
        physical_to_flow_relative_property,
    },
};
use oxc_css_parser::{
    Span,
    ast::{ComponentValue, Declaration, DelimiterKind},
};

crate::define_stx_lint! {
    /// Enforce physical or flow-relative layout properties.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/property-layout-mappings>
    ///
    /// Set the primary option to `"physical"` or `"flow-relative"` to allow
    /// that mapping. `ignoreProperties` accepts exact names or slash-delimited
    /// regexes. The check includes property references in `transition`,
    /// `transition-property`, and `will-change`. Outside those references, it
    /// skips declarations inside `@page`. Flow-relative mode also reports
    /// supported multi-value physical shorthands. No edits are applied.
    ///
    /// With `"flow-relative"`, use `margin-inline-start` instead of `margin-left`
    /// when targeting the start side of the inline axis:
    ///
    /// ```css
    /// .card { margin-inline-start: 1rem; }
    /// ```
    "property-layout-mappings", restriction;
    pub struct PropertyLayoutMappings {
        physical: bool,
        ignore: Vec<OptionMatcher>,
    }
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

    fn disallowed(&self, lower: &str) -> bool {
        if self.physical {
            FLOW_RELATIVE_PROPERTIES.contains(&lower)
        } else {
            physical_to_flow_relative_property(lower).is_some()
        }
    }

    fn report(&self, ctx: &mut LintContext<'_>, span: Span, prop: &str) {
        let kind = if self.physical {
            "flow-relative"
        } else {
            "physical"
        };
        ctx.report(
            self.name(),
            span,
            format!("Disallowed {kind} property \"{prop}\""),
        );
    }

    fn check_references(&self, values: &[ComponentValue<'_>], ctx: &mut LintContext<'_>) {
        for value in values {
            match value {
                ComponentValue::InterpolableIdent(ident) if ident_name(ident).is_some() => {
                    let raw = ctx.slice(ident.span());
                    if self.disallowed(&raw.to_ascii_lowercase())
                        && !options_matches(&self.ignore, raw)
                    {
                        self.report(ctx, ident.span(), raw);
                    }
                }
                ComponentValue::Function(func) => self.check_references(&func.args, ctx),
                _ => {}
            }
        }
    }
}
impl Rule for PropertyLayoutMappings {
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
        let lower = prop.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "transition" | "transition-property" | "will-change"
        ) {
            self.check_references(&decl.value, ctx);
            return;
        }
        if ctx.ancestors.iter().any(|parent| {
            matches!(parent, AstParent::AtRule(at) if at.name.name.eq_ignore_ascii_case("page"))
        }) {
            return;
        }
        let physical_shorthand = !self.physical
            && !decl.value_is_raw
            && physical_shorthand_to_flow_relative_by_axis(&lower)
                .is_some_and(|(_, _, axes)| has_multiple_components(&decl.value, axes));
        if physical_shorthand || self.disallowed(&lower) {
            self.report(ctx, decl.name.span(), prop);
        }
    }
}
fn has_multiple_components(values: &[ComponentValue<'_>], axes: bool) -> bool {
    let mut count = 0;
    for value in values {
        match value {
            ComponentValue::Delimiter(delim)
                if matches!(delim.kind, DelimiterKind::Comma | DelimiterKind::Solidus) =>
            {
                return false;
            }
            ComponentValue::InterpolableIdent(_)
            | ComponentValue::Number(_)
            | ComponentValue::Dimension(_)
            | ComponentValue::Percentage(_)
            | ComponentValue::HexColor(_)
            | ComponentValue::Function(_) => count += 1,
            _ => {}
        }
    }
    (2..=if axes { 2 } else { 4 }).contains(&count)
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;
    #[test]
    fn physical() {
        let pass = [
            "a { margin-left: 0; }",
            "a { width: 0; }",
            "@PAGE { @top-left { margin-inline-start: 0; } }",
        ];
        let fail = [
            reject_message(
                "a { margin-inline-start: 0; }",
                None,
                "Disallowed flow-relative property \"margin-inline-start\"",
            ),
            reject(
                "a { TRANSITION-PROPERTY: INLINE-SIZE; }",
                None,
                "Disallowed flow-relative property \"INLINE-SIZE\"",
                (1, 26, 1, 37),
            ),
        ];
        Tester::new("property-layout-mappings", json!(["physical"]), pass, fail).test();
    }
    #[test]
    fn flow_relative() {
        let pass = [
            "a { margin-inline-start: 0; }",
            "a { inline-size: 0; }",
            "a { margin: calc(0px + 1px); overflow: hidden auto scroll; }",
            "a { margin: 0, 1px; }",
            "a { will-change: \"width\"; }",
            "@page { margin-left: 0; margin: 0 1px; }",
        ];
        let fail = [
            reject_message(
                "a { margin-left: 0; }",
                None,
                "Disallowed physical property \"margin-left\"",
            ),
            reject_message(
                "a { margin: 0 calc(1px + 2px); }",
                None,
                "Disallowed physical property \"margin\"",
            ),
            reject(
                "a { transition: var(--foo, left) 0 ease; }",
                None,
                "Disallowed physical property \"left\"",
                (1, 28, 1, 32),
            ),
        ];
        Tester::new(
            "property-layout-mappings",
            json!(["flow-relative"]),
            pass,
            fail,
        )
        .test();
    }
}
