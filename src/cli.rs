use crate::{
    config::Config, diagnostic::StylelintWarning, fix_from_pass, lint_once, rules::build_rules,
    syntax_for_path,
};
use clap::{CommandFactory, Parser, ValueEnum};
use globset::{GlobBuilder, GlobMatcher};
use ignore::{
    WalkBuilder,
    gitignore::{Gitignore, GitignoreBuilder},
};
use rayon::prelude::*;
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap, hash_map::Entry},
    ffi::OsString,
    fmt::{Display, Write as _},
    fs,
    io::{self, IsTerminal, Read as _, Write as _},
    path::{Component, Path, PathBuf},
};

type CliResult<T> = Result<T, (u8, String)>;

#[derive(Parser)]
#[command(name = "stx", version, about = "CSS, SCSS, Sass, and Less linter")]
struct Cli {
    #[arg(
        value_name = "PATH_OR_GLOB",
        help = "Files, directories, or quoted globs (prefix a glob with ! to exclude)"
    )]
    paths: Vec<PathBuf>,
    #[arg(short, long, help = "Path to a JSON config file")]
    config: Option<PathBuf>,
    #[arg(long, help = "Fix problems in place, or write fixed stdin to stdout")]
    fix: bool,
    #[arg(
        short,
        long,
        value_enum,
        default_value = "string",
        help = "Report format (written to stderr)"
    )]
    formatter: Formatter,
    #[arg(short, long, help = "Only report problems with error severity")]
    quiet: bool,
    #[arg(long, help = "Accept stdin, even if empty")]
    stdin: bool,
    #[arg(
        long,
        help = "Virtual filename for stdin (used for syntax and config lookup)"
    )]
    stdin_filename: Option<PathBuf>,
    #[arg(long, help = "Succeed when no files matched")]
    allow_empty_input: bool,
    #[arg(
        short = 'i',
        long,
        help = "Read ignore patterns from this file instead of .stylelintignore"
    )]
    ignore_path: Vec<PathBuf>,
    #[arg(long, help = "Additional Git-ignore pattern")]
    ignore_pattern: Vec<String>,
    #[arg(long, help = "Do not automatically ignore node_modules")]
    disable_default_ignores: bool,
    #[arg(long, help = "Fail when the number of warnings exceeds this limit")]
    max_warnings: Option<usize>,
    #[arg(long, help = "Also write the report to this file")]
    output_file: Option<PathBuf>,
}

#[derive(Clone, ValueEnum)]
enum Formatter {
    String,
    Json,
    #[value(alias = "text")]
    Verbose,
}

#[derive(Default, Serialize)]
struct FileReport {
    source: String,
    errored: bool,
    warnings: Vec<StylelintWarning>,
    ignored: bool,
    autofixed: bool,
}

fn fatal(error: impl Display) -> (u8, String) {
    (1, error.to_string())
}

fn absolute(cwd: &Path, path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in cwd.join(path).components() {
        if component == Component::ParentDir {
            result.pop();
        } else {
            result.push(component.as_os_str());
        }
    }
    result
}

fn ignore_matcher(
    base: &Path,
    files: &[PathBuf],
    patterns: &[String],
) -> Result<Gitignore, String> {
    let mut builder = GitignoreBuilder::new(base);
    for path in files {
        let path = base.join(path);
        if path.try_exists().map_err(|error| error.to_string())?
            && let Some(error) = builder.add(&path)
        {
            return Err(error.to_string());
        }
    }
    for pattern in patterns {
        builder
            .add_line(None, pattern)
            .map_err(|error| error.to_string())?;
    }
    builder.build().map_err(|error| error.to_string())
}

fn ignored_by(ignorer: &Gitignore, path: &Path) -> bool {
    path.strip_prefix(ignorer.path()).is_ok_and(|relative| {
        ignorer
            .matched_path_or_any_parents(relative, false)
            .is_ignore()
    })
}

fn default_ignored(path: &Path) -> bool {
    path.components()
        .any(|part| part.as_os_str() == "node_modules")
}

