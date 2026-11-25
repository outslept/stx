use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rules::Rule;
use oxc_css_parser::token::Comment;
use serde_json::Value;

crate::define_stx_lint! {
    /// Flag block comments that contain only whitespace or no text.
    ///
    /// Enable with `true`; there are no rule-specific secondary options.
    /// Preprocessor line comments (`//`) are not checked.
    ///
    /// ```css
    /// /* */               /* reported */
    /// /* Print styles */  /* accepted */
    /// ```
    "comment-no-empty", suspicious, recommended = true;
    pub struct CommentNoEmpty {
        pub severity: Severity,
    }
}

pub const REJECTED_MSG: &str = "Empty comment";

impl CommentNoEmpty {
    pub fn from_config(primary: &Value, severity: Severity) -> Option<Self> {
        if primary.as_bool() != Some(true) {
            return None;
        }
        Some(Self { severity })
    }
}

impl Rule for CommentNoEmpty {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_comment<'a>(&self, comment: &'a Comment<'a>, ctx: &mut LintContext<'a>) {
        let raw = ctx.slice(comment.span);

        if !(raw.starts_with("/*") && raw.ends_with("*/") && raw.len() >= 4) {
            return;
        }

        let text = raw[2..raw.len() - 2].trim();
        if !text.is_empty() {
            return;
        }

        ctx.report(self.name(), comment.span, REJECTED_MSG.to_string());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject_start};

    const RULE_NAME: &str = "comment-no-empty";
    const REJECTED: &str = "Empty comment";

    #[test]
    fn default_config() {
        let pass = [
            "/* comment */",
            "/*\n multi-line comment\n */\n",
            "a { color: red; /* comment */ }",
        ];
        let fail = [
            reject_start("/**/", None, REJECTED, (1, 1)),
            reject_start(" /* */", None, REJECTED, (1, 2)),
            reject_start("\n/*\n*/", None, REJECTED, (2, 1)),
            reject_start("\n /*\r\n*/", None, REJECTED, (2, 2)),
            reject_start("/*\t\t*/", None, REJECTED, (1, 1)),
            reject_start("a {/*\r\n*/\n color: red;}", None, REJECTED, (1, 4)),
        ];

        Tester::new(RULE_NAME, json!([true]), pass, fail).test();
    }

    #[test]
    fn scss_syntax() {
        let pass = ["//", "a { color: red; } // "];

        Tester::new(RULE_NAME, json!([true]), pass, [])
            .with_custom_syntax("postcss-scss")
            .test();
    }
}
