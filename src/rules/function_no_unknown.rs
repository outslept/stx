use crate::{context::LintContext, rules::Rule};
use oxc_css_parser::{Span, ast::ComponentValue};

pub struct FunctionNoUnknown;

const KNOWN_FUNCTIONS: &[&str] = &[
    "abs",
    "acos",
    "anchor",
    "anchor-size",
    "annotation",
    "asin",
    "atan",
    "atan2",
    "attr",
    "blur",
    "brightness",
    "calc",
    "calc-size",
    "character-variant",
    "circle",
    "clamp",
    "color",
    "color-mix",
    "conic-gradient",
    "contrast",
    "cos",
    "counter",
    "counters",
    "cross-fade",
    "cubic-bezier",
    "device-cmyk",
    "drop-shadow",
    "element",
    "ellipse",
    "env",
    "exp",
    "fit-content",
    "format",
    "grayscale",
    "hsl",
    "hsla",
    "hue-rotate",
    "hwb",
    "hypot",
    "image",
    "image-set",
    "inset",
    "invert",
    "lab",
    "layer",
    "lch",
    "leader",
    "light-dark",
    "linear",
    "linear-gradient",
    "local",
    "log",
    "matrix",
    "matrix3d",
    "max",
    "min",
    "minmax",
    "mod",
    "oklab",
    "oklch",
    "opacity",
    "ornaments",
    "paint",
    "palette-mix",
    "path",
    "perspective",
    "polygon",
    "pow",
    "radial-gradient",
    "ray",
    "rect",
    "rem",
    "repeat",
    "repeating-conic-gradient",
    "repeating-linear-gradient",
    "repeating-radial-gradient",
    "rgb",
    "rgba",
    "rotate",
    "rotate3d",
    "rotatex",
    "rotatey",
    "rotatez",
    "round",
    "saturate",
    "scale",
    "scale3d",
    "scalex",
    "scaley",
    "scalez",
    "scroll",
    "sepia",
    "sign",
    "sin",
    "skew",
    "skewx",
    "skewy",
    "sqrt",
    "steps",
    "styleset",
    "stylistic",
    "supports",
    "swash",
    "symbols",
    "tan",
    "target-counter",
    "target-counters",
    "target-text",
    "tech",
    "translate",
    "translate3d",
    "translatex",
    "translatey",
    "translatez",
    "type",
    "url",
    "var",
    "view",
    "xywh",
];

impl Rule for FunctionNoUnknown {
    fn name(&self) -> &'static str {
        "function-no-unknown"
    }

    fn check_component_value<'a>(&self, val: &'a ComponentValue<'a>, ctx: &mut LintContext<'a>) {
        if let ComponentValue::Function(func) = val {
            let raw = ctx.slice(func.span);
            let Some(paren_idx) = raw.find('(') else {
                return;
            };
            let func_name = raw[..paren_idx].trim();
            if func_name.is_empty() || func_name.starts_with('-') || func_name.contains('.') {
                return;
            }

            let lower = func_name.to_ascii_lowercase();
            if KNOWN_FUNCTIONS.contains(&lower.as_str()) {
                return;
            }

            let name_span = Span {
                start: func.span.start,
                end: func.span.start + func_name.len() as u32,
            };

            ctx.report(
                self.name(),
                name_span,
                format!("Unexpected unknown function \"{func_name}\""),
            );
        }
    }
}
