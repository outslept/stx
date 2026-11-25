use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::Declaration;

pub struct KeyframeDeclarationNoImportant;

impl Rule for KeyframeDeclarationNoImportant {
    fn name(&self) -> &'static str {
        "keyframe-declaration-no-important"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if let Some(important) = &decl.important {
            if ctx.in_keyframes() {
                ctx.report(
                    self.name(),
                    important.span,
                    "Disallowed !important".to_string(),
                );
            }
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
        let pass = [
            "a { color: pink !important; }",
            "@keyframes important { from { margin: 1px; } }",
            "@-webkit-keyframes important { from { margin: 1px; } }",
            "@-WEBKIT-KEYFRAMES important { from { margin: 1px; } }",
            "@non-keyframes important { from { margin: 1px !important; } }",
            "@keyframes-non important { from { margin: 1px !important; } }",
        ];
        let fail = [
            reject(
                "@keyframes important { from { margin: 1px !important; } }",
                None,
                REJECTED,
                (1, 43, 1, 53),
            ),
            reject(
                "@-webkit-keyframes important { from { margin: 1px !important; } }",
                None,
                REJECTED,
                (1, 51, 1, 61),
            ),
            reject(
                "@-WEBKIT-KEYFRAMES important { from { margin: 1px !important; } }",
                None,
                REJECTED,
                (1, 51, 1, 61),
            ),
            reject(
                "@keyframes important { from { margin: 1px!important; } }",
                None,
                REJECTED,
                (1, 42, 1, 52),
            ),
            reject(
                "@keyframes important { from { margin: 1px ! important; } }",
                None,
                REJECTED,
                (1, 43, 1, 54),
            ),
            reject(
                "@kEyFrAmEs important { from { margin: 1px !important; } }",
                None,
                REJECTED,
                (1, 43, 1, 53),
            ),
            reject(
                "@KEYFRAMES important { from { margin: 1px !important; } }",
                None,
                REJECTED,
                (1, 43, 1, 53),
            ),
        ];

        Tester::new(
            "keyframe-declaration-no-important",
            json!([true]),
            pass,
            fail,
        )
        .test();
    }
}