struct FilePattern {
    root: PathBuf,
    matcher: Option<GlobMatcher>,
    max_depth: Option<usize>,
}

impl FilePattern {
    fn new(path: PathBuf) -> CliResult<Self> {
        if path.exists() {
            return Ok(Self {
                root: path,
                matcher: None,
                max_depth: None,
            });
        }
        let mut parts = path.components().peekable();
        let mut root = PathBuf::new();
        while let Some(part) = parts.next_if(|part| {
            !part.as_os_str().to_str().is_some_and(|s| {
                s.contains(['*', '?', '[', '{']) || (!cfg!(windows) && s.contains('\\'))
            })
        }) {
            root.push(part.as_os_str());
        }
        let depth = parts.clone().count();
        let suffix: PathBuf = parts.collect();
        if depth == 0 {
            return Ok(Self {
                root,
                matcher: None,
                max_depth: None,
            });
        }
        let pattern = suffix
            .to_str()
            .ok_or_else(|| (64, format!("Invalid glob: {}", path.display())))?;
        let pattern = if cfg!(windows) {
            pattern.replace('\\', "/")
        } else {
            pattern.to_owned()
        };
        let glob = GlobBuilder::new(&pattern)
            .literal_separator(true)
            .backslash_escape(!cfg!(windows))
            .build()
            .map_err(|error| (64, error.to_string()))?;
        Ok(Self {
            root,
            matcher: Some(glob.compile_matcher()),
            max_depth: (!pattern.contains("**")).then_some(depth),
        })
    }

    fn matches(&self, path: &Path) -> bool {
        path.strip_prefix(&self.root).is_ok_and(|relative| {
            self.matcher
                .as_ref()
                .is_none_or(|matcher| matcher.is_match(relative))
        })
    }
}

fn collect_files(cli: &Cli, cwd: &Path, ignored: &Gitignore) -> CliResult<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    let mut excludes = Vec::new();
    let use_default_ignores = !cli.disable_default_ignores;
    for input in &cli.paths {
        let path = absolute(cwd, input);
        if let Some(exclude) = input.to_str().and_then(|value| value.strip_prefix('!'))
            && !path.exists()
        {
            excludes.push(FilePattern::new(absolute(cwd, Path::new(exclude)))?);
            continue;
        }
        let pattern = FilePattern::new(path)?;
        if use_default_ignores && default_ignored(&pattern.root) {
            continue;
        }
        let metadata = match fs::metadata(&pattern.root) {
            Ok(metadata) => metadata,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                continue;
            }
            Err(error) => return Err(fatal(format!("{}: {error}", pattern.root.display()))),
        };
        if metadata.is_file() && pattern.matcher.is_none() {
            files.insert(pattern.root);
        } else if metadata.is_dir() {
            let walk = WalkBuilder::new(&pattern.root)
                .standard_filters(false)
                .hidden(true)
                .max_depth(pattern.max_depth)
                .filter_entry(move |entry| {
                    !use_default_ignores || entry.file_name() != "node_modules"
                })
                .build();
            for entry in walk {
                let entry = entry.map_err(fatal)?;
                let path = entry.path();
                if path.is_file() && pattern.matches(path) && syntax_for_path(path).is_some() {
                    files.insert(entry.into_path());
                }
            }
        }
    }
    files.retain(|path| {
        syntax_for_path(path).is_some()
            && (!use_default_ignores || !default_ignored(path))
            && !ignored_by(ignored, path)
            && !excludes.iter().any(|exclude| exclude.matches(path))
    });
    if files.is_empty() && !cli.allow_empty_input {
        return Err(fatal(
            "No files matching the given paths or globs were found",
        ));
    }
    Ok(files)
}

