//! Unit-testing harness for individual lint rules.

use std::collections::HashMap;

use oxc_css_parser::Syntax;
use serde_json::Value;

use crate::{
    config::Config,
    fix::{Fix, apply_fixes},
    lint_and_fix, lint_once,
    rules::build_rules,
};

type Position = (usize, usize);

/// Expected warning assertion for a single diagnostic emitted by a rule.
#[derive(Debug, Clone)]
pub struct ExpectedWarning {
    pub message: String,
    /// 1-based start and optional end positions.
    pub span: Option<(Position, Option<Position>)>,
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

impl FailCase<'_> {
    pub fn with_edit_info(
        mut self,
        warning_index: usize,
        edit_info: (u32, u32, &'static str),
    ) -> Self {
        self.warnings[warning_index].edit_info = Some(edit_info);
        self.unfixable = false;
        self
    }
}

/// Constructs a [`FailCase`] expecting a single warning with `(line, col, end_line, end_col)`.
pub fn reject<'a>(
    code: &'a str,
    fixed: impl Into<Option<&'a str>>,
    message: impl Into<String>,
    (line, col, end_line, end_col): (usize, usize, usize, usize),
) -> FailCase<'a> {
    let mut case = reject_message(code, fixed, message);
    case.warnings[0].span = Some(((line, col), Some((end_line, end_col))));
    case
}

pub fn reject_start<'a>(
    code: &'a str,
    fixed: impl Into<Option<&'a str>>,
    message: impl Into<String>,
    start: (usize, usize),
) -> FailCase<'a> {
    let mut case = reject_message(code, fixed, message);
    case.warnings[0].span = Some((start, None));
    case
}

/// Constructs a [`FailCase`] expecting a single warning without asserting its span.
pub fn reject_message<'a>(
    code: &'a str,
    fixed: impl Into<Option<&'a str>>,
    message: impl Into<String>,
) -> FailCase<'a> {
    let fixed = fixed.into();
    FailCase {
        code,
        fixed,
        unfixable: fixed.is_none(),
        warnings: vec![ExpectedWarning {
            message: message.into(),
            span: None,
            edit_info: None,
        }],
    }
}

/// Constructs a [`FailCase`] expecting multiple warnings without asserting spans.
pub fn reject_messages<'a>(
    code: &'a str,
    fixed: impl Into<Option<&'a str>>,
    messages: Vec<String>,
) -> FailCase<'a> {
    let fixed = fixed.into();
    FailCase {
        code,
        fixed,
        unfixable: fixed.is_none(),
        warnings: messages
            .into_iter()
            .map(|message| ExpectedWarning {
                message,
                span: None,
                edit_info: None,
            })
            .collect(),
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
            .map(
                |(message, (line, col, end_line, end_col))| ExpectedWarning {
                    message,
                    span: Some(((line, col), Some((end_line, end_col)))),
                    edit_info: None,
                },
            )
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
    reject(code, None, message, span).with_edit_info(0, edit_info)
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
            .map(
                |(message, (line, col, end_line, end_col), edit_info)| ExpectedWarning {
                    message,
                    span: Some(((line, col), Some((end_line, end_col)))),
                    edit_info: edit_info.into(),
                },
            )
            .collect(),
    }
}

/// Rule test runner inspired by `oxc_linter::Tester`.
pub struct Tester<'a> {
    rule_name: &'static str,
    config: Option<Value>,
    pass: Vec<&'a str>,
    fail: Vec<FailCase<'a>>,
    custom_syntax: Option<&'static str>,
    fix: bool,
    compute_edit_info: bool,
}

