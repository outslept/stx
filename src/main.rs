mod context;
mod diagnostic;
mod line_index;
mod rules;
mod visitor;

use context::LintContext;
use diagnostic::StylelintResult;
use line_index::LineIndex;
use oxc_allocator::Allocator;
use oxc_css_parser::{ParserBuilder, Syntax, ast::Stylesheet};
use rules::default_rules;
use visitor::SinglePassVisitor;

fn lint_str<'a>(source_name: &'a str, source: &str) -> StylelintResult<'a> {
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
    let active_rules = default_rules();

    let mut ctx = LintContext::new(source, &stylesheet, comments, &line_index);
    SinglePassVisitor::new(&mut ctx, &active_rules).visit_stylesheet(&stylesheet);

    let warnings = ctx.into_warnings();
    StylelintResult {
        source: source_name,
        errored: !warnings.is_empty(),
        warnings,
    }
}

fn main() {
    let source = r#"
/*   */

#header .btn {
    color: #12345 !important;
}

.empty {}

@keyframes fade {
    from {
        opacity: 0 !important;
    }
}
"#;

    let result = vec![lint_str("test.css", source)];
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
