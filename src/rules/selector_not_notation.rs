use crate::{context::LintContext, fix::Fix, rules::Rule, utils::is_standard_syntax_selector};
use oxc_css_parser::{
    Span,
    ast::{QualifiedRule, UnknownQualifiedRule},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorNotMode {
    Complex,
    Simple,
}

pub struct SelectorNotNotation {
    pub mode: SelectorNotMode,
}

impl Default for SelectorNotNotation {
    fn default() -> Self {
        Self {
            mode: SelectorNotMode::Complex,
        }
    }
}

/// A `:not(...)` found in a selector, with byte offsets into it.
struct NotPseudo<'a> {
    start: usize,
    end: usize,
    name: &'a str,
    /// Top-level comma-separated arguments, untrimmed.
    args: Vec<&'a str>,
}

fn skip_comment_or_string(bytes: &[u8], i: usize) -> Option<usize> {
    match bytes[i] {
        b'/' if bytes.get(i + 1) == Some(&b'*') => {
            let mut j = i + 2;
            while j + 1 < bytes.len() && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
                j += 1;
            }
            Some((j + 2).min(bytes.len()))
        }
        q @ (b'"' | b'\'') => {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != q {
                j += if bytes[j] == b'\\' { 2 } else { 1 };
            }
            Some((j + 1).min(bytes.len()))
        }
        _ => None,
    }
}

/// End of the group opened at `open`, plus its top-level comma positions.
fn group_end(bytes: &[u8], open: usize) -> Option<(usize, Vec<usize>)> {
    let (mut depth, mut i, mut commas) = (0usize, open, Vec::new());
    while i < bytes.len() {
        if let Some(next) = skip_comment_or_string(bytes, i) {
            i = next;
            continue;
        }
        match bytes[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i + 1, commas));
                }
            }
            b',' if depth == 1 => commas.push(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn find_nots(selector: &str) -> Vec<NotPseudo<'_>> {
    let bytes = selector.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(next) = skip_comment_or_string(bytes, i) {
            i = next;
            continue;
        }
        let is_not = bytes[i] == b':'
            && bytes.get(i + 1) != Some(&b':')
            && (i == 0 || bytes[i - 1] != b':')
            && selector
                .get(i + 1..i + 5)
                .is_some_and(|n| n.eq_ignore_ascii_case("not("));
        if is_not {
            if let Some((end, commas)) = group_end(bytes, i + 4) {
                let mut args = Vec::new();
                let mut from = i + 5;
                for comma in commas {
                    args.push(&selector[from..comma]);
                    from = comma + 1;
                }
                args.push(&selector[from..end - 1]);
                out.push(NotPseudo {
                    start: i,
                    end,
                    name: &selector[i..i + 4],
                    args,
                });
            }
            i += 5;
            continue;
        }
        i += 1;
    }
    out
}

/// Number of postcss-selector-parser nodes in a selector, and whether a lone node is a simple
/// selector other than `:not()`.
fn nodes(selector: &str) -> (usize, bool) {
    let bytes = selector.trim().as_bytes();
    let (mut count, mut i, mut simple) = (0, 0, false);
    let mut pending_combinator = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() || matches!(b, b'>' | b'+' | b'~') {
            pending_combinator = true;
            i += 1;
            continue;
        }
        if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            i = skip_comment_or_string(bytes, i).unwrap_or(bytes.len());
            count += 1;
            continue;
        }
        if pending_combinator {
            count += 1;
            pending_combinator = false;
        }
        let start = i;
        i += match b {
            b':' if bytes.get(i + 1) == Some(&b':') => 2,
            b'.' | b'#' | b':' => 1,
            _ => 0,
        };
        if b == b'[' {
            i = group_end(bytes, i).map_or(bytes.len(), |(end, _)| end);
        } else if b == b'*' || b == b'&' {
            i += 1;
        } else {
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || matches!(bytes[i], b'-' | b'_' | b'\\')
                    || bytes[i] >= 0x80)
            {
                i += if bytes[i] == b'\\' { 2 } else { 1 };
            }
            if bytes.get(i) == Some(&b'(') {
                i = group_end(bytes, i).map_or(bytes.len(), |(end, _)| end);
            }
            i = i.max(start + 1);
        }
        let piece = String::from_utf8_lossy(&bytes[start..i.min(bytes.len())]).to_ascii_lowercase();
        let pseudo_element = piece.starts_with("::")
            || ["before", "after", "first-line", "first-letter"]
                .iter()
                .any(|name| piece.strip_prefix(':') == Some(name));
        simple = !pseudo_element && !piece.starts_with(":not(") && b != b'&';
        count += 1;
    }
    (count, count == 1 && simple)
}

fn not_text(name: &str, args: &[&str]) -> String {
    format!("{name}({})", args.join(", "))
}

