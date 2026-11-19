use crate::{
    context::LintContext,
    rules::{
        Rule,
        value_utils::{
            PatternMatcher, ValueTokenKind, at_rule_prelude_span, declaration_value_span,
            tokenize_value,
        },
    },
};
use oxc_css_parser::ast::{AtRule, Declaration, InterpolableIdent};

#[derive(Default)]
pub struct ContainerNamePattern {
    pub matcher: PatternMatcher,
}

const IGNORED_KEYWORDS: &[&str] = &[
    "none",
    "and",
    "not",
    "or",
    "normal",
    "auto",
    "inherit",
    "initial",
    "unset",
    "revert",
    "revert-layer",
];

impl Rule for ContainerNamePattern {
    fn name(&self) -> &'static str {
        "container-name-pattern"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if !prop.eq_ignore_ascii_case("container-name") {
            return;
        }
        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };

        for tok in tokenize_value(ctx.source, val_span) {
            if tok.kind == ValueTokenKind::Ident
                && !IGNORED_KEYWORDS
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(tok.raw))
                && !self.matcher.is_match(tok.raw)
            {
                ctx.report(
                    self.name(),
                    tok.span,
                    format!(
                        "Expected container name \"{}\" to match pattern \"{}\"",
                        tok.raw, self.matcher.raw_pattern
                    ),
                );
            }
        }
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if !name.eq_ignore_ascii_case("container") {
            return;
        }
        let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };
        let tokens = tokenize_value(ctx.source, p_span);
        if let Some(first) = tokens.first() {
            if first.kind == ValueTokenKind::Ident
                && !IGNORED_KEYWORDS
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(first.raw))
                && !self.matcher.is_match(first.raw)
            {
                ctx.report(
                    self.name(),
                    first.span,
                    format!(
                        "Expected container name \"{}\" to match pattern \"{}\"",
                        first.raw, self.matcher.raw_pattern
                    ),
                );
            }
        }
    }
}
