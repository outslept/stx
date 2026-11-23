//! Unit-testing harness for individual lint rules (`oxlint`-style `Tester`).

use std::collections::HashMap;

use oxc_css_parser::Syntax;
use serde_json::Value;

use crate::{
    config::{Config, RuleSetting},
    diagnostic::Severity,
    fix::apply_fixes,
    lint_once,
    rules::build_rules,
};

/// Expected warning assertion for a single diagnostic emitted by a rule.
#[derive(Debug, Clone)]
pub struct ExpectedWarning {
    pub message: String,
    /// 1-based `(line, column, end_line, end_column)`.
    pub span: Option<(usize, usize, usize, usize)>,
    /// Optional expected `EditInfo`: `(range_start, range_end, replacement_text)`.
    pub edit_info: Option<(u32, u32, &'static str)>,
}

/// A test case that is expected to produce one or more lint warnings.
#[derive(Debug, Clone)]
pub struct FailCase<'a> {
    pub code: &'a str,
    pub fixed: Option<&'a str>,
    pub unfixable: bool,
    pub warnings: Vec<ExpectedWarning>,
}

/// Constructs a [`FailCase`] expecting a single warning with `(line, col, end_line, end_col)`.
pub fn reject<'a>(
    code: &'a str,
    fixed: impl Into<Option<&'a str>>,
    message: impl Into<String>,
    span: (usize, usize, usize, usize),
) -> FailCase<'a> {
    let fixed = fixed.into();
    FailCase {
        code,
        fixed,
        unfixable: fixed.is_none(),
        warnings: vec![ExpectedWarning {
            message: message.into(),
            span: Some(span),
            edit_info: None,
        }],
    }
}

/// Constructs a [`FailCase`] expecting multiple warnings.
pub fn reject_warnings<'a>(
    code: &'a str,
    fixed: impl Into<Option<&'a str>>,
    warnings: Vec<(String, (usize, usize, usize, usize))>,
) -> FailCase<'a> {
    let fixed = fixed.into();
    FailCase {
        code,
        fixed,
        unfixable: fixed.is_none(),
        warnings: warnings
            .into_iter()
            .map(|(message, span)| ExpectedWarning {
                message,
                span: Some(span),
                edit_info: None,
            })
            .collect(),
    }
}

/// Constructs a [`FailCase`] expecting a single warning with `computeEditInfo` data.
pub fn reject_edit<'a>(
    code: &'a str,
    message: impl Into<String>,
    span: (usize, usize, usize, usize),
    edit_info: (u32, u32, &'static str),
) -> FailCase<'a> {
    FailCase {
        code,
        fixed: None,
        unfixable: false,
        warnings: vec![ExpectedWarning {
            message: message.into(),
            span: Some(span),
            edit_info: Some(edit_info),
        }],
    }
}

/// Constructs a [`FailCase`] expecting multiple warnings with optional `computeEditInfo` data.
pub fn reject_edit_warnings<'a, E>(
    code: &'a str,
    warnings: Vec<(String, (usize, usize, usize, usize), E)>,
) -> FailCase<'a>
where
    E: Into<Option<(u32, u32, &'static str)>>,
{
    FailCase {
        code,
        fixed: None,
        unfixable: false,
        warnings: warnings
            .into_iter()
            .map(|(message, span, edit_info)| ExpectedWarning {
                message,
                span: Some(span),
                edit_info: edit_info.into(),
            })
            .collect(),
    }
}

/// Rule test runner inspired by `oxc_linter::Tester`.
pub struct Tester<'a> {
    rule_name: &'static str,
    config: Value,
    pass: &'a [&'a str],
    fail: &'a [FailCase<'a>],
    custom_syntax: Option<&'static str>,
    fix: bool,
    compute_edit_info: bool,
}

