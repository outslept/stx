pub(crate) mod context;
pub(crate) mod disable;
pub(crate) mod fix;
pub(crate) mod line_index;
pub(crate) mod visitor;

use crate::{diagnostic::StylelintWarning, rules::ConfiguredRule};
use context::LintContext;
use disable::DisableDirectives;
use line_index::LineIndex;
use oxc_css_parser::{Allocator, ParserBuilder, Syntax, ast::Stylesheet, token::Comment};
use visitor::SinglePassVisitor;

#[derive(Default)]
pub struct LintPass {
    pub warnings: Vec<StylelintWarning>,
    pub fixes: Vec<fix::Fix>,
    pub parse_error: Option<StylelintWarning>,
}

impl LintPass {
    pub fn into_warnings(self) -> Vec<StylelintWarning> {
        let mut warnings = self.warnings;
        warnings.extend(self.parse_error);
        warnings.sort_by_key(|w| (w.line, w.column, w.rule));
        warnings
    }
}

fn run_pass<'a>(
    input: &'a str,
    stylesheet: &'a Stylesheet<'a>,
    comments: &'a [Comment<'a>],
    syntax: Syntax,
    active_rules: &[ConfiguredRule],
    quiet: bool,
) -> LintPass {
    let source = input.strip_prefix('\u{feff}').unwrap_or(input);
    let bom_len = (input.len() - source.len()) as u32;
    let line_index = LineIndex::new(source);
    let disables = DisableDirectives::parse(comments, source, &line_index);
    let mut ctx = LintContext::new(
        source,
        stylesheet,
        comments,
        &line_index,
        &disables,
        bom_len != 0,
        syntax,
    );
    SinglePassVisitor::new(&mut ctx, active_rules).visit_stylesheet(stylesheet);
    let fixes = ctx.fixes();
    let mut pass = LintPass {
        warnings: ctx.into_warnings(quiet),
        fixes,
        parse_error: None,
    };
    if bom_len != 0 {
        for fix in pass
            .fixes
            .iter_mut()
            .chain(pass.warnings.iter_mut().filter_map(|w| w.fix.as_mut()))
        {
            for edit in &mut fix.edits {
                edit.span.start += bom_len;
                edit.span.end += bom_len;
            }
        }
    }
    pass
}

pub fn lint_once(
    input: &str,
    syntax: Syntax,
    active_rules: &[ConfiguredRule],
    quiet: bool,
) -> LintPass {
    let allocator = Allocator::default();
    let mut parser = ParserBuilder::new(&allocator, input)
        .syntax(syntax)
        .comments()
        .build();
    let err = match parser.parse::<Stylesheet>() {
        Ok(stylesheet) => {
            return run_pass(
                input,
                &stylesheet,
                parser.comments(),
                syntax,
                active_rules,
                quiet,
            );
        }
        Err(err) => err,
    };
    if input.chars().any(|c| !c.is_ascii() && c.is_whitespace()) {
        let mut sanitized = String::with_capacity(input.len());
        for ch in input.chars() {
            if !ch.is_ascii() && ch.is_whitespace() {
                sanitized.push_str(&"    "[..ch.len_utf8()]);
            } else {
                sanitized.push(ch);
            }
        }
        let alloc2 = Allocator::default();
        let mut parser2 = ParserBuilder::new(&alloc2, &sanitized)
            .syntax(syntax)
            .comments()
            .build();
        if let Ok(stylesheet) = parser2.parse::<Stylesheet>() {
            return run_pass(
                input,
                &stylesheet,
                parser2.comments(),
                syntax,
                active_rules,
                quiet,
            );
        }
    }
    let empty_alloc = Allocator::default();
    let mut empty_parser = ParserBuilder::new(&empty_alloc, "")
        .syntax(syntax)
        .comments()
        .build();
    let mut pass = match empty_parser.parse::<Stylesheet>() {
        Ok(stylesheet) => run_pass(
            input,
            &stylesheet,
            empty_parser.comments(),
            syntax,
            active_rules,
            quiet,
        ),
        Err(_) => LintPass::default(),
    };
    let source = input.strip_prefix('\u{feff}').unwrap_or(input);
    let index = LineIndex::new(source);
    let (line, column) = index.line_col(source, err.span.start);
    let (end_line, end_column) = index.line_col(source, err.span.end);
    pass.parse_error = Some(StylelintWarning {
        line,
        column,
        end_line,
        end_column,
        rule: "CssSyntaxError",
        severity: "error",
        text: format!("{} (CssSyntaxError)", err.kind),
        fix: None,
    });
    pass
}

pub fn lint_and_fix(
    source: &str,
    syntax: Syntax,
    active_rules: &[ConfiguredRule],
    quiet: bool,
) -> (String, LintPass) {
    let pass = lint_once(source, syntax, active_rules, quiet);
    fix::fix_from_pass(source, syntax, active_rules, quiet, pass)
}
