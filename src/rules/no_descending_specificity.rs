use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::{SimpleBlock, Statement, Stylesheet};

pub struct NoDescendingSpecificity;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Specificity(usize, usize, usize);

fn compute_specificity_and_tail(selector: &str) -> Option<(Specificity, String)> {
    let trimmed = selector.trim();
    if trimmed.is_empty() || trimmed.contains('(') {
        return None;
    }

    let bytes = trimmed.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    let mut a = 0usize;
    let mut b = 0usize;
    let mut c = 0usize;
    let mut at_compound_start = true;

    while i < len {
        if bytes[i].is_ascii_whitespace() || matches!(bytes[i], b'>' | b'+' | b'~') {
            at_compound_start = true;
            i += 1;
            continue;
        }
        if bytes[i] == b'[' {
            at_compound_start = false;
            b += 1;
            i += 1;
            while i < len && bytes[i] != b']' {
                i += 1;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'#' {
            at_compound_start = false;
            a += 1;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'.' {
            at_compound_start = false;
            b += 1;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b':' {
            at_compound_start = false;
            if i + 1 < len && bytes[i + 1] == b':' {
                c += 1;
                i += 2;
            } else {
                let name_start = i + 1;
                let mut j = name_start;
                while j < len
                    && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'-' || bytes[j] == b'_')
                {
                    j += 1;
                }
                let pname = &trimmed[name_start..j];
                if pname.eq_ignore_ascii_case("before")
                    || pname.eq_ignore_ascii_case("after")
                    || pname.eq_ignore_ascii_case("first-line")
                    || pname.eq_ignore_ascii_case("first-letter")
                {
                    c += 1;
                } else {
                    b += 1;
                }
                i = j;
                continue;
            }
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        if at_compound_start && bytes[i].is_ascii_alphabetic() {
            c += 1;
            at_compound_start = false;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }
        at_compound_start = false;
        i += 1;
    }

    let last_compound = trimmed
        .rsplit(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '>' | '+' | '~'))
        .find(|s| !s.is_empty())?;

    let mut base_end = 0usize;
    let lb = last_compound.as_bytes();
    while base_end < lb.len() && lb[base_end] != b':' {
        base_end += 1;
    }
    let tail_key = last_compound[..base_end].to_ascii_lowercase();
    if tail_key.is_empty() {
        return None;
    }

    Some((Specificity(a, b, c), tail_key))
}

fn check_statements_specificity<'a>(
    statements: &'a [Statement<'a>],
    rule_name: &'static str,
    ctx: &mut LintContext<'a>,
) {
    let mut seen: Vec<(String, String, Specificity)> = Vec::new();

    for stmt in statements {
        if let Statement::QualifiedRule(qrule) = stmt {
            for complex in &qrule.selector.selectors {
                let raw = ctx.slice(complex.span).trim();
                let Some((spec, tail_key)) = compute_specificity_and_tail(raw) else {
                    continue;
                };

                if let Some((_, prev_raw, _)) = seen
                    .iter()
                    .find(|(prev_tail, _, prev_spec)| prev_tail == &tail_key && *prev_spec > spec)
                {
                    ctx.report(
                        rule_name,
                        complex.span,
                        format!(
                            "Expected selector \"{raw}\" to come before selector \"{prev_raw}\""
                        ),
                    );
                }

                seen.push((tail_key, raw.to_string(), spec));
            }
        }
    }
}

impl Rule for NoDescendingSpecificity {
    fn name(&self) -> &'static str {
        "no-descending-specificity"
    }

    fn check_stylesheet<'a>(&self, stylesheet: &'a Stylesheet<'a>, ctx: &mut LintContext<'a>) {
        check_statements_specificity(&stylesheet.statements, self.name(), ctx);
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        check_statements_specificity(&block.statements, self.name(), ctx);
    }
}
