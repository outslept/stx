//! STX lints and fixes CSS, SCSS, Sass, and Less stylesheets.
//!
//! See [`rules`] for rule configuration and CSS examples, and [`utils`] for the
//! source, AST, and configuration helpers used by rule implementations.
pub mod cli;
pub mod config;
pub mod diagnostic;
mod lint;
pub mod rules;
#[cfg(test)]
mod tester;
pub mod utils;

pub use lint::{LintPass, lint_and_fix, lint_once};
pub(crate) use lint::{context, disable, fix, fix::fix_from_pass, line_index};
pub use oxc_css_parser::Syntax;
use std::path::Path;

pub fn syntax_for_path(path: &Path) -> Option<Syntax> {
    match path.extension()?.to_str()? {
        "css" | "pcss" | "postcss" => Some(Syntax::Css),
        "scss" => Some(Syntax::Scss),
        "sass" | "sss" | "sugarss" => Some(Syntax::Sass),
        "less" => Some(Syntax::Less),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Syntax, config::Config, lint_once, rules::build_rules};

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

    #[test]
    fn top_level_src_entries_match_layers() {
        const TOP_LEVEL: &[&str] = &[
            "bin",
            "cli.rs",
            "config",
            "diagnostic.rs",
            "lib.rs",
            "lint",
            "main.rs",
            "rules",
            "tester.rs",
            "utils",
        ];
        let mut entries: Vec<_> = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        entries.sort();
        assert_eq!(entries, TOP_LEVEL);
    }
}
