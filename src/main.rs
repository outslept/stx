mod context;
mod diagnostic;
mod disable;
mod fix;
mod line_index;
mod rules;
mod visitor;

use context::LintContext;
use diagnostic::StylelintResult;
use disable::DisableDirectives;
use fix::apply_fixes;
use line_index::LineIndex;
use oxc_allocator::Allocator;
use oxc_css_parser::{ParserBuilder, Syntax, ast::Stylesheet};
use rules::{Rule, default_rules};
use visitor::SinglePassVisitor;

fn lint_once(
    source: &str,
    active_rules: &[Box<dyn Rule>],
) -> (Vec<diagnostic::StylelintWarning>, Vec<fix::Fix>) {
    let allocator = Allocator::default();
    let mut parser = ParserBuilder::new(&allocator, source)
        .syntax(Syntax::Css)
        .comments()
        .build();

    let stylesheet = parser
        .parse::<Stylesheet<'_>>()
        .expect("Failed to parse CSS");

    let comments = parser.comments();
    let line_index = LineIndex::new(source);
    let disables = DisableDirectives::parse(comments, source, &line_index);

    let mut ctx = LintContext::new(source, &stylesheet, comments, &line_index, &disables);
    SinglePassVisitor::new(&mut ctx, active_rules).visit_stylesheet(&stylesheet);

    let fixes = ctx.take_fixes();
    let warnings = ctx.into_warnings();
    (warnings, fixes)
}

pub fn lint_str<'a>(
    source_name: &'a str,
    source: &str,
    fix: bool,
) -> (StylelintResult<'a>, Option<String>) {
    let active_rules = default_rules();

    if !fix {
        let (warnings, _) = lint_once(source, &active_rules);
        return (
            StylelintResult {
                source: source_name,
                errored: !warnings.is_empty(),
                warnings,
            },
            None,
        );
    }

    let mut current = source.to_string();
    for _ in 0..5 {
        let (_, fixes) = lint_once(&current, &active_rules);
        let (next, applied) = apply_fixes(&current, fixes);
        current = next;
        if !applied {
            break;
        }
    }

    let (warnings, _) = lint_once(&current, &active_rules);
    (
        StylelintResult {
            source: source_name,
            errored: !warnings.is_empty(),
            warnings,
        },
        Some(current),
    )
}

fn main() {
    let source = r#"
.card {
    color: #ffffff;
    background: #11223344;
    /* stylelint-disable-next-line declaration-no-important -- intentional override */
    display: flex !important;
}

/* stylelint-disable block-no-empty */
.ignored-empty {}
/* stylelint-enable block-no-empty */

.reported-empty {}
"#;

    let (report_before, _) = lint_str("test.css", source, false);
    println!("--- BEFORE --fix ---");
    println!(
        "{}",
        serde_json::to_string_pretty(&vec![report_before]).unwrap()
    );

    let (report_after, fixed_code) = lint_str("test.css", source, true);
    println!("\n--- FIXED CSS ---");
    print!("{}", fixed_code.unwrap());
    println!("\n--- AFTER --fix ---");
    println!(
        "{}",
        serde_json::to_string_pretty(&vec![report_after]).unwrap()
    );
}
