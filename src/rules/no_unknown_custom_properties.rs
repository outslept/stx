use std::collections::HashSet;

use oxc_css_parser::ast::{Statement, Stylesheet};

use crate::{
    context::LintContext,
    rules::Rule,
    utils::{ValueTokenKind, at_rule_prelude_span, decl_value_span, ident_name, tokenize_value},
};

crate::define_stx_lint! {
    /// Report `var()` references to undeclared custom properties without a fallback.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/no-unknown-custom-properties>
    ///
    /// Enable with `true`; there are no rule-specific options. Custom property
    /// declarations and `@property` registrations anywhere in the same stylesheet
    /// count, regardless of selector scope or order. Other stylesheets are not checked.
    ///
    /// ```css
    /// a { color: var(--link-color); } /* Reported if --link-color is undeclared */
    /// b { color: var(--link-color, blue); } /* Has a fallback */
    /// ```
    "no-unknown-custom-properties", correctness;
    #[derive(Debug, Default)]
    pub struct NoUnknownCustomProperties;
}

pub fn rejected_message(property: &str) -> String {
    format!("Unknown custom property \"{property}\"")
}

impl Rule for NoUnknownCustomProperties {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let stylesheet = ctx.stylesheet;
        let source = ctx.source;
        let mut declared = HashSet::new();
        collect(stylesheet, source, &mut declared);
        check(stylesheet, &declared, ctx);
    }
}

fn collect<'a>(stylesheet: &'a Stylesheet<'a>, source: &'a str, declared: &mut HashSet<String>) {
    collect_statements(&stylesheet.statements, source, declared);
}

fn collect_statements<'a>(
    statements: &[Statement<'a>],
    source: &'a str,
    declared: &mut HashSet<String>,
) {
    for statement in statements {
        match statement {
            Statement::Declaration(decl) => {
                if let Some(name) = ident_name(&decl.name)
                    && name.starts_with("--")
                {
                    declared.insert(name.to_string());
                }
            }
            Statement::QualifiedRule(rule) => {
                collect_statements(&rule.block.statements, source, declared);
            }
            Statement::AtRule(at_rule) => {
                if at_rule.name.name.eq_ignore_ascii_case("property")
                    && let Some(prelude) = at_rule_prelude_span(source, at_rule)
                {
                    for part in source[prelude.start as usize..prelude.end as usize].split(',') {
                        let name = part.trim();
                        if name.starts_with("--") {
                            declared.insert(name.to_string());
                        }
                    }
                }
                if let Some(block) = &at_rule.block {
                    collect_statements(&block.statements, source, declared);
                }
            }
            Statement::KeyframeBlock(block) => {
                collect_statements(&block.block.statements, source, declared);
            }
            _ => {}
        }
    }
}

fn check<'a>(
    stylesheet: &'a Stylesheet<'a>,
    declared: &HashSet<String>,
    ctx: &mut LintContext<'a>,
) {
    check_statements(&stylesheet.statements, declared, ctx);
}

fn check_statements<'a>(
    statements: &[Statement<'a>],
    declared: &HashSet<String>,
    ctx: &mut LintContext<'a>,
) {
    let source = ctx.source;
    for statement in statements {
        match statement {
            Statement::Declaration(decl) => {
                let Some(value_span) = decl_value_span(source, decl) else {
                    continue;
                };
                let tokens = tokenize_value(source, value_span);
                for (index, token) in tokens.iter().enumerate() {
                    if token.kind != ValueTokenKind::Function {
                        continue;
                    }
                    if !token
                        .raw
                        .strip_suffix('(')
                        .unwrap_or(token.raw)
                        .eq_ignore_ascii_case("var")
                    {
                        continue;
                    }
                    let Some(first) = tokens.get(index + 1) else {
                        continue;
                    };
                    if first.kind == ValueTokenKind::CloseParen || !first.raw.starts_with("--") {
                        continue;
                    }
                    if tokens
                        .get(index + 2)
                        .is_some_and(|next| next.kind == ValueTokenKind::Comma)
                    {
                        continue;
                    }
                    if declared.contains(first.raw) {
                        continue;
                    }
                    ctx.report(
                        NoUnknownCustomProperties::NAME,
                        first.span,
                        rejected_message(first.raw),
                    );
                }
            }
            Statement::QualifiedRule(rule) => {
                check_statements(&rule.block.statements, declared, ctx);
            }
            Statement::AtRule(at_rule) => {
                if let Some(block) = &at_rule.block {
                    check_statements(&block.statements, declared, ctx);
                }
            }
            Statement::KeyframeBlock(block) => {
                check_statements(&block.block.statements, declared, ctx);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn default_config() {
        let pass = vec![
            "a { --foo: #f00; color: var(--foo); }",
            "a { --foo: #f00; border: solid var(--foo); }",
            "a { color: var(--foo, #f00); }",
            "a { --foo: #f00; color: var(--bar, var(--foo)); }",
            "@property --foo { syntax: \"<color>\"; inherits: false; initial-value: #f00; }\na { color: var(--foo); }",
            "@property --foo, --bar { syntax: \"<color>\"; inherits: false; initial-value: #f00; }\na { color: var(--bar); }",
            "a { color: var(); }",
        ];

        let fail = vec![
            reject(
                "a { color: var(--foo); }",
                None,
                rejected_message("--foo"),
                (1, 16, 1, 21),
            ),
            reject(
                "a { border: solid var(--foo); }",
                None,
                rejected_message("--foo"),
                (1, 23, 1, 28),
            ),
            reject(
                "a { color: var(--foo, var(--bar)); }",
                None,
                rejected_message("--bar"),
                (1, 27, 1, 32),
            ),
        ];

        Tester::new("no-unknown-custom-properties", json!(true), &pass, &fail).test();
    }
}