impl SelectorNotNotation {
    fn check_selector(&self, selector: &str, base: u32, ctx: &mut LintContext<'_>) {
        if !is_standard_syntax_selector(selector) {
            return;
        }
        let nots = find_nots(selector);
        let span = |start: usize, end: usize| Span {
            start: base + start as u32,
            end: base + end as u32,
        };
        for (n, not) in nots.iter().enumerate() {
            let report_span = span(not.start, not.end);
            let fix = match self.mode {
                SelectorNotMode::Complex => {
                    let Some(prev) = n
                        .checked_sub(1)
                        .map(|p| &nots[p])
                        .filter(|p| p.end == not.start)
                    else {
                        continue;
                    };
                    // Merge the whole run of adjacent `:not()`s into `prev`, once.
                    let first_in_run = n < 2 || nots[n - 2].end != prev.start;
                    first_in_run.then(|| {
                        let mut run = vec![prev];
                        while let Some(next) = nots
                            .get(n - 1 + run.len())
                            .filter(|x| x.end > 0 && x.start == run.last().unwrap().end)
                        {
                            run.push(next);
                        }
                        let selectors: Vec<&str> = run
                            .iter()
                            .flat_map(|x| x.args.iter().map(|a| a.trim()))
                            .filter(|a| !a.is_empty())
                            .collect();
                        let text = if selectors.is_empty() {
                            format!("{}()", prev.name)
                        } else {
                            not_text(prev.name, &selectors)
                        };
                        Fix::single(span(prev.start, run.last().unwrap().end), text)
                    })
                }
                SelectorNotMode::Simple => {
                    let counts: Vec<(usize, bool)> = not.args.iter().map(|a| nodes(a)).collect();
                    let simple = counts.len() == 1 && (counts[0].0 == 0 || counts[0].1);
                    if simple {
                        continue;
                    }
                    let fixable = counts.len() > 1
                        && (counts.iter().any(|c| c.0 == 0) || counts.iter().all(|c| c.0 <= 1));
                    fixable.then(|| {
                        let non_empty: Vec<&str> = not
                            .args
                            .iter()
                            .map(|a| a.trim())
                            .filter(|a| !a.is_empty())
                            .collect();
                        let first = non_empty.first().copied().unwrap_or("");
                        let mut last = not;
                        while let Some(next) = nots.iter().find(|x| x.start == last.end) {
                            last = next;
                        }
                        let clones: String = non_empty
                            .iter()
                            .skip(1)
                            .map(|sel| format!("{}({sel})", last.name))
                            .collect();
                        let mut fix = Fix::single(report_span, format!("{}({first})", not.name));
                        if last.start == not.start {
                            fix.edits[0].replacement.push_str(&clones);
                        } else {
                            fix.edits.push(crate::fix::Edit {
                                span: span(last.end, last.end),
                                replacement: clones,
                            });
                        }
                        fix
                    })
                }
            };
            ctx.report_with_fix(
                self.name(),
                report_span,
                format!(
                    "Expected {} :not() pseudo-class notation",
                    match self.mode {
                        SelectorNotMode::Simple => "simple",
                        SelectorNotMode::Complex => "complex",
                    }
                ),
                fix,
            );
        }
    }
}

impl Rule for SelectorNotNotation {
    fn name(&self) -> &'static str {
        "selector-not-notation"
    }

    fn check_qualified_rule<'a>(&self, rule: &'a QualifiedRule<'a>, ctx: &mut LintContext<'a>) {
        self.check_selector(ctx.slice(rule.selector.span), rule.selector.span.start, ctx);
    }

    fn check_unknown_qualified_rule<'a>(
        &self,
        rule: &'a UnknownQualifiedRule<'a>,
        ctx: &mut LintContext<'a>,
    ) {
        self.check_selector(ctx.slice(rule.prelude.span), rule.prelude.span.start, ctx);
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn simple_notation() {
        let msg = "Expected simple :not() pseudo-class notation";
        let pass = [
            ":not( a ) {}",
            ":not(*) {}",
            ":not(:link) {}",
            ":not([title]) {}",
        ];
        let fail = [
            reject(":not(a, div) {}", ":not(a):not(div) {}", msg, (1, 1, 1, 13)),
            reject(
                ":not(.bar, .baz) .qux :not(.foo) {}",
                ":not(.bar):not(.baz) .qux :not(.foo) {}",
                msg,
                (1, 1, 1, 17),
            ),
            reject(":not(a.foo) {}", None, msg, (1, 1, 1, 12)),
            reject(":not(::before) {}", None, msg, (1, 1, 1, 15)),
        ];
        Tester::new("selector-not-notation", json!("simple"), pass, fail)
            .with_fix(true)
            .test();
    }

    #[test]
    fn complex_notation() {
        let msg = "Expected complex :not() pseudo-class notation";
        let pass = [":not(a, div) {}", ":not(a).foo:not(:empty) {}"];
        let fail = [
            reject(
                ":not( .foo ,:hover ):not(a,div) {}",
                ":not(.foo, :hover, a, div) {}",
                msg,
                (1, 21, 1, 32),
            ),
            reject(":not():not(a) {}", ":not(a) {}", msg, (1, 7, 1, 14)),
        ];
        Tester::new("selector-not-notation", json!("complex"), pass, fail)
            .with_fix(true)
            .test();
    }
}
