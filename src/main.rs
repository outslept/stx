mod config;
mod context;
mod diagnostic;
mod disable;
mod fix;
mod line_index;
pub mod rules;
#[cfg(test)]
pub mod tester;
pub mod utils;
mod visitor;

use clap::Parser as ClapParser;
use config::Config;
use context::LintContext;
use diagnostic::StylelintWarning;
use disable::DisableDirectives;
use ignore::WalkBuilder;
use line_index::LineIndex;
use oxc_css_parser::{Allocator, ParserBuilder, Syntax, ast::Stylesheet, token::Comment};
use rayon::prelude::*;
use rules::{ConfiguredRule, build_rules};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};
use visitor::SinglePassVisitor;

#[derive(ClapParser, Debug)]
#[command(
    name = "stx",
    version,
    about = "Ultra-fast CSS/SCSS/Less linter written in Rust"
)]
struct Cli {
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    #[arg(short, long)]
    config: Option<PathBuf>,
    #[arg(long)]
    fix: bool,
    #[arg(short, long, default_value = "text")]
    formatter: String,
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Serialize)]
struct FileReport {
    source: String,
    errored: bool,
    warnings: Vec<StylelintWarning>,
}

fn syntax_for_path(path: &Path) -> Option<Syntax> {
    match path.extension()?.to_str()? {
        "css" | "pcss" | "postcss" => Some(Syntax::Css),
        "scss" => Some(Syntax::Scss),
        "sass" | "sss" | "sugarss" => Some(Syntax::Sass),
        "less" => Some(Syntax::Less),
        _ => None,
    }
}

fn collect_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in paths {
        if root.is_file() {
            if syntax_for_path(root).is_some() {
                files.push(root.clone());
            }
            continue;
        }
        let walker = WalkBuilder::new(root)
            .hidden(true)
            .git_ignore(true)
            .filter_entry(|entry| {
                let name = entry.file_name().to_str().unwrap_or("");
                name != "node_modules" && name != "target" && name != "dist" && name != "build"
            })
            .build();
        for entry in walker.flatten() {
            let p = entry.path();
            if p.is_file() && syntax_for_path(p).is_some() {
                files.push(p.to_path_buf());
            }
        }
    }
    files
}

fn run_pass<'a>(
    source: &'a str,
    stylesheet: &'a Stylesheet<'a>,
    comments: &'a [Comment<'a>],
    active_rules: &[ConfiguredRule],
    quiet: bool,
) -> (Vec<StylelintWarning>, Vec<fix::Fix>) {
    let line_index = LineIndex::new(source);
    let disables = DisableDirectives::parse(comments, source, &line_index);
    let mut ctx = LintContext::new(source, stylesheet, comments, &line_index, &disables);
    SinglePassVisitor::new(&mut ctx, active_rules).visit_stylesheet(stylesheet);
    let fixes = ctx.take_fixes();
    (ctx.into_warnings(quiet), fixes)
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
    if let Ok(stylesheet) = parser.parse::<Stylesheet>() {
        let comments = parser.comments();
        return run_pass(source, &stylesheet, &comments, active_rules, quiet);
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
            return run_pass(source, &stylesheet, &comments, active_rules, quiet);
        }
    }

    let empty_alloc = Allocator::default();
    let mut empty_parser = ParserBuilder::new(&empty_alloc, "")
        .syntax(syntax)
        .comments()
        .build();
    if let Ok(stylesheet) = empty_parser.parse::<Stylesheet>() {
        let comments = empty_parser.comments();
        return run_pass(source, &stylesheet, &comments, active_rules, quiet);
    }

    (Vec::new(), Vec::new())
}

fn process_file(
    path: &Path,
    active_rules: &[ConfiguredRule],
    do_fix: bool,
    quiet: bool,
) -> Option<FileReport> {
    let syntax = syntax_for_path(path)?;
    let mut source = fs::read_to_string(path).ok()?;
    let mut warnings = if do_fix {
        let mut modified = false;
        for _ in 0..10 {
            let (_, fixes) = lint_once(&source, syntax, active_rules, quiet);
            if fixes.is_empty() {
                break;
            }
            let (new_source, applied) = fix::apply_fixes(&source, fixes);
            if !applied || new_source == source {
                break;
            }
            source = new_source;
            modified = true;
        }
        if modified {
            let _ = fs::write(path, &source);
        }
        lint_once(&source, syntax, active_rules, quiet).0
    } else {
        lint_once(&source, syntax, active_rules, quiet).0
    };
    warnings.sort_by(|a, b| a.line.cmp(&b.line).then(a.column.cmp(&b.column)));
    let errored = warnings.iter().any(|w| w.severity == "error");
    Some(FileReport {
        source: path.to_string_lossy().into_owned(),
        errored,
        warnings,
    })
}

fn print_text_report(reports: &[FileReport], elapsed_ms: u128, total_files: usize) {
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    for rep in reports {
        if rep.warnings.is_empty() {
            continue;
        }
        println!("\n\x1b[4m{}\x1b[0m", rep.source);
        for w in &rep.warnings {
            let sev_symbol = if w.severity == "error" {
                total_errors += 1;
                "\x1b[31m✖\x1b[0m"
            } else {
                total_warnings += 1;
                "\x1b[33m⚠\x1b[0m"
            };
            let pos_width = w.line.max(1).ilog10() as usize + w.column.max(1).ilog10() as usize + 3;
            let pad = 8usize.saturating_sub(pos_width);
            println!(
                "  \x1b[2m{}:{}\x1b[0m{:pad$}  {}  {}",
                w.line,
                w.column,
                "",
                sev_symbol,
                w.text,
                pad = pad
            );
        }
    }
    let total_problems = total_errors + total_warnings;
    if total_problems > 0 {
        println!(
            "\n\x1b[31m✖ {} problem{} ({} error{}, {} warning{})\x1b[0m \x1b[2m[{} files in {}ms]\x1b[0m",
            total_problems,
            if total_problems == 1 { "" } else { "s" },
            total_errors,
            if total_errors == 1 { "" } else { "s" },
            total_warnings,
            if total_warnings == 1 { "" } else { "s" },
            total_files,
            elapsed_ms
        );
    } else {
        println!(
            "\x1b[32m✔ 0 problems\x1b[0m \x1b[2m[{} files linted in {}ms]\x1b[0m",
            total_files, elapsed_ms
        );
    }
}

fn main() -> ExitCode {
    let start_time = Instant::now();
    let cli = Cli::parse();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config = if let Some(ref cfg_path) = cli.config {
        match Config::from_file(cfg_path) {
            Ok(cfg) => cfg,
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(2);
            }
        }
    } else if let Some(discovered) = Config::discover(&cwd) {
        Config::from_file(&discovered).unwrap_or_else(|_| Config::standard_preset())
    } else {
        Config::standard_preset()
    };
    let active_rules = build_rules(&config);
    let files = collect_files(&cli.paths);
    let total_files = files.len();
    let mut reports: Vec<FileReport> = files
        .par_iter()
        .filter_map(|path| process_file(path, &active_rules, cli.fix, cli.quiet))
        .collect();
    reports.sort_by(|a, b| a.source.cmp(&b.source));
    let has_errors = reports.iter().any(|r| r.errored);
    let elapsed_ms = start_time.elapsed().as_millis();
    if cli.formatter == "json" {
        if let Ok(json) = serde_json::to_string(&reports) {
            println!("{json}");
        }
    } else {
        print_text_report(&reports, elapsed_ms, total_files);
    }
    if has_errors {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}
