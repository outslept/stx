use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        OptionMatcher, ValueTokenKind, at_rule_prelude_span, declaration_value_span,
        options_matches, parse_option_matchers, secondary_has_flag, tokenize_value,
    },
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, Declaration, InterpolableIdent},
};
use serde_json::{Map, Value};

crate::define_stx_lint! {
    /// Disallow length units on zero values in declarations and at-rule preludes.
    ///
    /// Enable with `true`. Set `ignore` to `"custom-properties"` or include it
    /// in the array to skip custom property declarations. `ignoreFunctions`
    /// and `ignorePreludeOfAtRules` accept a string or array of literal names
    /// and `/regex/` patterns. Function names are lowercased before matching;
    /// omit `@` from at-rule names.
    /// Non-length units, CSS math functions such as `calc()`, `flex`,
    /// `line-height`, and the line-height component of `font` are exempt.
    /// Declarations in Less detached rulesets are also skipped.
    ///
    /// Example: `a { margin: 0px; }` is fixed to `a { margin: 0; }`.
    "length-zero-no-unit", style, fix = fix;
    pub struct LengthZeroNoUnit {
        ignore_custom_properties: bool,
        ignore_functions: Vec<OptionMatcher>,
        ignore_at_rules: Vec<OptionMatcher>,
    }
}

impl LengthZeroNoUnit {
    pub fn from_config(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_custom_properties: secondary_has_flag(secondary, "ignore", "custom-properties"),
            ignore_functions: parse_option_matchers(secondary, "ignoreFunctions"),
            ignore_at_rules: parse_option_matchers(secondary, "ignorePreludeOfAtRules"),
        }
    }
}

const LENGTH_UNITS: &[&str] = &[
    "px", "em", "rem", "ex", "ch", "cap", "ic", "lh", "rlh", "vw", "vh", "vi", "vb", "vmin",
    "vmax", "svw", "svh", "svi", "svb", "svmin", "svmax", "lvw", "lvh", "lvi", "lvb", "lvmin",
    "lvmax", "dvw", "dvh", "dvi", "dvb", "dvmin", "dvmax", "cqw", "cqh", "cqi", "cqb", "cqmin",
    "cqmax", "cm", "mm", "q", "in", "pc", "pt",
];

impl LengthZeroNoUnit {
    fn check_span(&self, span: Span, is_font: bool, ctx: &mut LintContext<'_>) {
        let mut ignored_functions = Vec::new();
        let mut skip_font_line_height = false;
        for tok in tokenize_value(ctx.source, span) {
            match tok.kind {
                ValueTokenKind::Function => {
                    let name = tok.raw[..tok.raw.len() - 1]
                        .trim_start_matches(['$', '@'])
                        .to_ascii_lowercase();
                    ignored_functions.push(
                        ignored_functions.last().copied().unwrap_or(false)
                            || matches!(
                                name.as_str(),
                                "calc"
                                    | "calc-size"
                                    | "min"
                                    | "max"
                                    | "clamp"
                                    | "round"
                                    | "mod"
                                    | "rem"
                                    | "abs"
                                    | "sign"
                                    | "sin"
                                    | "cos"
                                    | "tan"
                                    | "asin"
                                    | "acos"
                                    | "atan"
                                    | "atan2"
                                    | "pow"
                                    | "sqrt"
                                    | "hypot"
                                    | "log"
                                    | "exp"
                            )
                            || options_matches(&self.ignore_functions, &name),
                    );
                    continue;
                }
                ValueTokenKind::CloseParen => {
                    ignored_functions.pop();
                    continue;
                }
                ValueTokenKind::Slash if is_font => {
                    skip_font_line_height = true;
                    continue;
                }
                _ => {}
            }
            if skip_font_line_height {
                skip_font_line_height = false;
                if matches!(tok.kind, ValueTokenKind::Dimension { .. }) {
                    continue;
                }
            }
            if ignored_functions.last().copied().unwrap_or(false) {
                continue;
            }
            if let ValueTokenKind::Dimension { num_len } = tok.kind {
                let num_str = &tok.raw[..num_len];
                let unit_str = &tok.raw[num_len..];
                if !LENGTH_UNITS
                    .iter()
                    .any(|u| u.eq_ignore_ascii_case(unit_str))
                {
                    continue;
                }
                if num_str.parse::<f64>().is_ok_and(|value| value == 0.0) {
                    let unit_span = Span {
                        start: tok.span.start + num_len as u32,
                        end: tok.span.end,
                    };
                    ctx.report_with_fix(
                        self.name(),
                        unit_span,
                        "Disallowed unit".to_string(),
                        Some(Fix::single(
                            tok.span,
                            num_str.strip_prefix('.').unwrap_or(num_str),
                        )),
                    );
                }
            }
        }
    }
}

impl Rule for LengthZeroNoUnit {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        if ctx.less_detached_depth > 0 {
            return;
        }
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.name;
        if prop.starts_with("--") && self.ignore_custom_properties
            || prop.eq_ignore_ascii_case("line-height")
            || prop.eq_ignore_ascii_case("flex")
        {
            return;
        }

        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };

        self.check_span(val_span, prop.eq_ignore_ascii_case("font"), ctx);
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        if options_matches(&self.ignore_at_rules, at_rule.name.name) {
            return;
        }
        if let Some(span) = at_rule_prelude_span(ctx.source, at_rule) {
            self.check_span(span, false, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject};
    use serde_json::json;

    #[test]
    fn ignores_computed_and_non_length_zero_values() {
        let accept = [
            "a { padding: calc(0px + 0px); }",
            "a { padding: max(0px, 1px); }",
            "a { padding: calc(var(--space, 0px) + 1px); }",
            "a { line-height: 0px; }",
            "a { flex: 0px; }",
            "a { font: normal 16px/0px Arial; }",
        ];
        let fail = [
            reject(
                "a { font: normal 0px/0px Arial; }",
                None,
                "Disallowed unit",
                (1, 19, 1, 21),
            ),
            reject(
                "a { padding: var(--space, 0px); }",
                None,
                "Disallowed unit",
                (1, 28, 1, 30),
            ),
        ];
        Tester::new("length-zero-no-unit", json!(true), accept, &fail).test();
    }

    #[test]
    fn fix_keeps_the_number_but_drops_a_leading_dot() {
        let fail = [
            reject(
                "a { top: 0.000px; }",
                "a { top: 0.000; }",
                "Disallowed unit",
                (1, 15, 1, 17),
            ),
            reject(
                "a { top: .0px; }",
                "a { top: 0; }",
                "Disallowed unit",
                (1, 12, 1, 14),
            ),
        ];
        Tester::new("length-zero-no-unit", json!(true), [], &fail)
            .with_fix(true)
            .test();
    }
}
