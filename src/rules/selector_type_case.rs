use crate::{
    context::LintContext,
    fix::Fix,
    rules::{Rule, function_name_case::CaseMode},
    utils::{OptionMatcher, SelectorItemKind, options_matches, scan_selector_items},
};
use oxc_css_parser::ast::SelectorList;

crate::define_stx_lint! {
    /// Enforces the case of type selectors.
    ///
    /// Set to `"lower"` or `"upper"`. `ignoreTypes` accepts names and `/regex/`
    /// patterns. Canonical camel-case SVG tags, keyframes, and `An+B` formulas are
    /// exempt. Auto-fix converts the reported type name to the configured case.
    ///
    /// Example: `"lower"` changes `:nth-child(2N of DIV) {}` to `:nth-child(2N of div) {}`.
    "selector-type-case", style, fix = fix;
    pub struct SelectorTypeCase {
        pub mode: CaseMode,
        pub ignore_types: Vec<OptionMatcher>,
    }
}

const CAMEL_CASE_SVG_TAGS: &[&str] = &[
    "altGlyph",
    "altGlyphDef",
    "altGlyphItem",
    "animateColor",
    "animateMotion",
    "animateTransform",
    "clipPath",
    "feBlend",
    "feColorMatrix",
    "feComponentTransfer",
    "feComposite",
    "feConvolveMatrix",
    "feDiffuseLighting",
    "feDisplacementMap",
    "feDistantLight",
    "feDropShadow",
    "feFlood",
    "feFuncA",
    "feFuncB",
    "feFuncG",
    "feFuncR",
    "feGaussianBlur",
    "feImage",
    "feMerge",
    "feMergeNode",
    "feMorphology",
    "feOffset",
    "fePointLight",
    "feSpecularLighting",
    "feSpotLight",
    "feTile",
    "feTurbulence",
    "foreignObject",
    "glyphRef",
    "linearGradient",
    "radialGradient",
    "textPath",
];

fn anb_end(args: &str) -> usize {
    let bytes = args.as_bytes();
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if depth == 0
            && i > 0
            && bytes[i..]
                .get(..2)
                .is_some_and(|word| word.eq_ignore_ascii_case(b"of"))
            && bytes[i - 1].is_ascii_whitespace()
            && bytes.get(i + 2).is_some_and(u8::is_ascii_whitespace)
        {
            return i + 2;
        }
        match bytes[i] {
            b'(' => depth += 1,
            b')' if depth == 0 => return i,
            b')' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}

fn is_nth_anb_item(source: &str, position: usize) -> bool {
    let lower = source[..position].to_ascii_lowercase();
    let Some((start, needle)) = [
        ":nth-child(",
        ":nth-last-child(",
        ":nth-of-type(",
        ":nth-last-of-type(",
    ]
    .iter()
    .filter_map(|needle| lower.rfind(needle).map(|start| (start, *needle)))
    .max_by_key(|(start, _)| *start) else {
        return false;
    };
    let args_start = start + needle.len();
    position < args_start + anb_end(&source[args_start..])
}

impl Rule for SelectorTypeCase {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        if ctx.in_keyframes() {
            return;
        }
        if !crate::utils::is_standard_syntax_selector(ctx.slice(list.span)) {
            return;
        }
        let items = scan_selector_items(ctx.source, list.span);
        for item in items {
            // `&-bar` suffixes and `%placeholder`s are not type selectors.
            let prev = ctx.source.as_bytes()[..item.span.start as usize].last();
            if item.kind != SelectorItemKind::Type
                || is_nth_anb_item(ctx.source, item.span.start as usize)
                || matches!(prev, Some(b'&' | b'%'))
            {
                continue;
            }
            if CAMEL_CASE_SVG_TAGS.contains(&item.name)
                || options_matches(&self.ignore_types, item.name)
            {
                continue;
            }
            let expected = match self.mode {
                CaseMode::Lower => item.name.to_ascii_lowercase(),
                CaseMode::Upper => item.name.to_ascii_uppercase(),
            };
            if item.name != expected {
                ctx.report_with_fix(
                    self.name(),
                    item.span,
                    format!("Expected \"{}\" to be \"{}\"", item.name, expected),
                    Some(Fix::single(item.span, expected)),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_nth_anb_item;
    use crate::tester::Tester;
    use serde_json::json;

    #[test]
    fn ignores_anb_keywords_without_ignoring_of_selectors() {
        let selector = ":nth-child(2N of DIV)";
        assert!(is_nth_anb_item(selector, 12));
        assert!(!is_nth_anb_item(selector, 17));
        let selector = ":nth-child(2N\nOF DIV)";
        assert!(is_nth_anb_item(selector, selector.find('N').unwrap()));
        assert!(!is_nth_anb_item(selector, selector.find("DIV").unwrap()));
        Tester::new(
            "selector-type-case",
            json!("lower"),
            [":nth-child(ODD), :nth-child(2N + 1) {}"],
            [],
        )
        .test();
    }

    #[test]
    fn ignores_non_standard_scss_type_selectors() {
        Tester::new(
            "selector-type-case",
            json!("upper"),
            [
                ".foo { &-bar {} }",
                "#{$variable} {}",
                "%foo {}",
                ".foo, %foo {}",
            ],
            [],
        )
        .with_custom_syntax("postcss-scss")
        .test();
    }

    #[test]
    fn ignore_types() {
        Tester::new(
            "selector-type-case",
            json!(["lower", {"ignoreTypes": ["/^my/"]}]),
            ["myParentClass { color: pink; }"],
            [],
        )
        .test();
    }
}
