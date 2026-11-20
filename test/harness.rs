use crate::config::{Config, RuleSetting};
use crate::diagnostic::Severity;
use crate::lint_once;
use crate::rules::build_rules;
use oxc_css_parser::Syntax;
use serde_json::Value;
use std::collections::HashMap;

pub struct AcceptCase {
    pub code: &'static str,
    pub description: Option<&'static str>,
    pub skip: bool,
}

#[derive(Clone, Debug)]
pub struct ExpectedEditInfo {
    pub range: (u32, u32),
    pub text: &'static str,
}

pub struct ExpectedWarning {
    pub message: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    pub fix: Option<ExpectedEditInfo>,
}

pub struct RejectCase {
    pub code: &'static str,
    pub fixed: Option<&'static str>,
    pub unfixable: bool,
    pub description: Option<&'static str>,
    pub skip: bool,
    pub message: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    pub fix: Option<ExpectedEditInfo>,
    pub warnings: Option<Vec<ExpectedWarning>>,
}

pub struct TestSuite {
    pub rule_name: &'static str,
    pub config: Value,
    pub custom_syntax: Option<&'static str>,
    pub fix: bool,
    pub compute_edit_info: bool,
    pub skip: bool,
    pub accept: Vec<AcceptCase>,
    pub reject: Vec<RejectCase>,
}

