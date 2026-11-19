mod config;
mod context;
mod diagnostic;
mod disable;
mod fix;
mod line_index;
mod rules;
mod visitor;

use clap::Parser;
use config::Config;
use context::LintContext;
use diagnostic::StylelintWarning;
use disable::DisableDirectives;
use fix::apply_fixes;
use ignore::WalkBuilder;
use line_index::LineIndex;
use oxc_allocator::Allocator;
use oxc_css_parser::{ParserBuilder, Syntax, ast::Stylesheet};
use rayon::prelude::*;
use rules::{ConfiguredRule, build_rules};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
use visitor::SinglePassVisitor;

#[derive(Parser, Debug)]
#[command(name = "stx", version, about = "Ultra-fast CSS/SCSS/Less linter")]
struct Cli {
    #[arg(default_value = ".")]
    files: Vec<PathBuf>,

    #[arg(long)]
    fix: bool,

    #[arg(short = 'f', long = "formatter", default_value = "text")]
    formatter: String,

    #[arg(short = 'c', long = "config")]
    config: Option<PathBuf>,

    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
}

#[derive(Debug, Serialize)]
struct FileReport {
    source: String,
    errored: bool,
    warnings: Vec<StylelintWarning>,
}

fn syntax_for_path(path: &Path) -> Option<Syntax> {
    match path.extension()?.to_str()? {
        "css" => Some(Syntax::Css),
        "scss" => Some(Syntax::Scss),
        "sass" => Some(Syntax::Sass),
        "less" => Some(Syntax::Less),
        _ => None,
    }
}

fn collect_files(inputs: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for input in inputs {
        if input.is_file() {
            if syntax_for_path(input).is_some() {
                out.push(input.clone());
            }
            continue;
        }
        for entry in WalkBuilder::new(input)
            .standard_filters(true)
            .build()
            .flatten()
        {
            let path = entry.path();
            if path.is_file() && syntax_for_path(path).is_some() {
                out.push(path.to_path_buf());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn lint_once(
    source: &str,
    syntax: Syntax,
    active_rules: &[ConfiguredRule],
    quiet: bool,
) -> (Vec<StylelintWarning>, Vec<fix::Fix>) {
    let allocator = Allocator::default();
    let mut parser = ParserBuilder::new(&allocator, source)
        .syntax(syntax)
        .comments()
        .build();

    let Ok(stylesheet) = parser.parse::<Stylesheet<'_>>() else {
        return (Vec::new(), Vec::new());
    };

    let comments = parser.comments();
    let line_index = LineIndex::new(source);
    let disables = DisableDirectives::parse(comments, source, &line_index);

    let mut ctx = LintContext::new(source, &stylesheet, comments, &line_index, &disables);
    SinglePassVisitor::new(&mut ctx, active_rules).visit_stylesheet(&stylesheet);

    let fixes = ctx.take_fixes();
    let warnings = ctx.into_warnings(quiet);
    (warnings, fixes)
}

fn process_file(
    path: &Path,
    active_rules: &[ConfiguredRule],
    fix: bool,
    quiet: bool,
) -> Option<FileReport> {
    let syntax = syntax_for_path(path)?;
    let source = fs::read_to_string(path).ok()?;

    let warnings = if !fix {
        let (w, _) = lint_once(&source, syntax, active_rules, quiet);
        w
    } else {
        let mut current = source.clone();
        let mut changed = false;
        for _ in 0..5 {
            let (_, fixes) = lint_once(&current, syntax, active_rules, false);
            let (next, applied) = apply_fixes(&current, fixes);
            current = next;
            if applied {
                changed = true;
            } else {
                break;
            }
        }
        if changed {
            let _ = fs::write(path, &current);
        }
        let (w, _) = lint_once(&current, syntax, active_rules, quiet);
        w
    };

    let errored = warnings.iter().any(|w| w.severity == "error");
    Some(FileReport {
        source: path.to_string_lossy().into_owned(),
        errored,
        warnings,
    })
}

fn print_text_report(reports: &[FileReport]) {
    let mut total_problems = 0;
    for report in reports {
        if report.warnings.is_empty() {
            continue;
        }
        println!("\n{}", report.source);
        for w in &report.warnings {
            total_problems += 1;
            let sym = if w.severity == "error" { "✖" } else { "⚠" };
            println!(
                "  {:<7}  {}  {}",
                format!("{}:{}", w.line, w.column),
                sym,
                w.text
            );
        }
    }
    if total_problems > 0 {
        println!("\n✖ {} problems", total_problems);
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config = if let Some(ref cfg_path) = cli.config {
        Config::from_file(cfg_path).unwrap_or_else(|_| Config::standard_preset())
    } else {
        Config::discover(&cwd).unwrap_or_else(Config::standard_preset)
    };

    let active_rules = build_rules(&config);
    let files = collect_files(&cli.files);

    let reports: Vec<FileReport> = files
        .par_iter()
        .filter_map(|path| process_file(path, &active_rules, cli.fix, cli.quiet))
        .collect();

    let has_errors = reports.iter().any(|r| r.errored);

    match cli.formatter.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&reports).unwrap());
        }
        _ => {
            print_text_report(&reports);
        }
    }

    if has_errors {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}
