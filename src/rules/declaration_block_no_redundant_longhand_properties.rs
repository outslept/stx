use crate::{
    context::LintContext,
    rules::{Rule, value_utils::scan_direct_declarations},
};
use oxc_css_parser::ast::SimpleBlock;

pub struct DeclarationBlockNoRedundantLonghandProperties;

const SHORTHAND_GROUPS: &[(&str, &[&str])] = &[
    (
        "margin",
        &["margin-top", "margin-right", "margin-bottom", "margin-left"],
    ),
    (
        "padding",
        &[
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
        ],
    ),
    (
        "border-width",
        &[
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
        ],
    ),
    (
        "border-style",
        &[
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
        ],
    ),
    (
        "border-color",
        &[
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
        ],
    ),
    (
        "border-radius",
        &[
            "border-top-left-radius",
            "border-top-right-radius",
            "border-bottom-right-radius",
            "border-bottom-left-radius",
        ],
    ),
    ("inset", &["top", "right", "bottom", "left"]),
    ("overflow", &["overflow-x", "overflow-y"]),
    (
        "overscroll-behavior",
        &["overscroll-behavior-x", "overscroll-behavior-y"],
    ),
    ("gap", &["row-gap", "column-gap"]),
    (
        "scroll-margin",
        &[
            "scroll-margin-top",
            "scroll-margin-right",
            "scroll-margin-bottom",
            "scroll-margin-left",
        ],
    ),
    (
        "scroll-padding",
        &[
            "scroll-padding-top",
            "scroll-padding-right",
            "scroll-padding-bottom",
            "scroll-padding-left",
        ],
    ),
];

impl Rule for DeclarationBlockNoRedundantLonghandProperties {
    fn name(&self) -> &'static str {
        "declaration-block-no-redundant-longhand-properties"
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        let decls = scan_direct_declarations(ctx.source, block.span);
        if decls.len() < 2 {
            return;
        }

        for &(shorthand, longhands) in SHORTHAND_GROUPS {
            let mut seen: Vec<(&str, bool)> = Vec::new();
            for d in &decls {
                let lower = d.prop.to_ascii_lowercase();
                if lower == shorthand {
                    seen.clear();
                    continue;
                }
                if let Some(&matched) = longhands.iter().find(|&&lh| lh == lower) {
                    if !seen.iter().any(|&(s, _)| s == matched) {
                        seen.push((matched, d.important));
                    }
                    if seen.len() == longhands.len() {
                        let first_imp = seen[0].1;
                        if seen.iter().all(|&(_, imp)| imp == first_imp) {
                            ctx.report(
                                self.name(),
                                d.span,
                                format!("Expected shorthand property \"{}\"", shorthand),
                            );
                        }
                        seen.clear();
                    }
                }
            }
        }
    }
}
