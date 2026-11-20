use std::path::Path;

use oxc_css_parser::Syntax;
use serde_json::Value;

use crate::config::Config;
use crate::diagnostic::StylelintWarning;
use crate::fix::apply_fixes;
use crate::lint_once;
use crate::rules::{ConfiguredRule, build_rules};

#[derive(Default)]
pub struct AcceptCase<'a> {
    pub code: &'a str,
    pub description: &'a str,
    pub skip: bool,
}

#[derive(Default)]
pub struct WarningExpectation<'a> {
    pub message: Option<&'a str>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

#[derive(Default)]
pub struct RejectCase<'a> {
    pub code: &'a str,
    pub fixed: Option<&'a str>,
    pub unfixable: bool,
    pub description: &'a str,
    pub skip: bool,
    pub message: Option<&'a str>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    pub warnings: Vec<WarningExpectation<'a>>,
}

pub struct TestRuleSuite<'a> {
    pub rule_name: &'a str,
    pub config: Value,
    pub custom_syntax: Option<&'a str>,
    pub fix: bool,
    pub skip: bool,
    pub accept: Vec<AcceptCase<'a>>,
    pub reject: Vec<RejectCase<'a>>,
}

fn resolve_syntax(custom_syntax: Option<&str>) -> Option<Syntax> {
    match custom_syntax {
        None => Some(Syntax::Css),
        Some("postcss-scss") => Some(Syntax::Scss),
        Some("postcss-sass") => Some(Syntax::Sass),
        Some("postcss-less") => Some(Syntax::Less),
        Some(_) => None,
    }
}

fn run_lint_case(
    source: &str,
    syntax: Syntax,
    active_rules: &[ConfiguredRule],
    fix: bool,
) -> (Vec<StylelintWarning>, String) {
    let (warnings, _) = lint_once(source, syntax, active_rules, false);
    if !fix {
        return (warnings, source.to_string());
    }
    let mut current = source.to_string();
    for _ in 0..5 {
        let (_, fixes) = lint_once(&current, syntax, active_rules, false);
        let (next, applied) = apply_fixes(&current, fixes);
        current = next;
        if !applied {
            break;
        }
    }
    (warnings, current)
}

pub fn run_test_rule_suite(suite: TestRuleSuite<'_>) {
    if suite.skip {
        return;
    }

    let Some(syntax) = resolve_syntax(suite.custom_syntax) else {
        panic!(
            "[{}] Unsupported customSyntax {:?} (mark suite with skip: true if intentional)",
            suite.rule_name, suite.custom_syntax
        );
    };

    let raw_cfg = serde_json::json!({
        "rules": {
            suite.rule_name: suite.config
        }
    });
    let cfg = Config::from_json_str(&raw_cfg.to_string(), Path::new("."))
        .expect("Failed to parse test rule config");
    let active_rules = build_rules(&cfg);

    for (idx, case) in suite.accept.iter().enumerate() {
        if case.skip {
            continue;
        }
        let (warnings, _) = run_lint_case(case.code, syntax, &active_rules, false);
        assert!(
            warnings.is_empty(),
            "[{}] accept case #{} ({:?}) expected 0 warnings for code:\n{}\nGot warnings: {:#?}",
            suite.rule_name,
            idx,
            case.description,
            case.code,
            warnings
        );
    }

    for (idx, case) in suite.reject.iter().enumerate() {
        if case.skip {
            continue;
        }
        let (warnings, fixed_output) = run_lint_case(case.code, syntax, &active_rules, suite.fix);

        let expected: Vec<WarningExpectation<'_>> = if !case.warnings.is_empty() {
            case.warnings
                .iter()
                .map(|w| WarningExpectation {
                    message: w.message,
                    line: w.line,
                    column: w.column,
                    end_line: w.end_line,
                    end_column: w.end_column,
                })
                .collect()
        } else {
            vec![WarningExpectation {
                message: case.message,
                line: case.line,
                column: w_col(case.column),
                end_line: case.end_line,
                end_column: case.end_column,
            }]
        };

        assert_eq!(
            warnings.len(),
            expected.len(),
            "[{}] reject case #{} ({:?}) warning count mismatch for code:\n{}\nExpected {} warning(s), got: {:#?}",
            suite.rule_name,
            idx,
            case.description,
            case.code,
            expected.len(),
            warnings
        );

        for (w_idx, (actual, exp)) in warnings.iter().zip(expected.iter()).enumerate() {
            assert_eq!(
                actual.rule, suite.rule_name,
                "[{}] reject case #{} warning #{}: rule name mismatch",
                suite.rule_name, idx, w_idx
            );

            if let Some(msg) = exp.message {
                let expected_full = if msg.ends_with(&format!("({})", suite.rule_name)) {
                    msg.to_string()
                } else {
                    format!("{} ({})", msg, suite.rule_name)
                };
                assert_eq!(
                    actual.text, expected_full,
                    "[{}] reject case #{} warning #{} message mismatch for code:\n{}",
                    suite.rule_name, idx, w_idx, case.code
                );
            }

            if let Some(line) = exp.line {
                assert_eq!(
                    actual.line, line,
                    "[{}] reject case #{} warning #{} line mismatch for code:\n{}",
                    suite.rule_name, idx, w_idx, case.code
                );
            }

            if let Some(column) = exp.column {
                assert_eq!(
                    actual.column, column,
                    "[{}] reject case #{} warning #{} column mismatch for code:\n{}",
                    suite.rule_name, idx, w_idx, case.code
                );
            }

            if let Some(end_line) = exp.end_line {
                assert_eq!(
                    actual.end_line, end_line,
                    "[{}] reject case #{} warning #{} endLine mismatch for code:\n{}",
                    suite.rule_name, idx, w_idx, case.code
                );
            }

            if let Some(end_column) = exp.end_column {
                assert_eq!(
                    actual.end_column, end_column,
                    "[{}] reject case #{} warning #{} endColumn mismatch for code:\n{}",
                    suite.rule_name, idx, w_idx, case.code
                );
            }
        }

        if suite.fix {
            if case.unfixable {
                assert_eq!(
                    fixed_output, case.code,
                    "[{}] reject case #{} expected unfixable code to remain unchanged",
                    suite.rule_name, idx
                );
            } else if let Some(expected_fixed) = case.fixed {
                assert_eq!(
                    fixed_output, expected_fixed,
                    "[{}] reject case #{} autofix mismatch for code:\n{}",
                    suite.rule_name, idx, case.code
                );
            }
        }
    }
}

