use std::collections::HashMap;

use oxc_css_parser::ast::{InterpolableIdent, SimpleBlock, Statement};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        longhand_sub_properties_of_shorthand, physical_shorthand_to_flow_relative_properties,
        shorthand_to_reset_to_initial_property, vendor_prefix,
    },
};

pub struct DeclarationBlockNoShorthandPropertyOverrides;

impl Rule for DeclarationBlockNoShorthandPropertyOverrides {
    fn name(&self) -> &'static str {
        "declaration-block-no-shorthand-property-overrides"
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        let mut seen = HashMap::<String, HashMap<String, &str>>::new();

        for statement in &block.statements {
            let Statement::Declaration(decl) = statement else {
                continue;
            };
            let InterpolableIdent::Literal(ident) = &decl.name else {
                continue;
            };
            let raw = ident.raw;
            if raw.starts_with("--") {
                continue;
            }
            let lower = raw.to_ascii_lowercase();
            let prefix = vendor_prefix(&lower).unwrap_or_default();
            let property = &lower[prefix.len()..];
            let declarations = seen.entry(prefix.to_owned()).or_default();

            for &overridden in [
                longhand_sub_properties_of_shorthand(property),
                shorthand_to_reset_to_initial_property(property),
                physical_shorthand_to_flow_relative_properties(property),
            ]
            .into_iter()
            .flatten()
            .flatten()
            {
                if let Some(original) = declarations.get(overridden) {
                    ctx.report(
                        self.name(),
                        ident.span,
                        format!("Overridden property \"{original}\" by shorthand \"{raw}\""),
                    );
                }
            }

            declarations.insert(property.to_owned(), raw);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    #[test]
    fn declaration_block_no_shorthand_property_overrides() {
        let pass = [
            "a { padding: 10px; padding-inline: 5px; }",
            "a { padding-left: 10px; padding-inline: 5px; }",
            "a { padding-inline: 5px; & b { padding: 10px; } }",
            "a { -webkit-transition-property: opacity; transition: opacity 1s; }",
            "a { border-block: 1px solid; border-width: 2px; }",
        ];
        let fail = [
            reject(
                "a { padding-inline: 5px; padding: 10px; }",
                None,
                "Overridden property \"padding-inline\" by shorthand \"padding\"",
                (1, 26, 1, 33),
            ),
            reject(
                "a { padding-left: 0; PADDING-LEFT: 5px; padding: 10px; }",
                None,
                "Overridden property \"PADDING-LEFT\" by shorthand \"padding\"",
                (1, 41, 1, 48),
            ),
            reject(
                "a { -WEBKIT-transition-property: opacity; -webKIT-transition: opacity 1s linear; }",
                None,
                "Overridden property \"-WEBKIT-transition-property\" by shorthand \"-webKIT-transition\"",
                (1, 43, 1, 61),
            ),
            reject(
                "a { font-size-adjust: 0.545; font: Verdana; }",
                None,
                "Overridden property \"font-size-adjust\" by shorthand \"font\"",
                (1, 30, 1, 34),
            ),
            reject_warnings(
                "a { border-inline-width: 2px; border-block: 1px solid; border: 1px solid; }",
                None,
                vec![
                    (
                        "Overridden property \"border-block\" by shorthand \"border\"".into(),
                        (1, 56, 1, 62),
                    ),
                    (
                        "Overridden property \"border-inline-width\" by shorthand \"border\""
                            .into(),
                        (1, 56, 1, 62),
                    ),
                ],
            ),
        ];

        Tester::new(
            "declaration-block-no-shorthand-property-overrides",
            json!([true]),
            pass,
            fail,
        )
        .with_fix(true)
        .test();
    }
}