fn load_config(path: Option<&Path>) -> CliResult<(Config, Gitignore)> {
    let Some(path) = path else {
        return Ok((Config::standard_preset(), Gitignore::empty()));
    };
    let config = Config::from_file(path).map_err(|message| (78, message))?;
    if !config.validation_errors.is_empty() {
        return Err((78, config.validation_errors.join("\n")));
    }
    let ignores = ignore_matcher(
        path.parent().unwrap_or(Path::new(".")),
        &[],
        &config.ignore_files,
    )
    .map_err(|message| (78, message))?;
    Ok((config, ignores))
}

fn lint_source(
    source: &str,
    path: Option<&Path>,
    config: &Config,
    cli: &Cli,
) -> (FileReport, Option<String>) {
    let syntax = path.and_then(syntax_for_path).unwrap_or_default();
    let rules = if let Some(path) = path
        && !config.overrides.is_empty()
    {
        build_rules(&config.for_file(path))
    } else {
        build_rules(config)
    };
    let pass = lint_once(source, syntax, &rules, cli.quiet);
    let (pass, fixed) = if cli.fix && pass.parse_error.is_none() {
        let (fixed, pass) = fix_from_pass(source, syntax, &rules, cli.quiet, pass);
        (pass, Some(fixed))
    } else {
        (pass, None)
    };
    let warnings = pass.into_warnings();
    (
        FileReport {
            source: path.map_or_else(|| "<stdin>".into(), |path| path.display().to_string()),
            errored: warnings.iter().any(|warning| warning.severity == "error"),
            warnings,
            autofixed: fixed.as_deref().is_some_and(|fixed| fixed != source),
            ignored: false,
        },
        fixed,
    )
}

fn lint_file(
    path: &Path,
    (config, ignored): &(Config, Gitignore),
    cli: &Cli,
) -> CliResult<FileReport> {
    if ignored_by(ignored, path) {
        return Ok(FileReport {
            source: path.display().to_string(),
            ignored: true,
            ..FileReport::default()
        });
    }
    let source =
        fs::read_to_string(path).map_err(|error| fatal(format!("{}: {error}", path.display())))?;
    let (report, fixed) = lint_source(&source, Some(path), config, cli);
    if let Some(fixed) = fixed
        && report.autofixed
    {
        fs::write(path, fixed).map_err(|error| fatal(format!("{}: {error}", path.display())))?;
    }
    Ok(report)
}

fn format_report(
    cwd: &Path,
    reports: &[FileReport],
    formatter: &Formatter,
    warnings: usize,
) -> CliResult<String> {
    if matches!(formatter, Formatter::Json) {
        return serde_json::to_string(reports)
            .map(|json| json + "\n")
            .map_err(fatal);
    }
    let mut output = String::new();
    for report in reports.iter().filter(|report| !report.warnings.is_empty()) {
        let path = Path::new(&report.source);
        let _ = writeln!(
            output,
            "\n{}",
            path.strip_prefix(cwd).unwrap_or(path).display()
        );
        for warning in &report.warnings {
            let _ = writeln!(
                output,
                "  {}:{}  {}  {}",
                warning.line, warning.column, warning.severity, warning.text
            );
        }
    }
    if warnings > 0 || matches!(formatter, Formatter::Verbose) {
        let problems = if warnings == 1 { "problem" } else { "problems" };
        let files = if reports.len() == 1 { "file" } else { "files" };
        let _ = writeln!(
            output,
            "\n{warnings} {problems} in {} {files}",
            reports.len()
        );
    }
    Ok(output)
}

