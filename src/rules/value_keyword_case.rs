use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        function_name_case::CaseMode,
        value_utils::{ValueTokenKind, declaration_value_span, tokenize_value},
    },
};
use oxc_css_parser::ast::{Declaration, InterpolableIdent};

pub struct ValueKeywordCase {
    pub mode: CaseMode,
    pub camel_case_svg_keywords: bool,
}

impl Default for ValueKeywordCase {
    fn default() -> Self {
        Self {
            mode: CaseMode::Lower,
            camel_case_svg_keywords: false,
        }
    }
}

const IGNORED_PROPERTIES: &[&str] = &[
    "font-family",
    "font",
    "animation",
    "animation-name",
    "counter-reset",
    "counter-increment",
    "counter-set",
    "grid-area",
    "grid-row",
    "grid-row-start",
    "grid-row-end",
    "grid-column",
    "grid-column-start",
    "grid-column-end",
    "list-style-type",
    "list-style",
    "will-change",
    "container-name",
    "container",
    "view-transition-name",
];

const IGNORED_FUNCTIONS: &[&str] = &[
    "var(",
    "url(",
    "attr(",
    "counter(",
    "counters(",
    "env(",
    "theme(",
];

const CAMEL_CASE_SVG_KEYWORDS: &[&str] = &[
    "currentColor",
    "optimizeSpeed",
    "optimizeLegibility",
    "geometricPrecision",
    "sRGB",
    "linearRGB",
    "crispEdges",
    "visiblePainted",
    "visibleFill",
    "visibleStroke",
    " butt",
];

impl Rule for ValueKeywordCase {
    fn name(&self) -> &'static str {
        "value-keyword-case"
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if prop.starts_with("--")
            || IGNORED_PROPERTIES
                .iter()
                .any(|p| p.eq_ignore_ascii_case(prop))
        {
            return;
        }

        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };

        let tokens = tokenize_value(ctx.source, val_span);
        let mut ignored_depth = 0usize;
        let mut func_stack: Vec<bool> = Vec::new();

        for tok in tokens {
            match tok.kind {
                ValueTokenKind::Function => {
                    let is_ignored = IGNORED_FUNCTIONS
                        .iter()
                        .any(|f| tok.raw.eq_ignore_ascii_case(f));
                    func_stack.push(is_ignored);
                    if is_ignored {
                        ignored_depth += 1;
                    }
                }
                ValueTokenKind::CloseParen => {
                    if let Some(was_ignored) = func_stack.pop() {
                        if was_ignored {
                            ignored_depth = ignored_depth.saturating_sub(1);
                        }
                    }
                }
                ValueTokenKind::Ident if ignored_depth == 0 => {
                    if tok.raw.starts_with("--")
                        || tok.raw.starts_with('$')
                        || tok.raw.starts_with('@')
                    {
                        continue;
                    }

                    if self.camel_case_svg_keywords {
                        if let Some(&svg_kw) = CAMEL_CASE_SVG_KEYWORDS
                            .iter()
                            .find(|k| k.eq_ignore_ascii_case(tok.raw))
                        {
                            if tok.raw != svg_kw {
                                ctx.report_with_fix(
                                    self.name(),
                                    tok.span,
                                    format!("Expected \"{}\" to be \"{}\"", tok.raw, svg_kw),
                                    Some(Fix::single(tok.span, svg_kw.to_string())),
                                );
                            }
                            continue;
                        }
                    }

                    let expected = match self.mode {
                        CaseMode::Lower => tok.raw.to_ascii_lowercase(),
                        CaseMode::Upper => tok.raw.to_ascii_uppercase(),
                    };

                    if tok.raw != expected {
                        ctx.report_with_fix(
                            self.name(),
                            tok.span,
                            format!("Expected \"{}\" to be \"{}\"", tok.raw, expected),
                            Some(Fix::single(tok.span, expected)),
                        );
                    }
                }
                _ => {}
            }
        }
    }
}
