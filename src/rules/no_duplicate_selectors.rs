use std::collections::HashSet;

use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::{SimpleBlock, Statement, Stylesheet};

pub struct NoDuplicateSelectors;

fn normalize_selector_list(raw: &str) -> String {
    let mut parts: Vec<String> = raw
        .split(',')
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    parts.sort();
    parts.join(",")
}

fn check_statements_for_duplicates<'a>(
    statements: &'a [Statement<'a>],
    rule_name: &'static str,
    ctx: &mut LintContext<'a>,
) {
    let mut seen = HashSet::new();

    for stmt in statements {
        if let Statement::QualifiedRule(qrule) = stmt {
            let sel_span = qrule.selector.span;
            let raw_sel = ctx.slice(sel_span).trim();
            if raw_sel.is_empty() {
                continue;
            }
            let normalized = normalize_selector_list(raw_sel);
            if !seen.insert(normalized) {
                ctx.report(
                    rule_name,
                    sel_span,
                    format!("Unexpected duplicate selector \"{raw_sel}\""),
                );
            }
        }
    }
}

impl Rule for NoDuplicateSelectors {
    fn name(&self) -> &'static str {
        "no-duplicate-selectors"
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        check_statements_for_duplicates(&stylesheet.statements, self.name(), ctx);
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        check_statements_for_duplicates(&block.statements, self.name(), ctx);
    }
}
