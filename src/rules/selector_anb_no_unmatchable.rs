use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::ComplexSelector};

crate::define_stx_lint! {
    /// Reports `An+B` formulas that cannot select any element or column.
    ///
    /// Set to `true`. Checks formulas in `:nth-*()` pseudo-classes, not the selector after `of`.
    /// Does not fix violations.
    ///
    /// Example: `li:nth-child(0) {}` cannot match a child.
    "selector-anb-no-unmatchable", correctness, recommended = true;
    pub struct SelectorAnbNoUnmatchable;
}

fn is_unmatchable_anb(expr: &str) -> bool {
    let before_of = if let Some(idx) = expr.to_ascii_lowercase().find(" of ") {
        &expr[..idx]
    } else {
        expr
    };

    let clean: String = before_of
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();

    if clean.is_empty() || clean == "even" || clean == "odd" {
        return false;
    }

    if let Some(n_pos) = clean.find('n') {
        let a_part = &clean[..n_pos];
        let b_part = &clean[n_pos + 1..];

        let a: i64 = match a_part {
            "" | "+" => 1,
            "-" => -1,
            _ => match a_part.parse::<i64>() {
                Ok(v) => v,
                Err(_) => return false,
            },
        };

        let b: i64 = if b_part.is_empty() {
            0
        } else {
            match b_part.parse::<i64>() {
                Ok(v) => v,
                Err(_) => return false,
            }
        };

        if a == 0 {
            return b <= 0;
        }
        if a < 0 {
            return b <= 0;
        }
        false
    } else {
        match clean.parse::<i64>() {
            Ok(v) => v <= 0,
            Err(_) => false,
        }
    }
}

impl Rule for SelectorAnbNoUnmatchable {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_complex_selector<'a>(&self, sel: &'a ComplexSelector<'a>, ctx: &mut LintContext<'a>) {
        let start = sel.span.start as usize;
        let end = sel.span.end as usize;
        let src = ctx.source;
        if start >= end || end > src.len() {
            return;
        }

        let bytes = src.as_bytes();
        let mut i = start;

        while i < end {
            if bytes[i] == b':' {
                let pseudo_start = i;
                i += 1;
                let name_start = i;
                while i < end && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
                    i += 1;
                }
                let name = &src[name_start..i];
                if (name.eq_ignore_ascii_case("nth-child")
                    || name.eq_ignore_ascii_case("nth-last-child")
                    || name.eq_ignore_ascii_case("nth-of-type")
                    || name.eq_ignore_ascii_case("nth-last-of-type")
                    || name.eq_ignore_ascii_case("nth-col")
                    || name.eq_ignore_ascii_case("nth-last-col"))
                    && i < end
                    && bytes[i] == b'('
                {
                    let arg_start = i + 1;
                    let mut depth = 1usize;
                    i += 1;
                    while i < end && depth > 0 {
                        if bytes[i] == b'(' {
                            depth += 1;
                        } else if bytes[i] == b')' {
                            depth -= 1;
                        }
                        i += 1;
                    }
                    let arg_end = i.saturating_sub(1);
                    if arg_start <= arg_end && arg_end <= end {
                        let arg = &src[arg_start..arg_end];
                        if is_unmatchable_anb(arg) {
                            let full_pseudo = &src[pseudo_start..i];
                            ctx.report(
                                self.name(),
                                Span {
                                    start: pseudo_start as u32,
                                    end: i as u32,
                                },
                                format!("Unmatchable An+B selector \"{full_pseudo}\""),
                            );
                        }
                    }
                    continue;
                }
            }
            i += 1;
        }
    }
}
