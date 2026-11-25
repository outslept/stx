use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::Declaration;

crate::define_stx_lint! {
    /// Disallows `!important` in declarations.
    ///
    /// Set the primary option to `true`.
    ///
    /// ```css
    /// a { color: pink !important; } /* reported */
    /// ```
    "declaration-no-important", restriction;
    pub struct DeclarationNoImportant;
}

impl Rule for DeclarationNoImportant {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if let Some(important) = &decl.important {
            ctx.report(
                self.name(),
                important.span,
                "Disallowed !important".to_string(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    const REJECTED: &str = "Disallowed !important";

    #[test]
    fn default_config() {
        let pass = ["a { color: pink; }"];
        let fail = [
            reject(
                "a { color: pink !important; }",
                None,
                REJECTED,
                (1, 17, 1, 27),
            ),
            reject(
                "a { color: pink ! important; }",
                None,
                REJECTED,
                (1, 17, 1, 28),
            ),
            reject(
                "a { color: pink!important; }",
                None,
                REJECTED,
                (1, 16, 1, 26),
            ),
        ];

        Tester::new("declaration-no-important", json!([true]), pass, fail).test();
    }
}
