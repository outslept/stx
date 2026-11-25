use oxc_css_parser::{Allocator, ParserBuilder, Syntax, ast::Stylesheet};
use std::{ffi::OsStr, process::ExitCode};
use stx::{config::Config, lint_and_fix, lint_once, rules::build_rules};

fn audit_fixtures() -> ExitCode {
    use serde_json::{Value, json};
    use std::io::{BufRead, Write};

    std::panic::set_hook(Box::new(|_| {}));
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    for line in std::io::stdin().lock().lines() {
        let req: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result = std::panic::catch_unwind(|| {
            let source = req["code"].as_str().unwrap();
            let syntax = match req["syntax"].as_str().unwrap() {
                "css" => Syntax::Css,
                "postcss-scss" => Syntax::Scss,
                "postcss-less" => Syntax::Less,
                "postcss-sass" => Syntax::Sass,
                _ => unreachable!(),
            };
            let config = Config::from_json_str(&req["config"].to_string()).unwrap();
            let rules = build_rules(&config);
            let allocator = Allocator::default();
            let parsed = ParserBuilder::new(&allocator, source)
                .syntax(syntax)
                .comments()
                .build()
                .parse::<Stylesheet>()
                .is_ok();
            let mut pass = lint_once(source, syntax, &rules, false);
            pass.warnings
                .sort_by_key(|warning| (warning.line, warning.column));
            let edits: Vec<_> = pass
                .fixes
                .iter()
                .map(|fix| {
                    fix.edits
                        .iter()
                        .map(|edit| {
                            let offset = |position: u32| {
                                source
                                    .get(..position as usize)
                                    .map(|prefix| prefix.encode_utf16().count())
                            };
                            json!({
                                "range": [offset(edit.span.start), offset(edit.span.end)],
                                "text": edit.replacement
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            let fixed = if req["checkFix"] == true || req["hasFixed"] == true {
                Some(lint_and_fix(source, syntax, &rules, false).0)
            } else {
                None
            };
            json!({
                "id": req["id"],
                "registered": !rules.is_empty(),
                "parsed": parsed,
                "warnings": pass.warnings,
                "fixed": fixed,
                "edits": edits
            })
        });
        let result = result.unwrap_or_else(|error| {
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic");
            json!({ "id": req["id"], "panic": message })
        });
        writeln!(stdout, "{result}").unwrap();
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    if std::env::args_os().nth(1).as_deref() == Some(OsStr::new("--audit-fixtures")) {
        return audit_fixtures();
    }
    ExitCode::from(stx::cli::run(std::env::args_os()))
}

#[cfg(test)]
mod tests {
    use super::{Config, Syntax, build_rules, lint_once};

    fn lines_with_warnings(source: &str, syntax: Syntax) -> Vec<usize> {
        let config = Config::from_json_str(r#"{"rules":{"color-hex-length":"short"}}"#).unwrap();
        let pass = lint_once(source, syntax, &build_rules(&config), false);
        pass.warnings.iter().map(|w| w.line).collect()
    }

    #[test]
    fn disable_comments_suppress_warnings() {
        let scss = "// stylelint-disable-next-line color-hex-length\na { color: #FFFFFF; }\nb { color: #FFFFFF; }\n";
        assert_eq!(lines_with_warnings(scss, Syntax::Scss), [3]);
        let less = "a { color: #FFFFFF; } // stylelint-disable-line color-hex-length\nb { color: #FFFFFF; }\n";
        assert_eq!(lines_with_warnings(less, Syntax::Less), [2]);
        let reenabled = "/* stylelint-disable */\na { color: #FFFFFF; }\n/* stylelint-enable color-hex-length */\nb { color: #FFFFFF; }\n";
        assert_eq!(lines_with_warnings(reenabled, Syntax::Css), [4]);
    }

    #[test]
    fn prioritized_secondary_options_are_applied() {
        for (source, config) in [
            (
                "a { unknown-property: 0; }",
                r#"{"rules":{"property-no-unknown":[true,{"ignoreProperties":["unknown-property"]}]}}"#,
            ),
            (
                "a { width: calc(0px); }",
                r#"{"rules":{"length-zero-no-unit":[true,{"ignoreFunctions":["calc"]}]}}"#,
            ),
            (
                "a { opacity: round(0.12345); }",
                r#"{"rules":{"number-max-precision":[2,{"insideFunctions":{"round":5}}]}}"#,
            ),
            (
                "a {}",
                r#"{"rules":{"selector-max-type":[0,{"ignoreTypes":["a"]}]}}"#,
            ),
            (
                "a { &:hover { a { color: red; } } }",
                r#"{"rules":{"max-nesting-depth":[1,{"ignorePseudoClasses":["hover"]}]}}"#,
            ),
            (
                "a { width: custom(1xyz); }",
                r#"{"rules":{"unit-no-unknown":[true,{"ignoreFunctions":["custom"]}]}}"#,
            ),
        ] {
            let config = Config::from_json_str(config).unwrap();
            let pass = lint_once(source, Syntax::Css, &build_rules(&config), false);
            assert!(pass.warnings.is_empty(), "{source}: {:?}", pass.warnings);
        }
    }

    #[test]
    fn default_severity_and_custom_message_are_applied() {
        let config = Config::from_json_str(
            r#"{"defaultSeverity":"warning","rules":{"block-no-empty":[true,{"message":"Custom text"}]}}"#,
        )
        .unwrap();
        let pass = lint_once("a {}", Syntax::Css, &build_rules(&config), false);
        assert_eq!(pass.warnings.len(), 1);
        assert_eq!(pass.warnings[0].severity, "warning");
        assert_eq!(pass.warnings[0].text, "Custom text (block-no-empty)");
    }

    #[test]
    fn fatal_parse_failure_reports_syntax_error() {
        let pass = lint_once(", a {}", Syntax::Css, &[], true);
        let warning = pass.parse_error.expect("expected a syntax error");
        assert_eq!(warning.rule, "CssSyntaxError");
        assert_eq!((warning.line, warning.column), (1, 1));
        assert_eq!(warning.severity, "error");
        assert!(
            lint_once("a {}", Syntax::Css, &[], false)
                .parse_error
                .is_none()
        );
    }
}
