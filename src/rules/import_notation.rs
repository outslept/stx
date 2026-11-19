use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        value_utils::{ValueTokenKind, at_rule_prelude_span, tokenize_value},
    },
};
use oxc_css_parser::{Span, ast::AtRule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportNotationMode {
    Url,
    String,
}

pub struct ImportNotation {
    pub mode: ImportNotationMode,
}

impl Default for ImportNotation {
    fn default() -> Self {
        Self {
            mode: ImportNotationMode::Url,
        }
    }
}

impl Rule for ImportNotation {
    fn name(&self) -> &'static str {
        "import-notation"
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name: &str = at_rule.name.name;
        if !name.eq_ignore_ascii_case("import") {
            return;
        }
        let Some(p_span) = at_rule_prelude_span(ctx.source, at_rule) else {
            return;
        };

        let tokens = tokenize_value(ctx.source, p_span);
        let Some(first) = tokens.first() else {
            return;
        };

        match self.mode {
            ImportNotationMode::Url => {
                if first.kind == ValueTokenKind::String {
                    let expected = format!("url({})", first.raw);
                    ctx.report_with_fix(
                        self.name(),
                        first.span,
                        format!("Expected \"{}\" to be \"{}\"", first.raw, expected),
                        Some(Fix::single(first.span, expected)),
                    );
                }
            }
            ImportNotationMode::String => {
                if first.kind == ValueTokenKind::Function && first.raw.eq_ignore_ascii_case("url(")
                {
                    let start = first.span.start as usize;
                    let end = (p_span.end as usize).min(ctx.source.len());
                    let slice = &ctx.source[start..end];
                    if let Some(close_rel) = slice.find(')') {
                        let url_span = Span {
                            start: first.span.start,
                            end: (start + close_rel + 1) as u32,
                        };
                        let raw_url = &slice[..close_rel + 1];
                        let inner = slice[4..close_rel].trim();
                        if !inner.is_empty() {
                            let expected = if (inner.starts_with('"') && inner.ends_with('"'))
                                || (inner.starts_with('\'') && inner.ends_with('\''))
                            {
                                inner.to_string()
                            } else {
                                format!("\"{}\"", inner)
                            };
                            ctx.report_with_fix(
                                self.name(),
                                url_span,
                                format!("Expected \"{}\" to be \"{}\"", raw_url, expected),
                                Some(Fix::single(url_span, expected)),
                            );
                        }
                    }
                }
            }
        }
    }
}
