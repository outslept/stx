use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::ast::{InterpolableIdent, SimpleBlock, Statement};

pub struct DeclarationBlockNoShorthandPropertyOverrides;

fn shorthand_longhands(shorthand: &str) -> &'static [&'static str] {
    match shorthand {
        "animation" => &[
            "animation-name",
            "animation-duration",
            "animation-timing-function",
            "animation-delay",
            "animation-iteration-count",
            "animation-direction",
            "animation-fill-mode",
            "animation-play-state",
        ],
        "background" => &[
            "background-image",
            "background-position",
            "background-position-x",
            "background-position-y",
            "background-size",
            "background-repeat",
            "background-attachment",
            "background-origin",
            "background-clip",
            "background-color",
        ],
        "border" => &[
            "border-top",
            "border-right",
            "border-bottom",
            "border-left",
            "border-width",
            "border-style",
            "border-color",
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
            "border-image",
            "border-image-source",
            "border-image-slice",
            "border-image-width",
            "border-image-outset",
            "border-image-repeat",
        ],
        "border-top" => &["border-top-width", "border-top-style", "border-top-color"],
        "border-right" => &[
            "border-right-width",
            "border-right-style",
            "border-right-color",
        ],
        "border-bottom" => &[
            "border-bottom-width",
            "border-bottom-style",
            "border-bottom-color",
        ],
        "border-left" => &[
            "border-left-width",
            "border-left-style",
            "border-left-color",
        ],
        "border-width" => &[
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
        ],
        "border-style" => &[
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
        ],
        "border-color" => &[
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
        ],
        "border-radius" => &[
            "border-top-left-radius",
            "border-top-right-radius",
            "border-bottom-right-radius",
            "border-bottom-left-radius",
        ],
        "border-block" => &[
            "border-block-width",
            "border-block-style",
            "border-block-color",
            "border-block-start",
            "border-block-end",
            "border-block-start-width",
            "border-block-start-style",
            "border-block-start-color",
            "border-block-end-width",
            "border-block-end-style",
            "border-block-end-color",
        ],
        "border-inline" => &[
            "border-inline-width",
            "border-inline-style",
            "border-inline-color",
            "border-inline-start",
            "border-inline-end",
            "border-inline-start-width",
            "border-inline-start-style",
            "border-inline-start-color",
            "border-inline-end-width",
            "border-inline-end-style",
            "border-inline-end-color",
        ],
        "columns" => &["column-width", "column-count"],
        "column-rule" => &[
            "column-rule-width",
            "column-rule-style",
            "column-rule-color",
        ],
        "flex" => &["flex-grow", "flex-shrink", "flex-basis"],
        "flex-flow" => &["flex-direction", "flex-wrap"],
        "font" => &[
            "font-style",
            "font-variant",
            "font-weight",
            "font-stretch",
            "font-size",
            "line-height",
            "font-family",
        ],
        "gap" | "grid-gap" => &["row-gap", "column-gap", "grid-row-gap", "grid-column-gap"],
        "grid" => &[
            "grid-template",
            "grid-template-rows",
            "grid-template-columns",
            "grid-template-areas",
            "grid-auto-rows",
            "grid-auto-columns",
            "grid-auto-flow",
        ],
        "grid-area" => &[
            "grid-row",
            "grid-column",
            "grid-row-start",
            "grid-column-start",
            "grid-row-end",
            "grid-column-end",
        ],
        "grid-column" => &["grid-column-start", "grid-column-end"],
        "grid-row" => &["grid-row-start", "grid-row-end"],
        "grid-template" => &[
            "grid-template-columns",
            "grid-template-rows",
            "grid-template-areas",
        ],
        "inset" => &[
            "top",
            "right",
            "bottom",
            "left",
            "inset-block",
            "inset-inline",
            "inset-block-start",
            "inset-block-end",
            "inset-inline-start",
            "inset-inline-end",
        ],
        "inset-block" => &["inset-block-start", "inset-block-end"],
        "inset-inline" => &["inset-inline-start", "inset-inline-end"],
        "list-style" => &["list-style-type", "list-style-position", "list-style-image"],
        "margin" => &["margin-top", "margin-right", "margin-bottom", "margin-left"],
        "margin-block" => &["margin-block-start", "margin-block-end"],
        "margin-inline" => &["margin-inline-start", "margin-inline-end"],
        "mask" => &[
            "mask-image",
            "mask-mode",
            "mask-position",
            "mask-size",
            "mask-repeat",
            "mask-origin",
            "mask-clip",
            "mask-composite",
        ],
        "outline" => &["outline-width", "outline-style", "outline-color"],
        "overflow" => &[
            "overflow-x",
            "overflow-y",
            "overflow-block",
            "overflow-inline",
        ],
        "overscroll-behavior" => &[
            "overscroll-behavior-x",
            "overscroll-behavior-y",
            "overscroll-behavior-block",
            "overscroll-behavior-inline",
        ],
        "padding" => &[
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
        ],
        "padding-block" => &["padding-block-start", "padding-block-end"],
        "padding-inline" => &["padding-inline-start", "padding-inline-end"],
        "place-content" => &["align-content", "justify-content"],
        "place-items" => &["align-items", "justify-items"],
        "place-self" => &["align-self", "justify-self"],
        "scroll-margin" => &[
            "scroll-margin-top",
            "scroll-margin-right",
            "scroll-margin-bottom",
            "scroll-margin-left",
        ],
        "scroll-padding" => &[
            "scroll-padding-top",
            "scroll-padding-right",
            "scroll-padding-bottom",
            "scroll-padding-left",
        ],
        "text-decoration" => &[
            "text-decoration-line",
            "text-decoration-style",
            "text-decoration-color",
            "text-decoration-thickness",
        ],
        "transition" => &[
            "transition-property",
            "transition-duration",
            "transition-timing-function",
            "transition-delay",
            "transition-behavior",
        ],
        _ => &[],
    }
}

fn strip_vendor_prefix(prop: &str) -> (&str, &str) {
    if prop.starts_with('-') {
        if let Some(idx) = prop[1..].find('-') {
            let split_at = idx + 2;
            return (&prop[..split_at], &prop[split_at..]);
        }
    }
    ("", prop)
}

impl Rule for DeclarationBlockNoShorthandPropertyOverrides {
    fn name(&self) -> &'static str {
        "declaration-block-no-shorthand-property-overrides"
    }

    fn check_simple_block<'a>(&self, block: &'a SimpleBlock<'a>, ctx: &mut LintContext<'a>) {
        let mut seen_props: Vec<(&str, String, String)> = Vec::new();

        for stmt in &block.statements {
            if let Statement::Declaration(decl) = stmt {
                if let InterpolableIdent::Literal(ident) = &decl.name {
                    let raw: &str = ident.name;
                    if raw.starts_with("--") {
                        continue;
                    }
                    let lower = raw.to_ascii_lowercase();
                    let (prefix, unprefixed) = strip_vendor_prefix(&lower);
                    let longhands = shorthand_longhands(unprefixed);

                    if !longhands.is_empty() {
                        for (seen_raw, seen_prefix, seen_unprefixed) in &seen_props {
                            if seen_prefix == prefix
                                && longhands.contains(&seen_unprefixed.as_str())
                            {
                                ctx.report(
                                    self.name(),
                                    ident.span,
                                    format!("Unexpected shorthand \"{raw}\" after \"{seen_raw}\""),
                                );
                            }
                        }
                    }

                    seen_props.push((raw, prefix.to_string(), unprefixed.to_string()));
                }
            }
        }
    }
}