fn parse_rule_setting(val: &Value) -> Option<RuleSetting> {
    match val {
        Value::Null => None,
        Value::Bool(false) => None,
        Value::Array(arr) => {
            if arr.is_empty() || arr[0].is_null() || arr[0] == Value::Bool(false) {
                return None;
            }
            let primary = arr[0].clone();
            let mut secondary = None;
            if arr.len() > 1 {
                if let Some(obj) = arr[1].as_object() {
                    secondary = Some(obj.clone());
                }
            }
            Some(RuleSetting {
                primary,
                secondary,
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

pub fn run_test_suite(suite: TestSuite) {
    if suite.skip {
        return;
    }

    let Some(syntax) = resolve_syntax(suite.custom_syntax) else {
        panic!(
            "[{}] Unsupported customSyntax {:?} (mark suite with `skip: true` if intentional)",
            suite.rule_name, suite.custom_syntax
        );
    };

    let setting = parse_rule_setting(&suite.config);
    let mut rules_map = HashMap::new();
    rules_map.insert(suite.rule_name.to_string(), setting);

    let config = Config {
        rules: rules_map,
        default_severity: Severity::Error,
    };

    let active_rules = build_rules(&config);

    for (idx, case) in suite.accept.iter().enumerate() {
        if case.skip {
            continue;
        }
        let (warnings, _fixes) = lint_once(case.code, syntax, &active_rules, false);
        assert!(
            warnings.is_empty(),
            "[{}] accept case #{} ({:?}) produced unexpected warnings for code:\n{}\nWarnings: {:?}",
            suite.rule_name,
            idx,
            case.description.unwrap_or(""),
            case.code,
            warnings
        );
    }

    for (idx, case) in suite.reject.iter().enumerate() {
        if case.skip {
            continue;
        }
        let (warnings, fixes) = lint_once(case.code, syntax, &active_rules, false);

        let expected_list: Vec<ExpectedWarning> = if let Some(ref ws) = case.warnings {
            ws.iter()
                .map(|w| ExpectedWarning {
                    message: w.message.clone(),
                    line: w.line,
                    column: w.column,
                    end_line: w.end_line,
                    end_column: w.end_column,
                    fix: w.fix.clone(),
                })
                .collect()
        } else {
            vec![ExpectedWarning {
                message: case.message.clone(),
                line: case.line,
                column: case.column,
                end_line: case.end_line,
                end_column: case.end_column,
                fix: case.fix.clone(),
            }]
        };

        assert_eq!(
            warnings.len(),
            expected_list.len(),
            "[{}] reject case #{} ({:?}) expected {} warning(s), got {} for code:\n{}\nActual: {:?}",
            suite.rule_name,
            idx,
            case.description.unwrap_or(""),
            expected_list.len(),
            warnings.len(),
            case.code,
            warnings
        );

        for (w_idx, (actual, expected)) in warnings.iter().zip(expected_list.iter()).enumerate() {
            if let Some(ref exp_msg) = expected.message {
                let expected_formatted = format!("{} ({})", exp_msg, suite.rule_name);
                assert!(
                    actual.text == *exp_msg || actual.text == expected_formatted,
                    "[{}] reject case #{} warning #{} message mismatch for code:\n{}\nExpected: {:?}\nActual:   {:?}",
                    suite.rule_name,
                    idx,
                    w_idx,
                    case.code,
                    expected_formatted,
                    actual.text
                );
            }
            if let Some(exp_line) = expected.line {
                assert_eq!(
                    actual.line, exp_line,
                    "[{}] reject case #{} warning #{} line mismatch for code:\n{}",
                    suite.rule_name, idx, w_idx, case.code
                );
            }
            if let Some(exp_col) = expected.column {
                assert_eq!(
                    actual.column, exp_col,
                    "[{}] reject case #{} warning #{} column mismatch for code:\n{}",
                    suite.rule_name, idx, w_idx, case.code
                );
            }
            if let Some(exp_end_line) = expected.end_line {
                assert_eq!(
                    actual.end_line, exp_end_line,
                    "[{}] reject case #{} warning #{} endLine mismatch for code:\n{}",
                    suite.rule_name, idx, w_idx, case.code
                );
            }
            if let Some(exp_end_col) = expected.end_column {
                assert_eq!(
                    actual.end_column, exp_end_col,
                    "[{}] reject case #{} warning #{} endColumn mismatch for code:\n{}",
                    suite.rule_name, idx, w_idx, case.code
                );
            }
        }

        if suite.fix {
            if case.unfixable {
                assert!(
                    fixes.is_empty(),
                    "[{}] reject case #{} marked unfixable, but produced fixes for code:\n{}",
                    suite.rule_name,
                    idx,
                    case.code
                );
            } else if let Some(exp_fixed) = case.fixed {
                let mut current = case.code.to_string();
                for _ in 0..10 {
                    let (_w, pass_fixes) = lint_once(&current, syntax, &active_rules, false);
                    if pass_fixes.is_empty() {
                        break;
                    }
                    let (updated, changed) = crate::fix::apply_fixes(&current, pass_fixes);
                    current = updated;
                    if !changed {
                        break;
                    }
                }
                assert_eq!(
                    current, exp_fixed,
                    "[{}] reject case #{} autofix mismatch for code:\n{}",
                    suite.rule_name, idx, case.code
                );
            }
        }
    }
}

#[macro_export]
macro_rules! test_rule {
    (
        $(name: $test_name:ident,)?
        ruleName: $rule_name:expr,
        config: $config:tt,
        $(customSyntax: $syntax:expr,)?
        $(fix: $fix:expr,)?
        $(computeEditInfo: $compute_edit_info:expr,)?
        $(skip: $skip:expr,)?
        $(accept: [
            $(
                { $($a_tokens:tt)* }
            ),* $(,)?
        ] $(,)? )?
        $(reject: [
            $(
                { $($r_tokens:tt)* }
            ),* $(,)?
        ] $(,)? )?
    ) => {
        test_rule!(@impl
            name: $( $test_name )?,
            ruleName: $rule_name,
            config: $config,
            customSyntax: $( $syntax )?,
            fix: $( $fix )?,
            computeEditInfo: $( $compute_edit_info )?,
            skip: $( $skip )?,
            accept: [ $( $( { $($a_tokens)* } ),* )? ],
            reject: [ $( $( { $($r_tokens)* } ),* )? ],
        );
    };

    (@impl
        name: ,
        ruleName: $rule_name:expr,
        config: $config:tt,
        customSyntax: $($syntax:expr)?,
        fix: $($fix:expr)?,
        computeEditInfo: $($compute_edit_info:expr)?,
        skip: $($skip:expr)?,
        accept: [ $( { $($a_tokens:tt)* } ),* ],
        reject: [ $( { $($r_tokens:tt)* } ),* ],
    ) => {
        test_rule!(@build_fn
            suite_default,
            $rule_name,
            $config,
            ($($syntax)?),
            ($($fix)?),
            ($($compute_edit_info)?),
            ($($skip)?),
            [ $( { $($a_tokens)* } ),* ],
            [ $( { $($r_tokens)* } ),* ]
        );
    };

    (@impl
        name: $test_name:ident,
        ruleName: $rule_name:expr,
        config: $config:tt,
        customSyntax: $($syntax:expr)?,
        fix: $($fix:expr)?,
        computeEditInfo: $($compute_edit_info:expr)?,
        skip: $($skip:expr)?,
        accept: [ $( { $($a_tokens:tt)* } ),* ],
        reject: [ $( { $($r_tokens:tt)* } ),* ],
    ) => {
        test_rule!(@build_fn
            $test_name,
            $rule_name,
            $config,
            ($($syntax)?),
            ($($fix)?),
            ($($compute_edit_info)?),
            ($($skip)?),
            [ $( { $($a_tokens)* } ),* ],
            [ $( { $($r_tokens)* } ),* ]
        );
    };

    (@build_fn
        $fn_name:ident,
        $rule_name:expr,
        $config:tt,
        ($($syntax:expr)?),
        ($($fix:expr)?),
        ($($compute_edit_info:expr)?),
        ($($skip:expr)?),
        [ $( { $($a_tokens:tt)* } ),* ],
        [ $( { $($r_tokens:tt)* } ),* ]
    ) => {
        #[test]
        #[allow(unused_mut, unused_assignments)]
        fn $fn_name() {
            let mut custom_syntax: Option<&'static str> = None;
            $( custom_syntax = Some($syntax); )?

            let mut fix: bool = false;
            $( fix = $fix; )?

            let mut compute_edit_info: bool = false;
            $( compute_edit_info = $compute_edit_info; )?

            let mut skip: bool = false;
            $( skip = $skip; )?

            let mut accept_vec: Vec<$crate::test::harness::AcceptCase> = Vec::new();
            $(
                {
                    let mut c = $crate::test::harness::AcceptCase {
                        code: "",
                        description: None,
                        skip: false,
                    };
                    test_rule!(@accept_field c, $($a_tokens)*);
                    accept_vec.push(c);
                }
            )*

            let mut reject_vec: Vec<$crate::test::harness::RejectCase> = Vec::new();
            $(
                {
                    let mut c = $crate::test::harness::RejectCase {
                        code: "",
                        fixed: None,
                        unfixable: false,
                        description: None,
                        skip: false,
                        message: None,
                        line: None,
                        column: None,
                        end_line: None,
                        end_column: None,
                        fix: None,
                        warnings: None,
                    };
                    test_rule!(@reject_field c, $($r_tokens)*);
                    reject_vec.push(c);
                }
            )*

            let suite = $crate::test::harness::TestSuite {
                rule_name: $rule_name,
                config: serde_json::json!($config),
                custom_syntax,
                fix,
                compute_edit_info,
                skip,
                accept: accept_vec,
                reject: reject_vec,
            };

            $crate::test::harness::run_test_suite(suite);
        }
    };

    (@accept_field $c:ident, code: $val:expr $(, $($rest:tt)* )?) => {
        $c.code = $val;
        test_rule!(@accept_field $c $(, $($rest)* )?);
    };
    (@accept_field $c:ident, description: $val:expr $(, $($rest:tt)* )?) => {
        $c.description = Some($val);
        test_rule!(@accept_field $c $(, $($rest)* )?);
    };
    (@accept_field $c:ident, skip: $val:expr $(, $($rest:tt)* )?) => {
        $c.skip = $val;
        test_rule!(@accept_field $c $(, $($rest)* )?);
    };
    (@accept_field $c:ident $(,)?) => {};

    (@reject_field $c:ident, warnings: [ $( { $($w_tokens:tt)* } ),* $(,)? ] $(, $($rest:tt)* )?) => {
        let mut ws = Vec::new();
        $(
            {
                let mut w = $crate::test::harness::ExpectedWarning {
                    message: None,
                    line: None,
                    column: None,
                    end_line: None,
                    end_column: None,
                    fix: None,
                };
                test_rule!(@warning_fields w, $($w_tokens)*);
                ws.push(w);
            }
        )*
        $c.warnings = Some(ws);
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, fix: { range: [$start:expr, $end:expr], text: $text:expr $(,)? } $(, $($rest:tt)* )?) => {
        $c.fix = Some($crate::test::harness::ExpectedEditInfo {
            range: ($start, $end),
            text: $text,
        });
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, code: $val:expr $(, $($rest:tt)* )?) => {
        $c.code = $val;
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, fixed: $val:expr $(, $($rest:tt)* )?) => {
        $c.fixed = Some($val);
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, unfixable: $val:expr $(, $($rest:tt)* )?) => {
        $c.unfixable = $val;
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, description: $val:expr $(, $($rest:tt)* )?) => {
        $c.description = Some($val);
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, skip: $val:expr $(, $($rest:tt)* )?) => {
        $c.skip = $val;
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, message: $val:expr $(, $($rest:tt)* )?) => {
        $c.message = Some(($val).into());
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, line: $val:expr $(, $($rest:tt)* )?) => {
        $c.line = Some($val);
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, column: $val:expr $(, $($rest:tt)* )?) => {
        $c.column = Some($val);
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, endLine: $val:expr $(, $($rest:tt)* )?) => {
        $c.end_line = Some($val);
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident, endColumn: $val:expr $(, $($rest:tt)* )?) => {
        $c.end_column = Some($val);
        test_rule!(@reject_field $c $(, $($rest)* )?);
    };
    (@reject_field $c:ident $(,)?) => {};

    (@warning_fields $w:ident, fix: { range: [$start:expr, $end:expr], text: $text:expr $(,)? } $(, $($rest:tt)* )?) => {
        $w.fix = Some($crate::test::harness::ExpectedEditInfo {
            range: ($start, $end),
            text: $text,
        });
        test_rule!(@warning_fields $w $(, $($rest)* )?);
    };
    (@warning_fields $w:ident, message: $val:expr $(, $($rest:tt)* )?) => {
        $w.message = Some(($val).into());
        test_rule!(@warning_fields $w $(, $($rest)* )?);
    };
    (@warning_fields $w:ident, line: $val:expr $(, $($rest:tt)* )?) => {
        $w.line = Some($val);
        test_rule!(@warning_fields $w $(, $($rest)* )?);
    };
    (@warning_fields $w:ident, column: $val:expr $(, $($rest:tt)* )?) => {
        $w.column = Some($val);
        test_rule!(@warning_fields $w $(, $($rest)* )?);
    };
    (@warning_fields $w:ident, endLine: $val:expr $(, $($rest:tt)* )?) => {
        $w.end_line = Some($val);
        test_rule!(@warning_fields $w $(, $($rest)* )?);
    };
    (@warning_fields $w:ident, endColumn: $val:expr $(, $($rest:tt)* )?) => {
        $w.end_column = Some($val);
        test_rule!(@warning_fields $w $(, $($rest)* )?);
    };
    (@warning_fields $w:ident $(,)?) => {};
}
