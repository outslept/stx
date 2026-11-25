use crate::{context::LintContext, rules::Rule, utils::at_rule_prelude_span};
use oxc_css_parser::{
    Span,
    ast::{AtRule, Statement, Stylesheet},
};
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct NoUnknownCustomMedia;

fn custom_media_name(raw: &str) -> Option<&str> {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("--") {
        return None;
    }
    let end = trimmed
        .find(|c: char| c.is_ascii_whitespace() || c == '(' || c == ')')
        .unwrap_or(trimmed.len());
    (&end > &2).then_some(&trimmed[..end])
}

fn collect<'a>(at_rules: &mut Vec<&'a AtRule<'a>>, statements: &'a [Statement<'a>]) {
    for st in statements {
        match st {
            Statement::AtRule(a) => {
                at_rules.push(a);
                if let Some(b) = &a.block {
                    collect(at_rules, &b.statements);
                }
            }
            Statement::QualifiedRule(r) => collect(at_rules, &r.block.statements),
            Statement::KeyframeBlock(k) => collect(at_rules, &k.block.statements),
            _ => {}
        }
    }
}

impl Rule for NoUnknownCustomMedia {
    fn name(&self) -> &'static str {
        "no-unknown-custom-media"
    }
    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        let stylesheet = ctx.stylesheet;
        let source = ctx.source;
        let mut at_rules = Vec::new();
        collect(&mut at_rules, &stylesheet.statements);
        let mut declared = HashSet::new();
        for at_rule in &at_rules {
            if at_rule.name.name.eq_ignore_ascii_case("custom-media") {
                if let Some(sp) = at_rule_prelude_span(source, at_rule) {
                    if let Some(name) =
                        custom_media_name(&source[sp.start as usize..sp.end as usize])
                    {
                        declared.insert(name.to_string());
                    }
                }
            }
        }
        for at_rule in at_rules {
            if !at_rule.name.name.eq_ignore_ascii_case("media") {
                continue;
            }
            let Some(sp) = at_rule_prelude_span(source, at_rule) else {
                continue;
            };
            let params = &source[sp.start as usize..sp.end as usize];
            let bytes = params.as_bytes();
            let len = bytes.len();
            let mut i = 0usize;
            while i < len {
                if bytes[i] == b'(' {
                    let mut j = i + 1;
                    while j < len && bytes[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j + 2 <= len && &params[j..j + 2] == "--" {
                        let start = j;
                        j += 2;
                        while j < len
                            && !bytes[j].is_ascii_whitespace()
                            && !matches!(bytes[j], b')' | b':' | b',')
                        {
                            j += 1;
                        }
                        let name = &params[start..j];
                        if !declared.contains(name) {
                            ctx.report(
                                self.name(),
                                Span {
                                    start: sp.start + start as u32,
                                    end: sp.start + j as u32,
                                },
                                format!("Unknown custom media query \"{name}\""),
                            );
                        }
                    }
                    i = j.max(i + 1);
                    continue;
                }
                i += 1;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;
    #[test]
    fn default_config() {
        let pass = vec![
            "@media all {};",
            "@custom-media --foo (max-width: 30em); @media (--foo) {};",
            "@CUSTOM-MEDIA --foo (max-width: 30em); @media (--foo) {};",
            "@custom-media --modern (color), (hover);\n@media (--modern) and (width > 1024px) {}",
            "@media screen and (--bar) {}; @custom-media --bar (max-width: 30em);",
        ];
        let fail = vec![
            reject(
                "@custom-media --foo (max-width: 30em); @media screen and (--bar) {};",
                None,
                "Unknown custom media query \"--bar\"",
                (1, 59, 1, 64),
            ),
            reject(
                "@custom-media --foo (max-width: 30em); @media (--bar) {};",
                None,
                "Unknown custom media query \"--bar\"",
                (1, 48, 1, 53),
            ),
            reject(
                "@media (--bar) {};",
                None,
                "Unknown custom media query \"--bar\"",
                (1, 9, 1, 14),
            ),
            reject_warnings(
                "@media (--bar), (--baz) {};",
                None,
                vec![
                    (
                        "Unknown custom media query \"--bar\"".to_string(),
                        (1, 9, 1, 14),
                    ),
                    (
                        "Unknown custom media query \"--baz\"".to_string(),
                        (1, 18, 1, 23),
                    ),
                ],
            ),
            reject(
                "@media (--bar), (min-width: 40rem) {};",
                None,
                "Unknown custom media query \"--bar\"",
                (1, 9, 1, 14),
            ),
        ];
        Tester::new("no-unknown-custom-media", json!(true), &pass, &fail).test();
    }
}