impl<'a> Tester<'a> {
    pub fn new(
        rule_name: &'static str,
        config: Value,
        pass: &'a [&'a str],
        fail: &'a [FailCase<'a>],
    ) -> Self {
        Self {
            rule_name,
            config,
            pass,
            fail,
            custom_syntax: None,
            fix: false,
            compute_edit_info: false,
        }
    }

    pub fn with_custom_syntax(mut self, syntax: &'static str) -> Self {
        self.custom_syntax = Some(syntax);
        self
    }

    pub fn with_fix(mut self, fix: bool) -> Self {
        self.fix = fix;
        self
    }

    pub fn with_compute_edit_info(mut self, compute_edit_info: bool) -> Self {
        self.compute_edit_info = compute_edit_info;
        self
    }

    pub fn test(self) {
        let Some(syntax) = resolve_syntax(self.custom_syntax) else {
            panic!(
                "[{}] Unsupported customSyntax {:?}",
                self.rule_name, self.custom_syntax
            );
        };

        let setting = parse_rule_setting(&self.config);
        let mut rules_map = HashMap::new();
        rules_map.insert(self.rule_name.to_string(), setting);
        let config = Config {
            rules: rules_map,
            default_severity: Severity::Error,
        };
        let active_rules = build_rules(&config);

        for (idx, &code) in self.pass.iter().enumerate() {
            let (warnings, _fixes) = lint_once(code, syntax, &active_rules, false);
            assert!(
                warnings.is_empty(),
                "[{}] pass case #{idx} failed!\nCode:\n{code}\nUnexpected warnings:\n{warnings:#?}",
                self.rule_name
            );
        }

        for (idx, case) in self.fail.iter().enumerate() {
            let (warnings, fixes) = lint_once(case.code, syntax, &active_rules, false);

            assert_eq!(
                warnings.len(),
                case.warnings.len(),
                "[{}] fail case #{idx} warning count mismatch!\nCode:\n{}\nExpected {} warning(s), got {}:\n{warnings:#?}",
                self.rule_name,
                case.code,
                case.warnings.len(),
                warnings.len()
            );

            for (w_idx, (actual, expected)) in warnings.iter().zip(case.warnings.iter()).enumerate()
            {
                let expected_with_rule = format!("{} ({})", expected.message, self.rule_name);
                assert!(
                    actual.text == expected.message || actual.text == expected_with_rule,
                    "[{}] fail case #{idx} warning #{w_idx} message mismatch!\nCode:\n{}\nExpected: {:?}\nActual:   {:?}",
                    self.rule_name,
                    case.code,
                    expected_with_rule,
                    actual.text
                );

                if let Some((line, col, end_line, end_col)) = expected.span {
                    assert_eq!(
                        (
                            actual.line,
                            actual.column,
                            actual.end_line,
                            actual.end_column
                        ),
                        (line, col, end_line, end_col),
                        "[{}] fail case #{idx} warning #{w_idx} span mismatch!\nCode:\n{}",
                        self.rule_name,
                        case.code
                    );
                }
            }

            if self.compute_edit_info {
                let expected_edits = case
                    .warnings
                    .iter()
                    .filter(|w| w.edit_info.is_some())
                    .count();
                assert_eq!(
                    fixes.len(),
                    expected_edits,
                    "[{}] fail case #{idx} expected {expected_edits} edit(s), got {}!\nCode:\n{}",
                    self.rule_name,
                    fixes.len(),
                    case.code
                );
            }

            if self.fix {
                if case.unfixable {
                    assert!(
                        fixes.is_empty(),
                        "[{}] fail case #{idx} marked unfixable, but produced fixes!\nCode:\n{}",
                        self.rule_name,
                        case.code
                    );
                } else if let Some(expected_fixed) = case.fixed {
                    let mut current = case.code.to_string();
                    for _ in 0..10 {
                        let (_w, pass_fixes) = lint_once(&current, syntax, &active_rules, false);
                        if pass_fixes.is_empty() {
                            break;
                        }
                        let (updated, changed) = apply_fixes(&current, pass_fixes);
                        current = updated;
                        if !changed {
                            break;
                        }
                    }
                    assert_eq!(
                        current, expected_fixed,
                        "[{}] fail case #{idx} autofix mismatch!\nOriginal:\n{}",
                        self.rule_name, case.code
                    );
                }
            }
        }
    }
}

fn parse_rule_setting(val: &Value) -> Option<RuleSetting> {
    match val {
        Value::Null | Value::Bool(false) => None,
        Value::Array(arr) => {
            if arr.is_empty() || arr[0].is_null() || arr[0] == Value::Bool(false) {
                return None;
            }
            if arr.len() == 1 {
                return Some(RuleSetting {
                    primary: arr[0].clone(),
                    secondary: None,
                    severity: Severity::Error,
                });
            }
            if let Some(obj) = arr[1].as_object() {
                return Some(RuleSetting {
                    primary: arr[0].clone(),
                    secondary: Some(obj.clone()),
                    severity: Severity::Error,
                });
            }
            Some(RuleSetting {
                primary: Value::Array(arr.clone()),
                secondary: None,
                severity: Severity::Error,
            })
        }
        other => Some(RuleSetting {
            primary: other.clone(),
            secondary: None,
            severity: Severity::Error,
        }),
    }
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
