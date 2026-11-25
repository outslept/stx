use oxc_css_parser::{Allocator, ParserBuilder, Syntax, ast::Stylesheet};
use serde_json::{Value, json};
use std::{
    io::{BufRead, Write},
    process::ExitCode,
};
use stx::{config::Config, lint_and_fix, lint_once, rules::build_rules};

fn main() -> ExitCode {
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
            let pass = lint_once(source, syntax, &rules, false);
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
