use crate::{
    context::LintContext,
    rules::{Rule, value_utils::PatternMatcher},
};
use oxc_css_parser::ast::{Declaration, InterpolableIdent};

#[derive(Default)]
pub struct CustomPropertyPattern {
    pub matcher: PatternMatcher,
}

impl Rule for CustomPropertyPattern {
    fn name(&self) -> &'static str {
        "custom-property-pattern"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if let Some(stripped) = prop.strip_prefix("--") {
            if !self.matcher.is_match(stripped) {
                ctx.report(
                    self.name(),
                    ident.span,
                    format!(
                        "Expected custom property name \"{}\" to match pattern \"{}\"",
                        prop, self.matcher.raw_pattern
                    ),
                );
            }
        }
    }
}