fn lint(cli: Cli) -> CliResult<u8> {
    let mut stdin = io::stdin();
    if cli.paths.is_empty() && stdin.is_terminal() {
        if cli.stdin {
            return Err(fatal("No piped input for --stdin"));
        }
        Cli::command().print_help().map_err(fatal)?;
        return Ok(0);
    }
    let cwd = std::env::current_dir().map_err(fatal)?;
    let default_ignore = [PathBuf::from(".stylelintignore")];
    let ignore_paths = if cli.ignore_path.is_empty() {
        &default_ignore[..]
    } else {
        &cli.ignore_path
    };
    let ignored = ignore_matcher(&cwd, ignore_paths, &cli.ignore_pattern).map_err(fatal)?;
    let explicit = cli.config.as_ref().map(|path| absolute(&cwd, path));
    let fallback = Config::discover(&cwd);
    let discover = |directory: &Path| {
        explicit
            .clone()
            .or_else(|| Config::discover(directory))
            .or_else(|| fallback.clone())
    };
    let mut configs = HashMap::new();
    if let Some(path) = explicit.as_ref() {
        configs.insert(Some(path.clone()), load_config(Some(path))?);
    }
    let reports = if cli.paths.is_empty() {
        let path = cli.stdin_filename.as_ref().map(|path| absolute(&cwd, path));
        let path = path.as_deref();
        let config_path = discover(path.and_then(Path::parent).unwrap_or(&cwd));
        let (config, config_ignores) = match configs.remove(&config_path) {
            Some(config) => config,
            None => load_config(config_path.as_deref())?,
        };
        let mut source = String::new();
        stdin.read_to_string(&mut source).map_err(fatal)?;
        if path.is_some_and(|path| {
            (!cli.disable_default_ignores && default_ignored(path))
                || ignored_by(&ignored, path)
                || ignored_by(&config_ignores, path)
        }) {
            Vec::new()
        } else {
            let (report, fixed) = lint_source(&source, path, &config, &cli);
            if let Some(fixed) = fixed {
                let mut stdout = io::stdout().lock();
                stdout.write_all(fixed.as_bytes()).map_err(fatal)?;
                stdout.flush().map_err(fatal)?;
            }
            vec![report]
        }
    } else {
        let files = collect_files(&cli, &cwd, &ignored)?;
        let mut directories = HashMap::new();
        let mut inputs = Vec::with_capacity(files.len());
        for path in files {
            let directory = path.parent().unwrap_or(&cwd);
            let key = directories
                .entry(directory.to_path_buf())
                .or_insert_with(|| discover(directory));
            if let Entry::Vacant(slot) = configs.entry(key.clone()) {
                slot.insert(load_config(key.as_deref())?);
            }
            inputs.push((path, key.clone()));
        }
        inputs
            .par_iter()
            .map(|(path, key)| lint_file(path, &configs[key], &cli))
            .collect::<CliResult<Vec<_>>>()?
    };
    let warnings = reports.iter().map(|report| report.warnings.len()).sum();
    let report = format_report(&cwd, &reports, &cli.formatter, warnings)?;
    io::stderr()
        .lock()
        .write_all(report.as_bytes())
        .map_err(fatal)?;
    if let Some(path) = cli.output_file {
        fs::write(&path, report).map_err(|error| fatal(format!("{}: {error}", path.display())))?;
    }
    if reports.iter().any(|report| report.errored) {
        Ok(2)
    } else if let Some(max) = cli.max_warnings
        && warnings > max
    {
        eprintln!("Max warnings exceeded: {warnings} found. {max} allowed");
        Ok(2)
    } else {
        Ok(0)
    }
}

pub fn run(args: impl IntoIterator<Item = OsString>) -> u8 {
    match Cli::try_parse_from(args) {
        Ok(cli) => match lint(cli) {
            Ok(code) => code,
            Err((code, message)) => {
                eprintln!("{message}");
                code
            }
        },
        Err(error) => {
            let invalid = error.use_stderr();
            if error.print().is_err() {
                return 1;
            }
            if invalid { 64 } else { 0 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, ignore_matcher, ignored_by};

    #[test]
    fn configured_ignore_files_match_paths_under_the_config_directory() {
        let config = Config::from_json_str(r#"{"ignoreFiles":["vendor/**"]}"#).unwrap();
        let cwd = std::env::current_dir().unwrap();
        let root = cwd.join("project");
        let matcher = ignore_matcher(&root, &[], &config.ignore_files).unwrap();
        assert!(ignored_by(&matcher, &root.join("vendor/reset.css")));
        assert!(!ignored_by(&matcher, &root.join("src/reset.css")));
        assert!(!ignored_by(&matcher, &cwd.join("vendor/reset.css")));
    }
}
