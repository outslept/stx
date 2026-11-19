use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        value_utils::{ValueTokenKind, declaration_value_span, tokenize_value},
    },
};
use oxc_css_parser::ast::{Declaration, InterpolableIdent};

pub struct ShorthandPropertyNoRedundantValues;

const FOUR_VALUE_SHORTHANDS: &[&str] = &[
    "margin",
    "padding",
    "border-color",
    "border-style",
    "border-width",
    "border-radius",
    "inset",
    "scroll-margin",
    "scroll-padding",
];

const TWO_VALUE_SHORTHANDS: &[&str] = &[
    "gap",
    "grid-gap",
    "overscroll-behavior",
    "place-content",
    "place-items",
    "place-self",
];

impl Rule for ShorthandPropertyNoRedundantValues {
    fn name(&self) -> &'static str {
        "shorthand-property-no-redundant-values"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        let is_four = FOUR_VALUE_SHORTHANDS
            .iter()
            .any(|p| p.eq_ignore_ascii_case(prop));
        let is_two = TWO_VALUE_SHORTHANDS
            .iter()
            .any(|p| p.eq_ignore_ascii_case(prop));
        if !is_four && !is_two {
            return;
        }

        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };

        let tokens = tokenize_value(ctx.source, val_span);
        if tokens.is_empty() {
            return;
        }

        let mut parts: Vec<&str> = Vec::new();
        let mut depth = 0usize;
        let mut group_start: Option<u32> = None;

        for tok in &tokens {
            match tok.kind {
                ValueTokenKind::Comma | ValueTokenKind::Slash if depth == 0 => return,
                ValueTokenKind::Function => {
                    if depth == 0 && group_start.is_none() {
                        group_start = Some(tok.span.start);
                    }
                    depth += 1;
                }
                ValueTokenKind::CloseParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        if let Some(s) = group_start.take() {
                            parts.push(&ctx.source[s as usize..tok.span.end as usize]);
                        }
                    }
                }
                _ if depth == 0 => {
                    parts.push(tok.raw);
                }
                _ => {}
            }
        }

        if parts.len() < 2 || parts.len() > 4 {
            return;
        }
        if is_two && parts.len() != 2 {
            return;
        }

        let shortened: Option<Vec<&str>> = match parts.as_slice() {
            [a, b] if a.eq_ignore_ascii_case(b) => Some(vec![a]),
            [a, b, c] if a.eq_ignore_ascii_case(c) => {
                if a.eq_ignore_ascii_case(b) {
                    Some(vec![a])
                } else {
                    Some(vec![a, b])
                }
            }
            [a, b, c, d] => {
                if a.eq_ignore_ascii_case(b)
                    && b.eq_ignore_ascii_case(c)
                    && c.eq_ignore_ascii_case(d)
                {
                    Some(vec![a])
                } else if a.eq_ignore_ascii_case(c) && b.eq_ignore_ascii_case(d) {
                    Some(vec![a, b])
                } else if b.eq_ignore_ascii_case(d) {
                    Some(vec![a, b, c])
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(expected_parts) = shortened {
            let raw_val = ctx.slice(val_span);
            let expected = expected_parts.join(" ");
            ctx.report_with_fix(
                self.name(),
                val_span,
                format!("Expected \"{}\" to be \"{}\"", raw_val, expected),
                Some(Fix::single(val_span, expected)),
            );
        }
    }
}
