use crate::{
    config::Config,
    context::LintContext,
    diagnostic::StylelintWarning,
    disable::DisableDirectives,
    fix,
    line_index::LineIndex,
    rules::{ConfiguredRule, build_rules},
    visitor::SinglePassVisitor,
};
use oxc_css_parser::{Allocator, ParserBuilder, Syntax, ast::Stylesheet};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ExpectedWarning<'a> {
    pub message: Option<&'a str>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub end_line: Option<u32>,
    pub end_column: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct FailCase<'a> {
    pub code: &'a str,
    pub fixed: Option<&'a str>,
    pub unfixable: bool,
    pub warnings: Vec<ExpectedWarning<'a>>,
}

pub fn reject<'a>(
    code: &'a str,
    message: Option<&'a str>,
    line: Option<u32>,
    column: Option<u32>,
    end_line: Option<u32>,
    end_column: Option<u32>,
) -> FailCase<'a> {
    FailCase {
        code,
        fixed: None,
        unfixable: false,
        warnings: vec![ExpectedWarning {
            message,
            line,
            column,
            end_line,
            end_column,
        }],
    }
}

pub fn reject_warnings<'a>(code: &'a str, warnings: Vec<ExpectedWarning<'a>>) -> FailCase<'a> {
    FailCase {
        code,
        fixed: None,
        unfixable: false,
        warnings,
    }
}

pub fn reject_edit<'a>(
    code: &'a str,
    fixed: Option<&'a str>,
    unfixable: bool,
    message: Option<&'a str>,
    line: Option<u32>,
    column: Option<u32>,
    end_line: Option<u32>,
    end_column: Option<u32>,
) -> FailCase<'a> {
    FailCase {
        code,
        fixed,
        unfixable,
        warnings: vec![ExpectedWarning {
            message,
            line,
            column,
            end_line,
            end_column,
        }],
    }
}

pub fn reject_edit_warnings<'a>(
    code: &'a str,
    fixed: Option<&'a str>,
    unfixable: bool,
    warnings: Vec<ExpectedWarning<'a>>,
) -> FailCase<'a> {
    FailCase {
        code,
        fixed,
        unfixable,
        warnings,
    }
}

pub struct Tester<'a> {
    rule_name: &'static str,
    config: Option<Value>,
    syntax: Option<&'a str>,
    pass: Vec<&'a str>,
    fail: Vec<FailCase<'a>>,
}

impl<'a> Tester<'a> {
    pub fn new(
        rule_name: &'static str,
        config: Option<Value>,
        pass: Vec<&'a str>,
        fail: Vec<FailCase<'a>>,
    ) -> Self {
        Self {
            rule_name,
            config,
            syntax: None,
            pass,
            fail,
        }
    }

    pub fn with_syntax(mut self, syntax: &'a str) -> Self {
        self.syntax = Some(syntax);
        self
    }

