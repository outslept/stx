use napi::{Error, Result, Status};
use napi_derive::napi;
use serde_json::Value;
use std::{ffi::OsString, path::Path};
use stx::{Syntax, config::Config, lint_and_fix, lint_once, rules::build_rules, syntax_for_path};

#[napi(string_enum = "lowercase")]
pub enum LintSyntax {
    Css,
    Scss,
    Sass,
    Less,
}

impl From<LintSyntax> for Syntax {
    fn from(syntax: LintSyntax) -> Self {
        match syntax {
            LintSyntax::Css => Self::Css,
            LintSyntax::Scss => Self::Scss,
            LintSyntax::Sass => Self::Sass,
            LintSyntax::Less => Self::Less,
        }
    }
}

#[derive(Default)]
#[napi(object)]
pub struct LintOptions {
    pub syntax: Option<LintSyntax>,
    pub filename: Option<String>,
    pub fix: Option<bool>,
    #[napi(ts_type = "Record<string, unknown>")]
    pub config: Option<Value>,
}

#[napi(object, object_from_js = false)]
pub struct LintWarning {
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub rule: String,
    #[napi(ts_type = "'error' | 'warning'")]
    pub severity: String,
    pub text: String,
}

#[napi(object, object_from_js = false)]
pub struct LintResult {
    pub code: String,
    pub errored: bool,
    pub warnings: Vec<LintWarning>,
}

#[napi(js_name = "__runCli", skip_typescript, strict, catch_unwind)]
pub fn run_cli(args: Vec<String>) -> u32 {
    u32::from(stx::cli::run(args.into_iter().map(OsString::from)))
}

#[napi(strict, catch_unwind)]
pub fn lint(code: String, options: Option<LintOptions>) -> Result<LintResult> {
    let LintOptions {
        syntax,
        filename,
        fix,
        config,
    } = options.unwrap_or_default();
    let mut config = match config {
        Some(Value::Object(config)) => Config::from_json_object(config)
            .map_err(|error| Error::new(Status::InvalidArg, error))?,
        Some(_) => return Err(Error::new(Status::InvalidArg, "config must be an object")),
        None => Config::standard_preset(),
    };
    if !config.validation_errors.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            config.validation_errors.join("\n"),
        ));
    }
    let path = filename.as_deref().map(Path::new);
    let syntax = syntax
        .map(Syntax::from)
        .or_else(|| path.and_then(syntax_for_path))
        .unwrap_or_default();
    if let Some(path) = path {
        config = config.for_file(path);
    }
    let rules = build_rules(&config);
    let (code, pass) = if fix.unwrap_or(false) {
        lint_and_fix(&code, syntax, &rules, false)
    } else {
        let pass = lint_once(&code, syntax, &rules, false);
        (code, pass)
    };
    let warnings = pass.into_warnings();
    Ok(LintResult {
        code,
        errored: warnings.iter().any(|warning| warning.severity == "error"),
        warnings: warnings
            .into_iter()
            .map(|warning| LintWarning {
                line: warning.line as u32,
                column: warning.column as u32,
                end_line: warning.end_line as u32,
                end_column: warning.end_column as u32,
                rule: warning.rule.to_owned(),
                severity: warning.severity.to_owned(),
                text: warning.text,
            })
            .collect(),
    })
}