#[inline]
fn w_col(col: Option<usize>) -> Option<usize> {
    col
}

#[macro_export]
macro_rules! accept_field {
    ($c:ident, code: $val:expr) => {
        $c.code = $val;
    };
    ($c:ident, description: $val:expr) => {
        $c.description = $val;
    };
    ($c:ident, skip: $val:expr) => {
        $c.skip = $val;
    };
}

#[macro_export]
macro_rules! warning_field {
    ($w:ident, message: $val:expr) => {
        $w.message = Some($val);
    };
    ($w:ident, line: $val:expr) => {
        $w.line = Some($val);
    };
    ($w:ident, column: $val:expr) => {
        $w.column = Some($val);
    };
    ($w:ident, endLine: $val:expr) => {
        $w.end_line = Some($val);
    };
    ($w:ident, endColumn: $val:expr) => {
        $w.end_column = Some($val);
    };
}

#[macro_export]
macro_rules! reject_field {
    ($c:ident, code: $val:expr) => {
        $c.code = $val;
    };
    ($c:ident, fixed: $val:expr) => {
        $c.fixed = Some($val);
    };
    ($c:ident, unfixable: $val:expr) => {
        $c.unfixable = $val;
    };
    ($c:ident, description: $val:expr) => {
        $c.description = $val;
    };
    ($c:ident, skip: $val:expr) => {
        $c.skip = $val;
    };
    ($c:ident, message: $val:expr) => {
        $c.message = Some($val);
    };
    ($c:ident, line: $val:expr) => {
        $c.line = Some($val);
    };
    ($c:ident, column: $val:expr) => {
        $c.column = Some($val);
    };
    ($c:ident, endLine: $val:expr) => {
        $c.end_line = Some($val);
    };
    ($c:ident, endColumn: $val:expr) => {
        $c.end_column = Some($val);
    };
    ($c:ident, warnings: [ $( { $( $wk:ident : $wv:expr ),* $(,)? } ),* $(,)? ]) => {
        $c.warnings = vec![
            $(
                {
                    let mut w = $crate::test::harness::WarningExpectation::default();
                    $( $crate::warning_field!(w, $wk : $wv); )*
                    w
                }
            ),*
        ];
    };
}

#[macro_export]
macro_rules! suite_field {
    ($s:ident, ruleName: $val:expr) => {
        $s.rule_name = $val;
    };
    ($s:ident, config: $val:tt) => {
        $s.config = serde_json::json!($val);
    };
    ($s:ident, customSyntax: $val:expr) => {
        $s.custom_syntax = Some($val);
    };
    ($s:ident, fix: $val:expr) => {
        $s.fix = $val;
    };
    ($s:ident, skip: $val:expr) => {
        $s.skip = $val;
    };
    ($s:ident, accept: [ $( { $( $ak:ident : $av:expr ),* $(,)? } ),* $(,)? ]) => {
        $s.accept = vec![
            $(
                {
                    let mut c = $crate::test::harness::AcceptCase::default();
                    $( $crate::accept_field!(c, $ak : $av); )*
                    c
                }
            ),*
        ];
    };
    ($s:ident, reject: [ $( { $( $rk:ident : $rv:tt ),* $(,)? } ),* $(,)? ]) => {
        $s.reject = vec![
            $(
                {
                    let mut c = $crate::test::harness::RejectCase::default();
                    $( $crate::reject_field!(c, $rk : $rv); )*
                    c
                }
            ),*
        ];
    };
}

#[macro_export]
macro_rules! test_rule {
    (
        name: $test_fn_name:ident,
        $( $k:ident : $v:tt ),* $(,)?
    ) => {
        #[test]
        fn $test_fn_name() {
            let mut suite = $crate::test::harness::TestRuleSuite {
                rule_name: "",
                config: serde_json::Value::Null,
                custom_syntax: None,
                fix: false,
                skip: false,
                accept: Vec::new(),
                reject: Vec::new(),
            };
            $( $crate::suite_field!(suite, $k : $v); )*
            $crate::test::harness::run_test_rule_suite(suite);
        }
    };
}
