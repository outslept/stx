use crate::{
    context::LintContext,
    fix::Fix,
    rules::{
        Rule,
        function_name_case::CaseMode,
        value_utils::{SelectorItemKind, scan_selector_items},
    },
};
use oxc_css_parser::ast::SelectorList;

pub struct SelectorTypeCase {
    pub mode: CaseMode,
}

impl Default for SelectorTypeCase {
    fn default() -> Self {
        Self {
            mode: CaseMode::Lower,
        }
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

impl Rule for SelectorTypeCase {
    fn name(&self) -> &'static str {
        "selector-type-case"
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        if ctx.in_keyframes() {
            return;
        }
        let items = scan_selector_items(ctx.source, list.span);
        for item in items {
            if item.kind != SelectorItemKind::Type {
                continue;
            }
            if CAMEL_CASE_SVG_TAGS.contains(&item.name) {
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
