use crate::{
    context::LintContext,
    rules::Rule,
    utils::{
        ANIMATION_NAME_KEYWORDS, ANIMATION_SHORTHAND_KEYWORDS, ValueTokenKind,
        at_rule_prelude_span, decl_value_span, ident_name, is_standard_syntax_value,
        tokenize_value,
    },
};
use oxc_css_parser::ast::{AtRule, Declaration, Statement, Stylesheet};
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct NoUnknownAnimations;

fn collect<'a>(at_rules: &mut Vec<&'a AtRule<'a>>, statements: &'a [Statement<'a>]) {
    for st in statements {
        match st {
            Statement::AtRule(a) => {
                at_rules.push(a);
                if let Some(b) = &a.block {
                    collect(at_rules, &b.statements);
                }
            }
            Statement::QualifiedRule(r) => collect(at_rules, &r.block.statements),
            Statement::KeyframeBlock(k) => collect(at_rules, &k.block.statements),
            _ => {}
        }
    }
}
fn collect_decls<'a>(decls: &mut Vec<&'a Declaration<'a>>, statements: &'a [Statement<'a>]) {
    for st in statements {
        match st {
            Statement::Declaration(d) => decls.push(d),
            Statement::QualifiedRule(r) => collect_decls(decls, &r.block.statements),
            Statement::AtRule(a) => {
                if let Some(b) = &a.block {
                    collect_decls(decls, &b.statements);
                }
            }
            Statement::KeyframeBlock(k) => collect_decls(decls, &k.block.statements),
            _ => {}
        }
    }
}

impl Rule for NoUnknownAnimations {
    fn name(&self) -> &'static str {
        "no-unknown-animations"
    }
    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let stylesheet = ctx.stylesheet;
        let source = ctx.source;
        let mut at_rules = Vec::new();
        collect(&mut at_rules, &stylesheet.statements);
        let mut declared = HashSet::new();
        for at_rule in &at_rules {
            let lower = at_rule.name.name.to_ascii_lowercase();
            if (lower == "keyframes" || lower == "-webkit-keyframes")
                && let Some(sp) = at_rule_prelude_span(source, at_rule)
            {
                declared.insert(
                    source[sp.start as usize..sp.end as usize]
                        .trim()
                        .to_string(),
                );
            }
        }
        let mut decls = Vec::new();
        collect_decls(&mut decls, &stylesheet.statements);
        for decl in decls {
            let Some(prop) = ident_name(&decl.name) else {
                continue;
            };
            if !prop.eq_ignore_ascii_case("animation")
                && !prop.eq_ignore_ascii_case("animation-name")
            {
                continue;
            }
            let Some(vspan) = decl_value_span(source, decl) else {
                continue;
            };
            let tokens = tokenize_value(source, vspan);
            let mut depth = 0usize;
            let mut skip_from: Option<usize> = None;
            let mut non_standard = false;
            for token in &tokens {
                match token.kind {
                    ValueTokenKind::Function => {
                        depth += 1;
                        if skip_from.is_none() {
                            skip_from = Some(depth);
                        }
                    }
                    ValueTokenKind::CloseParen => {
                        depth = depth.saturating_sub(1);
                        if skip_from.is_some_and(|f| depth < f) {
                            skip_from = None;
                        }
                    }
                    _ if skip_from.is_some() => {}
                    ValueTokenKind::Ident => {
                        if !is_standard_syntax_value(token.raw) {
                            non_standard = true;
                            break;
                        }
                        let lower = token.raw.to_ascii_lowercase();
                        if ANIMATION_NAME_KEYWORDS.contains(&lower.as_str())
                            || ANIMATION_SHORTHAND_KEYWORDS.contains(&lower.as_str())
                        {
                            continue;
                        }
                        if declared.contains(token.raw) {
                            continue;
                        }
                        ctx.report(
                            self.name(),
                            token.span,
                            format!("Unknown animation \"{}\"", token.raw),
                        );
                    }
                    _ => {}
                }
            }
            let _ = non_standard;
        }
    }
}
#[cfg(test)]
mod tests {

    use crate::tester::{Tester, reject, reject_message};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec![
            "@keyframes foo {} a { animation-name: foo; }",
            "a { animation-name: none; }",
            "a { animation-name: initial; }",
            "a { animation-name: foo; } @keyframes foo {}",
            "a { animation: none; }",
            "@keyframes foo {} a { animation: foo 2s linear; }",
            "@-webkit-keyframes foo {} a { animation: foo 2s linear; }",
        ];
        let fail = vec![
            reject(
                "a { animation-name: bar; }",
                None,
                "Unknown animation \"bar\"",
                (1, 21, 1, 24),
            ),
            reject_message(
                "@keyframes foo {} a { animation: bar 2s linear; }",
                None,
                "Unknown animation \"bar\"",
            ),
            reject_message(
                "a { animation: 2s linear bar; }",
                None,
                "Unknown animation \"bar\"",
            ),
        ];
        Tester::new("no-unknown-animations", json!(true), &pass, &fail).test();
    }
}