    pub fn test(&self) {
        let mut cfg = Config::default();
        match &self.config {
            Some(val) => {
                let setting = Config::parse_rule_value(val.clone());
                cfg.rules.insert(self.rule_name.to_string(), setting);
            }
            None => {
                cfg.enable(self.rule_name);
            }
        }
        let active_rules = build_rules(&cfg);
        let syntax = resolve_syntax(self.syntax);
        for (idx, code) in self.pass.iter().enumerate() {
            let (warnings, _) = run_lint(code, syntax, &active_rules);
            if !warnings.is_empty() {
                panic!(
                    "[{}] Pass case #{} failed!\nCode:\n{}\nUnexpected warnings: {:#?}",
                    self.rule_name, idx, code, warnings
                );
            }
        }
        for (idx, case) in self.fail.iter().enumerate() {
            let (warnings, fixes) = run_lint(case.code, syntax, &active_rules);
            if warnings.len() != case.warnings.len() {
                panic!(
                    "[{}] Fail case #{} warning count mismatch!\nCode:\n{}\nExpected {} warning(s), got {}:\n{:#?}",
                    self.rule_name,
                    idx,
                    case.code,
                    case.warnings.len(),
                    warnings.len(),
                    warnings
                );
            }
            for (w_idx, (actual, expected)) in warnings.iter().zip(case.warnings.iter()).enumerate()
            {
                if let Some(exp_msg) = expected.message {
                    if !actual.text.starts_with(exp_msg) && actual.text != exp_msg {
                        panic!(
                            "[{}] Fail case #{} warning #{} message mismatch!\nCode:\n{}\nExpected: {:?}\nActual:   {:?}",
                            self.rule_name, idx, w_idx, case.code, exp_msg, actual.text
                        );
                    }
                }
                if let Some(exp_line) = expected.line {
                    assert_eq!(
                        actual.line, exp_line as usize,
                        "[{}] Fail case #{} warning #{} line mismatch!\nCode:\n{}",
                        self.rule_name, idx, w_idx, case.code
                    );
                }
                if let Some(exp_col) = expected.column {
                    assert_eq!(
                        actual.column, exp_col as usize,
                        "[{}] Fail case #{} warning #{} column mismatch!\nCode:\n{}",
                        self.rule_name, idx, w_idx, case.code
                    );
                }
                if let Some(exp_end_line) = expected.end_line {
                    assert_eq!(
                        actual.end_line, exp_end_line as usize,
                        "[{}] Fail case #{} warning #{} endLine mismatch!\nCode:\n{}",
                        self.rule_name, idx, w_idx, case.code
                    );
                }
                if let Some(exp_end_col) = expected.end_column {
                    assert_eq!(
                        actual.end_column, exp_end_col as usize,
                        "[{}] Fail case #{} warning #{} endColumn mismatch!\nCode:\n{}",
                        self.rule_name, idx, w_idx, case.code
                    );
                }
            }
            if case.unfixable {
                assert!(
                    fixes.is_empty(),
                    "[{}] Fail case #{} expected to be unfixable, but produced fixes: {:#?}\nCode:\n{}",
                    self.rule_name,
                    idx,
                    fixes,
                    case.code
                );
            } else if let Some(expected_fixed) = case.fixed {
                let (actual_fixed, applied) = fix::apply_fixes(case.code, fixes);
                assert!(
                    applied,
                    "[{}] Fail case #{} expected autofix {:?}, but rule produced no fixes!\nCode:\n{}",
                    self.rule_name, idx, expected_fixed, case.code
                );
                assert_eq!(
                    actual_fixed, expected_fixed,
                    "[{}] Fail case #{} autofix output mismatch!\nInput:\n{}\nExpected:\n{}\nActual:\n{}",
                    self.rule_name, idx, case.code, expected_fixed, actual_fixed
                );
            }
        }
    }
}

fn run_lint(
    source: &str,
    syntax: Syntax,
    active_rules: &[ConfiguredRule],
) -> (Vec<StylelintWarning>, Vec<fix::Fix>) {
    let allocator = Allocator::default();
    let mut parser = ParserBuilder::new(&allocator, source)
        .syntax(syntax)
        .comments()
        .build();
    if let Ok(stylesheet) = parser.parse::<Stylesheet>() {
        let comments = parser.comments();
        let line_index = LineIndex::new(source);
        let disables = DisableDirectives::parse(&comments, source, &line_index);
        let mut ctx = LintContext::new(source, &stylesheet, &comments, &line_index, &disables);
        SinglePassVisitor::new(&mut ctx, active_rules).visit_stylesheet(&stylesheet);
        let fixes = ctx.take_fixes();
        return (ctx.into_warnings(false), fixes);
    }

    if source.chars().any(|c| !c.is_ascii() && c.is_whitespace()) {
        let mut sanitized = String::with_capacity(source.len());
        for ch in source.chars() {
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
            let comments = parser2.comments();
            let line_index = LineIndex::new(source);
            let disables = DisableDirectives::parse(&comments, source, &line_index);
            let mut ctx = LintContext::new(source, &stylesheet, &comments, &line_index, &disables);
            SinglePassVisitor::new(&mut ctx, active_rules).visit_stylesheet(&stylesheet);
            let fixes = ctx.take_fixes();
            return (ctx.into_warnings(false), fixes);
        }
    }

    (Vec::new(), Vec::new())
}

fn resolve_syntax(syntax: Option<&str>) -> Syntax {
    match syntax {
        Some("scss") => Syntax::Scss,
        Some("less") => Syntax::Less,
        Some("sass" | "sugarss" | "sss") => Syntax::Sass,
        _ => Syntax::Css,
    }
}
