use crate::{
    context::LintContext,
    rules::Rule,
    utils::{PatternMatcher, ValueTokenKind, tokenize_value},
};
use oxc_css_parser::{
    Span,
    ast::{AtRule, AtRulePrelude, ComponentValue, ImportPreludeHref, ImportPreludeLayer},
};

crate::define_stx_lint! {
    /// Require named cascade layers to match a regular expression.
    ///
    /// The primary option is a pattern string. STX checks each name in an
    /// `@layer` list and each name in `@import ... layer(...)`. A dotted name
    /// is checked as a whole. Anonymous layers are ignored.
    ///
    /// With `"/^[a-z][a-z0-9-]*$/"`:
    ///
    /// ```css
    /// @layer components; /* accepted */
    /// @layer Bad, components.buttons; /* reported */
    /// ```
    "layer-name-pattern", style;
    #[derive(Default)]
    pub struct LayerNamePattern {
        /// Pattern applied to each layer name, including any dots.
        pub matcher: PatternMatcher,
    }
}

impl Rule for LayerNamePattern {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_at_rule<'a>(&self, at_rule: &'a AtRule<'a>, ctx: &mut LintContext<'a>) {
        let name = at_rule.name.raw;
        match &at_rule.prelude {
            Some(AtRulePrelude::Layer(layers)) if name.eq_ignore_ascii_case("layer") => {
                for layer in &layers.names {
                    self.check_name(layer.span, ctx);
                }
            }
            Some(AtRulePrelude::Import(import)) if name.eq_ignore_ascii_case("import") => {
                let start = match (&import.href, &import.layer) {
                    (_, Some(ImportPreludeLayer::WithName(layer))) => {
                        for arg in &layer.args {
                            if let ComponentValue::LayerName(name) = arg {
                                self.check_name(name.span, ctx);
                            }
                        }
                        layer.span.end
                    }
                    (ImportPreludeHref::Url(url), _) => url.span.end,
                    _ => import.span.start,
                };
                self.check_import_functions(
                    Span {
                        start,
                        end: import.span.end,
                    },
                    ctx,
                );
            }
            _ => {}
        }
    }
}

impl LayerNamePattern {
    fn check_name(&self, span: Span, ctx: &mut LintContext<'_>) {
        let name = ctx.slice(span);
        if !self.matcher.is_match(name) {
            ctx.report(
                self.name(),
                span,
                format!(
                    "Expected \"{name}\" to match pattern \"{}\"",
                    self.matcher.raw_pattern
                ),
            );
        }
    }

    fn check_import_functions(&self, span: Span, ctx: &mut LintContext<'_>) {
        let mut inside_layer = Vec::new();
        let mut word: Option<Span> = None;
        for token in tokenize_value(ctx.source, span) {
            let is_word = !matches!(
                token.kind,
                ValueTokenKind::Function
                    | ValueTokenKind::String
                    | ValueTokenKind::CloseParen
                    | ValueTokenKind::Comma
                    | ValueTokenKind::Colon
                    | ValueTokenKind::Slash
            ) && token.raw != "(";

            if is_word && inside_layer.last() == Some(&true) {
                if let Some(word) = &mut word
                    && word.end == token.span.start
                {
                    word.end = token.span.end;
                } else if let Some(previous) = word.replace(token.span) {
                    self.check_name(previous, ctx);
                }
                continue;
            }

            if let Some(word) = word.take() {
                self.check_name(word, ctx);
            }
            match token.kind {
                ValueTokenKind::Function => {
                    inside_layer.push(token.raw.eq_ignore_ascii_case("layer("))
                }
                ValueTokenKind::Other if token.raw == "(" => inside_layer.push(false),
                ValueTokenKind::CloseParen => {
                    inside_layer.pop();
                }
                _ => {}
            }
        }
        if let Some(word) = word {
            self.check_name(word, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject, reject_warnings};
    use serde_json::json;

    const RULE_NAME: &str = "layer-name-pattern";
    const PATTERN: &str = "/^[a-z][a-z0-9-]*$/";

    fn expected(name: &str) -> String {
        format!("Expected \"{name}\" to match pattern \"{PATTERN}\"")
    }

    #[test]
    fn matches_whole_dotted_names_and_individual_list_items() {
        let pass = ["@layer {}", "@layer foo, /* between, */ bar;"];
        let fail = [reject_warnings(
            "@layer\n  foo.bar,\n  /* skip, */ Bad;",
            None,
            vec![
                (expected("foo.bar"), (2, 3, 2, 10)),
                (expected("Bad"), (3, 15, 3, 18)),
            ],
        )];
        Tester::new(RULE_NAME, json!(PATTERN), pass, fail).test();
    }

    #[test]
    fn checks_only_names_in_import_layer_functions() {
        let pass = [
            "@import \"layer(Bad).css\" layer supports(display: flex) screen;",
            "@import \"a.css\" layer(foo, bar)",
        ];
        let fail = [
            reject(
                "@import \"foo.css\" layer(Bar)",
                None,
                expected("Bar"),
                (1, 25, 1, 28),
            ),
            reject_warnings(
                "@import url(foo.css) LaYeR(\n  foo, /* between, ) */\n  Bar.Baz, Qux\n) screen;",
                None,
                vec![
                    (expected("Bar.Baz"), (3, 3, 3, 10)),
                    (expected("Qux"), (3, 12, 3, 15)),
                ],
            ),
        ];
        Tester::new(RULE_NAME, json!(PATTERN), pass, fail).test();
    }
}
