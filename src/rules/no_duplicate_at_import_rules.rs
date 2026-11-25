use std::collections::HashSet;

use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::{Statement, Stylesheet};

pub struct NoDuplicateAtImportRules;

impl Rule for NoDuplicateAtImportRules {
    fn name(&self) -> &'static str {
        "no-duplicate-at-import-rules"
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let src = ctx.source;

        for stmt in &stylesheet.statements {
            if let Statement::AtRule(at_rule) = stmt {
                let ident = &at_rule.name;
                if !ident.name.eq_ignore_ascii_case("import") {
                    continue;
                }

                let prelude_start = ident.span.end as usize;
                let prelude_end = at_rule.span.end as usize;
                if prelude_start >= prelude_end || prelude_end > src.len() {
                    continue;
                }

                let raw_prelude = src[prelude_start..prelude_end]
                    .trim()
                    .trim_end_matches(';')
                    .trim();

                let Some((uri, rest)) = parse_import_prelude(raw_prelude) else {
                    continue;
                };

                if rest.is_empty() {
                    if !seen.insert((uri.clone(), String::new())) {
                        ctx.report(
                            self.name(),
                            at_rule.span,
                            format!("Duplicate @import rule {uri}"),
                        );
                    }
                } else {
                    let mut reported = false;
                    for media in rest.split(',') {
                        let norm_media = media
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_ascii_lowercase();
                        if !seen.insert((uri.clone(), norm_media)) && !reported {
                            reported = true;
                            ctx.report(
                                self.name(),
                                at_rule.span,
                                format!("Duplicate @import rule {uri}"),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn parse_import_prelude(prelude: &str) -> Option<(String, &str)> {
    let s = prelude.trim();
    if s.is_empty() {
        return None;
    }

    if s.len() >= 4 && s[..4].eq_ignore_ascii_case("url(") {
        let after_url = &s[4..];
        let close_idx = after_url.find(')')?;
        let inner = after_url[..close_idx].trim();
        let unquoted = inner.trim_matches(|c| c == '"' || c == '\'').to_string();
        let rest = after_url[close_idx + 1..].trim();
        return Some((unquoted, rest));
    }

    let first = s.as_bytes()[0];
    if first == b'"' || first == b'\'' {
        let quote = first as char;
        let rest_slice = &s[1..];
        let end_quote = rest_slice.find(quote)?;
        let uri = rest_slice[..end_quote].to_string();
        let rest = rest_slice[end_quote + 1..].trim();
        return Some((uri, rest));
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn duplicate_import_message_matches_stylelint_18() {
        let pass = ["@import 'a.css'; @import 'b.css';"];
        let fail = [reject(
            "@import 'a.css'; @import 'a.css';",
            None,
            "Duplicate @import rule a.css",
            (1, 18, 1, 33),
        )];
        Tester::new("no-duplicate-at-import-rules", json!(true), &pass, &fail).test();
    }
}
