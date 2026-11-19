use crate::{context::LintContext, fix::Fix, rules::Rule};
use oxc_css_parser::{Span, ast::SelectorList};

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

impl Rule for SelectorNotNotation {
    fn name(&self) -> &'static str {
        "selector-not-notation"
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        let start = list.span.start as usize;
        let end = (list.span.end as usize).min(ctx.source.len());
        if start >= end {
            return;
        }
        let slice = &ctx.source[start..end];

        match self.mode {
            SelectorNotMode::Simple => {
                let mut offset = 0usize;
                while let Some(rel) = slice[offset..].to_ascii_lowercase().find(":not(") {
                    let not_start = offset + rel;
                    let arg_start = not_start + 5;
                    let mut depth = 1usize;
                    let mut j = arg_start;
                    let mut top_commas = Vec::new();
                    let bytes = slice.as_bytes();

                    while j < slice.len() && depth > 0 {
                        if bytes[j] == b'(' {
                            depth += 1;
                        } else if bytes[j] == b')' {
                            depth -= 1;
                        } else if bytes[j] == b',' && depth == 1 {
                            top_commas.push(j);
                        }
                        j += 1;
                    }

                    if depth == 0 && !top_commas.is_empty() {
                        let not_end = j;
                        let args_str = &slice[arg_start..not_end - 1];
                        let parts: Vec<String> = args_str
                            .split(',')
                            .map(|p| format!(":not({})", p.trim()))
                            .collect();
                        let sp = Span {
                            start: (start + not_start) as u32,
                            end: (start + not_end) as u32,
                        };
                        ctx.report_with_fix(
                            self.name(),
                            sp,
                            "Expected simple :not() pseudo-class notation".to_string(),
                            Some(Fix::single(sp, parts.join(""))),
                        );
                    }
                    offset = j.max(not_start + 5);
                }
            }
            SelectorNotMode::Complex => {}
        }
    }
}
