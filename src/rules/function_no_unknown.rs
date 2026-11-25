use crate::{
    context::LintContext,
    rules::Rule,
    utils::{OptionMatcher, options_matches, parse_option_matchers},
};
use oxc_css_parser::{Span, ast::ComponentValue};

pub struct FunctionNoUnknown {
    ignore_functions: Vec<OptionMatcher>,
}

impl FunctionNoUnknown {
    pub fn from_config(secondary: Option<&serde_json::Map<String, serde_json::Value>>) -> Self {
        Self {
            ignore_functions: parse_option_matchers(secondary, "ignoreFunctions"),
        }
    }
}

const KNOWN_FUNCTIONS: &[&str] = &[
    "abs",
    "acos",
    "adjust",
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
    "color-stop",
    "color-mix",
    "conic-gradient",
    "contrast",
    "contrast-color",
    "content",
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
    "from",
    "grayscale",
    "hsl",
    "hsla",
    "hue-rotate",
    "hwb",
    "hypot",
    "if",
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
    "media",
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
    "running",
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
    "string",
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
    "to",
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
            if KNOWN_FUNCTIONS.contains(&lower.as_str())
                || options_matches(&self.ignore_functions, func_name)
            {
                return;
            }

            let name_span = Span {
                start: func.span.start,
                end: func.span.start + func_name.len() as u32,
            };

            ctx.report(
                self.name(),
                name_span,
                format!("Unknown function \"{func_name}\""),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::Tester;
    use serde_json::json;

    #[test]
    fn recognizes_legacy_and_paged_media_functions() {
        Tester::new(
            "function-no-unknown",
            json!(true),
            [
                "a { background: -webkit-gradient(linear, 0 50%, 0 100%, from(white), color-stop(0.5, yellow), to(red)); }",
                "a { position: running(); }",
                "h1 { string-set: Chapter content(); }",
                "@top-right { content: string(Chapter); }",
                "a { color: contrast-color(var(--qux)); }",
            ],
            [],
        )
        .test();
    }

    #[test]
    fn ignores_configured_functions() {
        Tester::new(
            "function-no-unknown",
            json!([true, {"ignoreFunctions": ["theme", "/^foo-/", "/^bar$/i"]}]),
            [
                "a { transform: theme(1px); }",
                "a { transform: foo-func(1px); }",
                "a { transform: bar(1px); }",
                "a { transform: BAR(1px); }",
            ],
            [],
        )
        .test();
    }
}
