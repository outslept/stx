mod line_index;

use line_index::LineIndex;
use oxc_allocator::Allocator;
use oxc_css_parser::{
    ParserBuilder, Syntax,
    ast::{SimpleBlock, Statement, Stylesheet},
    token::Comment,
};
use serde::Serialize;

#[derive(Serialize)]
struct StylelintWarning {
    line: usize,
    column: usize,
    #[serde(rename = "endLine")]
    end_line: usize,
    #[serde(rename = "endColumn")]
    end_column: usize,
    rule: &'static str,
    severity: &'static str,
    text: String,
}

#[derive(Serialize)]
struct StylelintResult<'a> {
    source: &'a str,
    errored: bool,
    warnings: Vec<StylelintWarning>,
}

fn check_block_no_empty(
    block: &SimpleBlock<'_>,
    comments: &[Comment<'_>],
    source: &str,
    line_index: &LineIndex,
    warnings: &mut Vec<StylelintWarning>,
) {
    if !block.statements.is_empty() {
        return;
    }

    let has_comment_inside = comments
        .binary_search_by(|c| {
            if c.span.end <= block.span.start {
                std::cmp::Ordering::Less
            } else if c.span.start >= block.span.end {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok();

    if has_comment_inside {
        return;
    }

    let (line, column) = line_index.line_col(source, block.span.start);
    let (end_line, end_column) = line_index.line_col(source, block.span.end);

    warnings.push(StylelintWarning {
        line,
        column,
        end_line,
        end_column,
        rule: "block-no-empty",
        severity: "error",
        text: "Unexpected empty block (block-no-empty)".to_string(),
    });
}

fn walk_statements(
    statements: &[Statement<'_>],
    comments: &[Comment<'_>],
    source: &str,
    line_index: &LineIndex,
    warnings: &mut Vec<StylelintWarning>,
) {
    for stmt in statements {
        match stmt {
            Statement::QualifiedRule(rule) => {
                check_block_no_empty(&rule.block, comments, source, line_index, warnings);
                walk_statements(
                    &rule.block.statements,
                    comments,
                    source,
                    line_index,
                    warnings,
                );
            }
            Statement::AtRule(at_rule) => {
                if let Some(block) = &at_rule.block {
                    check_block_no_empty(block, comments, source, line_index, warnings);
                    walk_statements(&block.statements, comments, source, line_index, warnings);
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let source = r#"
.valid {
    color: #fff;
}

.empty-with-comment {
    /* keep */
}

.invalid-empty {}

@media (min-width: 768px) {
    .nested-empty {
    }
}
"#;

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
    let mut warnings = Vec::new();

    walk_statements(
        &stylesheet.statements,
        comments,
        source,
        &line_index,
        &mut warnings,
    );

    let result = vec![StylelintResult {
        source: "test.css",
        errored: !warnings.is_empty(),
        warnings,
    }];

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