impl<'a> Tester<'a> {
    pub fn new(
        rule_name: &'static str,
        config: impl Into<Option<Value>>,
        pass: impl AsRef<[&'a str]>,
        fail: impl AsRef<[FailCase<'a>]>,
    ) -> Self {
        Self {
            rule_name,
            config: config.into(),
            pass: pass.as_ref().to_vec(),
            fail: fail.as_ref().to_vec(),
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

        let setting = self.config.and_then(|value| {
            Config::parse_rule_value(self.rule_name, value, crate::diagnostic::Severity::Error)
        });
        let mut rules_map = HashMap::new();
        rules_map.insert(self.rule_name.to_string(), setting);
        let config = Config {
            rules: rules_map,
            ..Config::default()
        };
        let active_rules = build_rules(&config);

        let name = self.rule_name;
        let mut errors: Vec<String> = Vec::new();
        let parse_check = |errors: &mut Vec<String>, what: &str, code: &str, err: &Option<_>| {
            let known = KNOWN_PARSE_FAILURES.contains(&(syntax, code));
            match (err, known) {
                (Some(err), false) => errors.push(format!(
                    "{what} failed to parse ({syntax:?})!\nCode:\n{code}\n{err:?}"
                )),
                (None, true) => errors.push(format!(
                    "{what} now parses; remove it from KNOWN_PARSE_FAILURES:\n{code:?}"
                )),
                _ => {}
            }
        };

        for (idx, &code) in self.pass.iter().enumerate() {
            let what = format!("pass case #{idx}");
            let pass = lint_once(code, syntax, &active_rules, false);
            parse_check(&mut errors, &what, code, &pass.parse_error);
            if !pass.warnings.is_empty() {
                errors.push(format!(
                    "{what} failed!\nCode:\n{code}\nUnexpected warnings:\n{:#?}",
                    pass.warnings
                ));
            }
            if self.fix {
                let (fixed, _) = lint_and_fix(code, syntax, &active_rules, false);
                if fixed != code {
                    errors.push(format!(
                        "{what} was changed by autofix!\nCode:\n{code}\nFixed:\n{fixed}"
                    ));
                }
            }
        }

        for (idx, case) in self.fail.iter().enumerate() {
            let code = case.code;
            let what = format!("fail case #{idx}");
            let pass = lint_once(code, syntax, &active_rules, false);
            parse_check(&mut errors, &what, code, &pass.parse_error);
            let parsed = pass.parse_error.is_none();
            let warnings = pass.warnings;

            if warnings.len() != case.warnings.len() {
                errors.push(format!(
                    "{what} warning count mismatch!\nCode:\n{code}\nExpected {} warning(s), got {}:\n{warnings:#?}",
                    case.warnings.len(),
                    warnings.len()
                ));
            }

            let mut edit_ranges: Vec<(u32, u32)> = Vec::new();
            for (w_idx, (actual, expected)) in warnings.iter().zip(case.warnings.iter()).enumerate()
            {
                let what = format!("{what} warning #{w_idx}");
                let expected_with_rule = format!("{} ({name})", expected.message);
                if actual.text != expected.message && actual.text != expected_with_rule {
                    errors.push(format!(
                        "{what} message mismatch!\nCode:\n{code}\nExpected: {expected_with_rule:?}\nActual:   {:?}",
                        actual.text
                    ));
                }

                if let Some((start, end)) = expected.span {
                    let got = (
                        (actual.line, actual.column),
                        end.map(|_| (actual.end_line, actual.end_column)),
                    );
                    if got != (start, end) {
                        errors.push(format!(
                            "{what} span mismatch!\nCode:\n{code}\nExpected: {:?}\nActual:   {got:?}",
                            (start, end)
                        ));
                    }
                }

                if self.compute_edit_info {
                    let edit = actual.fix.as_ref().map(|fix| edit_info(code, fix)).filter(
                        |&(start, end, _)| !edit_ranges.iter().any(|&(s, e)| start < e && s < end),
                    );
                    if let Some((start, end, _)) = edit {
                        edit_ranges.push((start, end));
                    }
                    let want = expected.edit_info.map(|(s, e, t)| (s, e, t.to_string()));
                    if edit != want {
                        errors.push(format!(
                            "{what} EditInfo mismatch!\nCode:\n{code}\nExpected: {want:?}\nActual:   {edit:?}"
                        ));
                    }
                }
            }

            if self.fix {
                let (fixed, after) = lint_and_fix(code, syntax, &active_rules, false);
                if parsed && after.parse_error.is_some() {
                    errors.push(format!(
                        "{what} autofix produced unparsable code!\nFixed:\n{fixed}"
                    ));
                }
                if case.unfixable {
                    if fixed != code {
                        errors.push(format!(
                            "{what} is unfixable but was changed!\nCode:\n{code}\nFixed:\n{fixed}"
                        ));
                    }
                } else if let Some(expected_fixed) = case.fixed {
                    if fixed != expected_fixed {
                        errors.push(format!(
                            "{what} autofix mismatch!\nCode:\n{code}\nExpected:\n{expected_fixed}\nActual:\n{fixed}"
                        ));
                    } else if fixed == code {
                        errors.push(format!(
                            "{what} expected a fix, code is unchanged!\nCode:\n{code}"
                        ));
                    }
                }
            }
        }

        assert!(
            errors.is_empty(),
            "[{name}] {} problem(s):\n\n{}",
            errors.len(),
            errors.join("\n\n")
        );
    }
}

/// Inputs that Stylelint (postcss) accepts but oxc-css-parser 0.0.22 rejects.
/// Rules still run on them (raw-text scanning), so their expectations stay valid.
// shortcut: per-input allow-list; shrink it as the parser improves (the Tester fails on stale entries).
const KNOWN_PARSE_FAILURES: &[(Syntax, &str)] = &[
    // `!important` is expected
    (Syntax::Scss, "a { color: #{$green} !imprtant; }"),
    // expect token `<string>`, but found `<ident>`
    (Syntax::Css, "@charset utf-8;"),
    // expect token `;`, but found `{`
    (Syntax::Css, "@charset \"utf-8\" {}"),
    // expect token `;`, but found `<ident>`
    (Syntax::Css, "@charset \"utf-8\" extra;"),
    // expect token `<string>`, but found `;`
    (Syntax::Css, "@charset;"),
    // expect token `'('`, but found `{`
    (Syntax::Css, "@supports {\n  @media {}\n}"),
    // expect token `'('`, but found `{`
    (Syntax::Css, "@supports {\n\n  @media {}\n}"),
    // expect token `'('`, but found `{`
    (Syntax::Css, "@supports {\n  @media {}\n  @media{}\n}"),
    // expect token `{`, but found `,`
    (Syntax::Css, "@property --foo, --bar {}"),
    // expect token `;`, but found `<string>`
    (Syntax::Css, "@layer \"foo\";"),
    // expect token `{`, but found `.`
    (Syntax::Css, "@scope .foo {}"),
    // expect token `<ident>`, but found `<hash>`
    (Syntax::Css, "@font-feature-values #foo {}"),
    // expect token `{`, but found `;`
    (Syntax::Css, "@property foo;"),
    // expect token `{`, but found `<eof>`
    (Syntax::Css, "@container (min-width: 700px)"),
    // expect token `{`, but found `<eof>`
    (Syntax::Css, "@CONTAINER (min-width: 500px)"),
    // expect token `:`, but found `#`
    (Syntax::Scss, "a { border-#$side: 0; }"),
    // component value is expected
    (Syntax::Scss, "a { box-sizing: #$type-box; }"),
    // expect token `{`, but found `<ident>`
    (Syntax::Css, "@container foo-bar style(--cards small) {}"),
    // expect token `{`, but found `(`
    (
        Syntax::Css,
        "@container foo-bar (inline-size > 30em) and foo-baz style(--large: true) {}",
    ),
    // expect token `<ident>`, but found `<eof>`
    (Syntax::Css, "@custom-media --Ape-ageLess"),
    // expect token `<ident>`, but found `<eof>`
    (Syntax::Css, "@custom-media --Purr-piratePlant"),
    // expect token `<ident>`, but found `<eof>`
    (Syntax::Css, "@custom-media --ape-ageLess"),
    // expect token `<ident>`, but found `<eof>`
    (Syntax::Css, "@custom-media --Ape-AgeLess"),
    // expect token `{`, but found `,`
    (Syntax::Css, "@property --foo, --Bar {}"),
    // expect token `;`, but found `)`
    (
        Syntax::Css,
        ":root { --foo-color: #f00; } a { color: VAR(--boo-color)); }",
    ),
    // expect token `;`, but found `)`
    (
        Syntax::Css,
        ":root { --foo-color: #f00; } a { color: var(--boo-color), var(--boo-sub-color)); }",
    ),
    // expect token `{`, but found `,`
    (Syntax::Css, "@property --foo, --bar {} a { color: --bar; }"),
    // expect token `<ident>`, but found `<at-keyword>`
    (Syntax::Css, "@media @foo {}"),
    // expect token `{`, but found `<ident>`
    (Syntax::Css, "@media screen or (min-width > 500px) {}"),
    // expect token `{`, but found `<ident>`
    (Syntax::Css, "@media (color) and (hover) or (width) {}"),
    // expect token `{`, but found `<ident>`
    (Syntax::Css, "@media foo screen {}"),
    // expect token `{`, but found `<ident>`
    (Syntax::Css, "@media not screen foo (min-width: 300px) {}"),
    // expect token `{`, but found `<eof>`
    (Syntax::Css, "@font-feature-values \"foo\u{a0}bar\""),
    // expect token `{`, but found `<eof>`
    (Syntax::Css, ":lang(\"en\u{a0}us \")"),
    // expect token `{`, but found `<eof>`
    (Syntax::Css, ":not([data-id=\"a\u{a0}b\"])"),
    // expect token `{`, but found `<eof>`
    (Syntax::Css, ":is(.class[attr=\"\u{a0}a\"])"),
    // expect token `{`, but found `<eof>`
    (Syntax::Css, ":is(input[type=\"checkbox\u{a0}\"])"),
    // invalid An+B syntax
    (Syntax::Css, ":nth-child(\" 2n+ 1\u{a0}\")"),
    // expect token `{`, but found `,`
    (
        Syntax::Css,
        "@property --foo, --bar { syntax: \"<color>\"; inherits: false; initial-value: #f00; }\na { color: var(--bar); }",
    ),
];

/// Stylelint `computeEditInfo`: the fix as one replacement, narrowed by `narrowFixRange`.
/// Offsets are UTF-16 code units, like JavaScript string indices.
fn edit_info(source: &str, fix: &Fix) -> (u32, u32, String) {
    let (fixed, _) = apply_fixes(source, vec![fix.clone()]);
    let original: Vec<u16> = source.encode_utf16().collect();
    let text: Vec<u16> = fixed.encode_utf16().collect();
    let mut start = original
        .iter()
        .zip(&text)
        .take_while(|(a, b)| a == b)
        .count();
    let mut text_start = start;
    let max_suffix = (original.len() - start).min(text.len() - start);
    let suffix = original
        .iter()
        .rev()
        .zip(text.iter().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    let mut end = original.len() - suffix;
    let mut text_end = text.len() - suffix;
    if start == end {
        if start > 0 {
            start -= 1;
            text_start -= 1;
        } else if end < original.len() {
            end += 1;
            text_end += 1;
        }
    }
    (
        start as u32,
        end as u32,
        String::from_utf16_lossy(&text[text_start..text_end]),
    )
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
