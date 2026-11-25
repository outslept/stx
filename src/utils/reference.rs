#![allow(dead_code)]

/// Case-sensitive membership check in a slice.
#[inline]
pub fn in_set(set: &[&str], value: &str) -> bool {
    set.contains(&value)
}

/// ASCII case-insensitive membership check in a slice.
#[inline]
pub fn in_set_ignore_case(set: &[&str], value: &str) -> bool {
    set.iter().any(|item| item.eq_ignore_ascii_case(value))
}

pub const LENGTH_UNITS: &[&str] = &[
    // Font-relative length units
    "cap", "ch", "em", "ex", "ic", "lh", "rcap", "rch", "rem", "rex", "ric", "rlh",
    // Viewport-percentage lengths
    "dvb", "dvh", "dvi", "dvmax", "dvmin", "dvw", "lvb", "lvh", "lvi", "lvmax", "lvmin", "lvw",
    "svb", "svh", "svi", "svmax", "svmin", "svw", "vb", "vh", "vi", "vw", "vmin", "vmax", "vm",
    // Absolute length units
    "px", "mm", "cm", "in", "pt", "pc", "q", "mozmm", // Flexible length units
    "fr",    // Container query units
    "cqw", "cqh", "cqi", "cqb", "cqmin", "cqmax",
];

pub const RESOLUTION_UNITS: &[&str] = &["dpi", "dpcm", "dppx", "x"];

/// Returns `(horizontal_inline, vertical_inline)` flow-relative equivalents for a physical unit.
pub fn physical_to_flow_relative_unit(unit: &str) -> Option<(&'static str, &'static str)> {
    match unit {
        "vw" => Some(("vi", "vb")),
        "vh" => Some(("vb", "vi")),
        "svw" => Some(("svi", "svb")),
        "svh" => Some(("svb", "svi")),
        "lvw" => Some(("lvi", "lvb")),
        "lvh" => Some(("lvb", "lvi")),
        "dvw" => Some(("dvi", "dvb")),
        "dvh" => Some(("dvb", "dvi")),
        "cqw" => Some(("cqi", "cqb")),
        "cqh" => Some(("cqb", "cqi")),
        _ => None,
    }
}

pub const FLOW_RELATIVE_UNITS: &[&str] = &[
    "vi", "vb", "svi", "svb", "lvi", "lvb", "dvi", "dvb", "cqi", "cqb",
];

pub const UNITS: &[&str] = &[
    // Font-relative length units
    "cap", "ch", "em", "ex", "ic", "lh", "rcap", "rch", "rem", "rex", "ric", "rlh",
    // Viewport-percentage lengths
    "dvb", "dvh", "dvi", "dvmax", "dvmin", "dvw", "lvb", "lvh", "lvi", "lvmax", "lvmin", "lvw",
    "svb", "svh", "svi", "svmax", "svmin", "svw", "vb", "vh", "vi", "vw", "vmin", "vmax", "vm",
    // Absolute length units
    "px", "mm", "cm", "in", "pt", "pc", "q", "mozmm", // Flexible length units
    "fr",    // Container query units
    "cqw", "cqh", "cqi", "cqb", "cqmin", "cqmax", // Resolution units
    "dpi", "dpcm", "dppx", "x", // Relative length units
    "%", // Time length units
    "s", "ms", // Angle
    "deg", "grad", "turn", "rad", // Frequency
    "Hz", "kHz",
];

pub const DEPRECATED_AT_KEYWORDS: &[&str] = &["document", "nest", "viewport"];

pub const NESTING_SUPPORTED_AT_KEYWORDS: &[&str] = &[
    "apply",
    "container",
    "layer",
    "media",
    "scope",
    "starting-style",
    "supports",
];

pub const DECLARATION_CONTAINING_AT_KEYWORDS: &[&str] = &["mixin", "scope"];

pub const PAGE_MARGIN_AT_KEYWORDS: &[&str] = &[
    "top-left-corner",
    "top-left",
    "top-center",
    "top-right",
    "top-right-corner",
    "bottom-left-corner",
    "bottom-left",
    "bottom-center",
    "bottom-right",
    "bottom-right-corner",
    "left-top",
    "left-middle",
    "left-bottom",
    "right-top",
    "right-middle",
    "right-bottom",
];

pub const FONT_FEATURE_VALUE_TYPES: &[&str] = &[
    "annotation",
    "character-variant",
    "historical-forms",
    "ornaments",
    "styleset",
    "stylistic",
    "swash",
];

pub const AT_KEYWORDS: &[&str] = &[
    // deprecatedAtKeywords
    "document",
    "nest",
    "viewport",
    // nestingSupportedAtKeywords
    "apply",
    "container",
    "layer",
    "media",
    "scope",
    "starting-style",
    "supports",
    // pageMarginAtKeywords
    "top-left-corner",
    "top-left",
    "top-center",
    "top-right",
    "top-right-corner",
    "bottom-left-corner",
    "bottom-left",
    "bottom-center",
    "bottom-right",
    "bottom-right-corner",
    "left-top",
    "left-middle",
    "left-bottom",
    "right-top",
    "right-middle",
    "right-bottom",
    // fontFeatureValueTypes
    "annotation",
    "character-variant",
    "historical-forms",
    "ornaments",
    "styleset",
    "stylistic",
    "swash",
    // rest
    "counter-style",
    "custom-media",
    "custom-selector",
    "font-face",
    "font-feature-values",
    "font-palette-values",
    "function",
    "import",
    "keyframes",
    "namespace",
    "page",
    "position-try",
    "property",
    "scroll-timeline",
    "view-transition",
];

pub const CAMEL_CASE_FUNCTIONS: &[&str] = &[
    "translateX",
    "translateY",
    "translateZ",
    "scaleX",
    "scaleY",
    "scaleZ",
    "rotateX",
    "rotateY",
    "rotateZ",
    "skewX",
    "skewY",
];

pub const HUE_AS_FIRST_COMPONENT_COLOR_FUNCTIONS: &[&str] = &["hsl", "hsla", "hwb"];
pub const HUE_AS_THIRD_COMPONENT_COLOR_FUNCTIONS: &[&str] = &["lch", "oklch"];
pub const HUE_COLOR_FUNCTIONS: &[&str] = &["hsl", "hsla", "hwb", "lch", "oklch"];

pub const LIGHTNESS_ZERO_TO_ONE_COLOR_FUNCTIONS: &[&str] = &["oklab", "oklch"];
pub const LIGHTNESS_ZERO_TO_HUNDRED_COLOR_FUNCTIONS: &[&str] = &["lab", "lch"];
pub const LIGHTNESS_COLOR_FUNCTIONS: &[&str] = &["oklab", "oklch", "lab", "lch"];

pub const WITH_ALPHA_ALIAS_COLOR_FUNCTIONS: &[&str] = &["hsla", "rgba"];
pub const WITHOUT_ALPHA_ALIAS_COLOR_FUNCTIONS: &[&str] = &["hsl", "rgb"];
pub const LEGACY_NOTATION_COLOR_FUNCTIONS: &[&str] = &["hsla", "rgba", "hsl", "rgb"];

pub const COLOR_FUNCTIONS: &[&str] = &[
    "color",
    "color-contrast",
    "color-mix",
    "contrast-color",
    "device-cmyk",
    "hsl",
    "hsla",
    "hwb",
    "lab",
    "lch",
    "light-dark",
    "oklab",
    "oklch",
    "rgb",
    "rgba",
];

pub const SINGLE_ARGUMENT_MATH_FUNCTIONS: &[&str] = &[
    "abs", "acos", "asin", "atan", "calc", "cos", "exp", "sign", "sin", "sqrt", "tan",
];

pub const MATH_FUNCTIONS: &[&str] = &[
    "abs",
    "acos",
    "asin",
    "atan",
    "calc",
    "cos",
    "exp",
    "sign",
    "sin",
    "sqrt",
    "tan",
    "atan2",
    "calc-size",
    "clamp",
    "hypot",
    "log",
    "max",
    "min",
    "mod",
    "pow",
    "random",
    "rem",
    "round",
];

pub fn is_math_function(name: &str) -> bool {
    in_set_ignore_case(MATH_FUNCTIONS, name)
}

pub const DEPRECATED_MEDIA_TYPES_NAMES: &[&str] = &[
    "aural",
    "braille",
    "embossed",
    "handheld",
    "projection",
    "speech",
    "tty",
    "tv",
];

pub const PREFIXES: &[&str] = &[
    "-webkit-", "-moz-", "-ms-", "-o-", "-xv-", "-apple-", "-wap-", "-khtml-",
];

pub const DEPRECATED_MEDIA_FEATURE_NAMES: &[&str] =
    &["device-aspect-ratio", "device-height", "device-width"];

pub const RANGE_TYPE_MEDIA_FEATURE_NAMES: &[&str] = &[
    "device-aspect-ratio",
    "device-height",
    "device-width",
    "aspect-ratio",
    "color",
    "color-index",
    "height",
    "horizontal-viewport-segments",
    "monochrome",
    "resolution",
    "vertical-viewport-segments",
    "width",
];

pub const RANGE_TYPE_MEDIA_FEATURE_NAMES_WITH_MIN_MAX_PREFIX: &[&str] = &[
    "min-device-aspect-ratio",
    "max-device-aspect-ratio",
    "min-device-height",
    "max-device-height",
    "min-device-width",
    "max-device-width",
    "min-aspect-ratio",
    "max-aspect-ratio",
    "min-color",
    "max-color",
    "min-color-index",
    "max-color-index",
    "min-height",
    "max-height",
    "min-horizontal-viewport-segments",
    "max-horizontal-viewport-segments",
    "min-monochrome",
    "max-monochrome",
    "min-resolution",
    "max-resolution",
    "min-vertical-viewport-segments",
    "max-vertical-viewport-segments",
    "min-width",
    "max-width",
];

pub const DISCRETE_TYPE_MEDIA_FEATURE_NAMES: &[&str] = &[
    "any-hover",
    "any-pointer",
    "color-gamut",
    "display-mode",
    "dynamic-range",
    "environment-blending",
    "forced-colors",
    "grid",
    "hover",
    "inverted-colors",
    "light-level",
    "nav-controls",
    "orientation",
    "overflow-block",
    "overflow-inline",
    "pointer",
    "prefers-color-scheme",
    "prefers-contrast",
    "prefers-reduced-data",
    "prefers-reduced-motion",
    "prefers-reduced-transparency",
    "scan",
    "scripting",
    "update",
    "video-color-gamut",
    "video-dynamic-range",
];

pub const MEDIA_FEATURE_NAMES: &[&str] = &[
    // rangeTypeMediaFeatureNames (includes deprecatedMediaFeatureNames)
    "device-aspect-ratio",
    "device-height",
    "device-width",
    "aspect-ratio",
    "color",
    "color-index",
    "height",
    "horizontal-viewport-segments",
    "monochrome",
    "resolution",
    "vertical-viewport-segments",
    "width",
    // rangeTypeMediaFeatureNamesWithMinMaxPrefix
    "min-device-aspect-ratio",
    "max-device-aspect-ratio",
    "min-device-height",
    "max-device-height",
    "min-device-width",
    "max-device-width",
    "min-aspect-ratio",
    "max-aspect-ratio",
    "min-color",
    "max-color",
    "min-color-index",
    "max-color-index",
    "min-height",
    "max-height",
    "min-horizontal-viewport-segments",
    "max-horizontal-viewport-segments",
    "min-monochrome",
    "max-monochrome",
    "min-resolution",
    "max-resolution",
    "min-vertical-viewport-segments",
    "max-vertical-viewport-segments",
    "min-width",
    "max-width",
    // discreteTypeMediaFeatureNames
    "any-hover",
    "any-pointer",
    "color-gamut",
    "display-mode",
    "dynamic-range",
    "environment-blending",
    "forced-colors",
    "grid",
    "hover",
    "inverted-colors",
    "light-level",
    "nav-controls",
    "orientation",
    "overflow-block",
    "overflow-inline",
    "pointer",
    "prefers-color-scheme",
    "prefers-contrast",
    "prefers-reduced-data",
    "prefers-reduced-motion",
    "prefers-reduced-transparency",
    "scan",
    "scripting",
    "update",
    "video-color-gamut",
    "video-dynamic-range",
];

pub fn media_feature_name_allowed_value_keywords(
    feature_name: &str,
) -> Option<&'static [&'static str]> {
    match feature_name {
        "any-hover" => Some(&["none", "hover"]),
        "any-pointer" => Some(&["none", "coarse", "fine"]),
        "color-gamut" => Some(&["srgb", "p3", "rec2020"]),
        "display-mode" => Some(&[
            "fullscreen",
            "standalone",
            "minimal-ui",
            "browser",
            "picture-in-picture",
        ]),
        "dynamic-range" => Some(&["standard", "high"]),
        "environment-blending" => Some(&["opaque", "additive", "subtractive"]),
        "forced-colors" => Some(&["none", "active"]),
        "hover" => Some(&["none", "hover"]),
        "inverted-colors" => Some(&["none", "inverted"]),
        "nav-controls" => Some(&["none", "back"]),
        "orientation" => Some(&["portrait", "landscape"]),
        "overflow-block" => Some(&["none", "scroll", "paged"]),
        "overflow-inline" => Some(&["none", "scroll"]),
        "pointer" => Some(&["none", "coarse", "fine"]),
        "prefers-color-scheme" => Some(&["light", "dark"]),
        "prefers-contrast" => Some(&["no-preference", "less", "more", "custom"]),
        "prefers-reduced-data" => Some(&["no-preference", "reduce"]),
        "prefers-reduced-motion" => Some(&["no-preference", "reduce"]),
        "prefers-reduced-transparency" => Some(&["no-preference", "reduce"]),
        "resolution" => Some(&["infinite"]),
        "scan" => Some(&["interlace", "progressive"]),
        "scripting" => Some(&["none", "initial-only", "enabled"]),
        "update" => Some(&["none", "slow", "fast"]),
        "video-color-gamut" => Some(&["srgb", "p3", "rec2020"]),
        "video-dynamic-range" => Some(&["standard", "high"]),
        _ => None,
    }
}

pub fn media_feature_name_allowed_value_types(
    feature_name: &str,
) -> Option<&'static [&'static str]> {
    match feature_name {
        "aspect-ratio" => Some(&["ratio"]),
        "color" => Some(&["integer"]),
        "color-index" => Some(&["integer"]),
        "device-aspect-ratio" => Some(&["ratio"]),
        "device-height" => Some(&["length"]),
        "device-width" => Some(&["length"]),
        "grid" => Some(&["mq-boolean"]),
        "height" => Some(&["length"]),
        "horizontal-viewport-segments" => Some(&["integer"]),
        "monochrome" => Some(&["integer"]),
        "resolution" => Some(&["resolution"]),
        "vertical-viewport-segments" => Some(&["integer"]),
        "width" => Some(&["length"]),
        _ => None,
    }
}

pub const BASIC_KEYWORDS: &[&str] = &["initial", "inherit", "revert", "revert-layer", "unset"];
pub const CSS_WIDE_KEYWORDS: &[&str] = BASIC_KEYWORDS;

pub fn is_css_wide_keyword(s: &str) -> bool {
    in_set_ignore_case(BASIC_KEYWORDS, s)
}

pub const DISPLAY_OUTSIDE_KEYWORDS: &[&str] = &["block", "inline", "run-in"];
pub const DISPLAY_INSIDE_KEYWORDS: &[&str] =
    &["flow", "flow-root", "table", "flex", "grid", "ruby"];
pub const DISPLAY_LEGACY_KEYWORDS: &[&str] =
    &["inline-block", "inline-table", "inline-flex", "inline-grid"];
pub const DISPLAY_LIST_ITEM_KEYWORD: &[&str] = &["list-item"];

pub const SYSTEM_FONT_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "caption",
    "icon",
    "menu",
    "message-box",
    "small-caption",
    "status-bar",
];

pub const FONT_FAMILY_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "cursive",
    "emoji",
    "fangsong",
    "fantasy",
    "math",
    "monospace",
    "sans-serif",
    "serif",
    "system-ui",
    "ui-monospace",
    "ui-rounded",
    "ui-sans-serif",
    "ui-serif",
];

pub const APPLE_SYSTEM_FONTS: &[&str] = &[
    "-apple-system",
    "-apple-system-headline",
    "-apple-system-body",
    "-apple-system-subheadline",
    "-apple-system-footnote",
    "-apple-system-caption1",
    "-apple-system-caption2",
    "-apple-system-short-headline",
    "-apple-system-short-body",
    "-apple-system-short-subheadline",
    "-apple-system-short-footnote",
    "-apple-system-short-caption1",
    "-apple-system-tall-body",
    "-apple-system-title0",
    "-apple-system-title1",
    "-apple-system-title2",
    "-apple-system-title3",
    "-apple-system-title4",
];

pub const MOZILLA_SYSTEM_FONTS: &[&str] = &[
    "-moz-button",
    "-moz-desktop",
    "-moz-dialog",
    "-moz-document",
    "-moz-field",
    "-moz-fixed",
    "-moz-info",
    "-moz-list",
    "-moz-pull-down-menu",
    "-moz-window",
    "-moz-workspace",
];

pub const WEBKIT_SYSTEM_FONTS: &[&str] = &[
    "-webkit-body",
    "-webkit-control",
    "-webkit-mini-control",
    "-webkit-pictograph",
    "-webkit-small-control",
    "-webkit-standard",
];

pub const PREFIXED_SYSTEM_FONTS: &[&str] = &[
    // appleSystemFonts
    "-apple-system",
    "-apple-system-headline",
    "-apple-system-body",
    "-apple-system-subheadline",
    "-apple-system-footnote",
    "-apple-system-caption1",
    "-apple-system-caption2",
    "-apple-system-short-headline",
    "-apple-system-short-body",
    "-apple-system-short-subheadline",
    "-apple-system-short-footnote",
    "-apple-system-short-caption1",
    "-apple-system-tall-body",
    "-apple-system-title0",
    "-apple-system-title1",
    "-apple-system-title2",
    "-apple-system-title3",
    "-apple-system-title4",
    // mozillaSystemFonts
    "-moz-button",
    "-moz-desktop",
    "-moz-dialog",
    "-moz-document",
    "-moz-field",
    "-moz-fixed",
    "-moz-info",
    "-moz-list",
    "-moz-pull-down-menu",
    "-moz-window",
    "-moz-workspace",
    // webkitSystemFonts
    "-webkit-body",
    "-webkit-control",
    "-webkit-mini-control",
    "-webkit-pictograph",
    "-webkit-small-control",
    "-webkit-standard",
];

pub const FONT_WEIGHT_RELATIVE_KEYWORDS: &[&str] = &["bolder", "lighter"];
pub const FONT_WEIGHT_ABSOLUTE_KEYWORDS: &[&str] = &["normal", "bold"];
pub const FONT_WEIGHT_NON_NUMERIC_KEYWORDS: &[&str] = &["bolder", "lighter", "normal", "bold"];
pub const FONT_WEIGHT_NUMERIC_KEYWORDS: &[&str] = &[
    "100", "200", "300", "400", "500", "600", "700", "800", "900",
];
pub const FONT_WEIGHT_NUMERIC_KEYWORDS_WITH_NAMED_EQUIVALENT: &[&str] = &["400", "700"];

pub const FONT_WEIGHT_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "bolder",
    "lighter",
    "normal",
    "bold",
    "100",
    "200",
    "300",
    "400",
    "500",
    "600",
    "700",
    "800",
    "900",
];

pub const FONT_STYLE_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "normal",
    "italic",
    "oblique",
];

pub const FONT_VARIANT_CSS2_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "normal",
    "none",
    "small-caps",
];

pub const FONT_STRETCH_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "condensed",
    "expanded",
    "extra-condensed",
    "extra-expanded",
    "semi-condensed",
    "semi-expanded",
    "ultra-condensed",
    "ultra-expanded",
];

pub const FONT_SIZE_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "xx-small",
    "x-small",
    "small",
    "medium",
    "large",
    "x-large",
    "xx-large",
    "xxx-large",
    "larger",
    "smaller",
    "math",
    "-konq-xxx-large",
    "-webkit-xxx-large",
];

pub const LINE_HEIGHT_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "normal",
];

pub const FONT_SHORTHAND_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "normal",
    "italic",
    "oblique",
    "none",
    "small-caps",
    "bolder",
    "lighter",
    "bold",
    "100",
    "200",
    "300",
    "400",
    "500",
    "600",
    "700",
    "800",
    "900",
    "condensed",
    "expanded",
    "extra-condensed",
    "extra-expanded",
    "semi-condensed",
    "semi-expanded",
    "ultra-condensed",
    "ultra-expanded",
    "xx-small",
    "x-small",
    "small",
    "medium",
    "large",
    "x-large",
    "xx-large",
    "xxx-large",
    "larger",
    "smaller",
    "math",
    "-konq-xxx-large",
    "-webkit-xxx-large",
    "cursive",
    "emoji",
    "fangsong",
    "fantasy",
    "monospace",
    "sans-serif",
    "serif",
    "system-ui",
    "ui-monospace",
    "ui-rounded",
    "ui-sans-serif",
    "ui-serif",
];

pub const ANIMATION_NAME_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "none",
];

pub const ANIMATION_TIMING_FUNCTION_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "cubic-bezier",
    "ease",
    "ease-in",
    "ease-in-out",
    "ease-out",
    "linear",
    "step-end",
    "step-start",
    "steps",
];

pub const ANIMATION_ITERATION_COUNT_KEYWORDS: &[&str] = &["infinite"];

pub const ANIMATION_DIRECTION_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "alternate",
    "alternate-reverse",
    "normal",
    "reverse",
];

pub const ANIMATION_FILL_MODE_KEYWORDS: &[&str] = &["none", "forwards", "backwards", "both"];

pub const ANIMATION_PLAY_STATE_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "running",
    "paused",
];

pub const ANIMATION_SHORTHAND_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "none",
    "cubic-bezier",
    "ease",
    "ease-in",
    "ease-in-out",
    "ease-out",
    "linear",
    "step-end",
    "step-start",
    "steps",
    "infinite",
    "alternate",
    "alternate-reverse",
    "normal",
    "reverse",
    "forwards",
    "backwards",
    "both",
    "running",
    "paused",
];

pub const GRID_ROW_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "auto",
    "span",
];
pub const GRID_COLUMN_KEYWORDS: &[&str] = GRID_ROW_KEYWORDS;
pub const GRID_AREA_KEYWORDS: &[&str] = GRID_ROW_KEYWORDS;

pub const COUNTER_INCREMENT_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "none",
];
pub const COUNTER_RESET_KEYWORDS: &[&str] = COUNTER_INCREMENT_KEYWORDS;

pub const LIST_STYLE_TYPE_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "afar",
    "amharic",
    "amharic-abegede",
    "arabic-indic",
    "armenian",
    "asterisks",
    "bengali",
    "binary",
    "cambodian",
    "circle",
    "cjk-decimal",
    "cjk-earthly-branch",
    "cjk-heavenly-stem",
    "cjk-ideographic",
    "decimal",
    "decimal-leading-zero",
    "devanagari",
    "disc",
    "disclosure-closed",
    "disclosure-open",
    "ethiopic-abegede",
    "ethiopic-abegede-am-et",
    "ethiopic-abegede-gez",
    "ethiopic-abegede-ti-er",
    "ethiopic-abegede-ti-et",
    "ethiopic-halehame",
    "ethiopic-halehame-aa-er",
    "ethiopic-halehame-aa-et",
    "ethiopic-halehame-am",
    "ethiopic-halehame-am-et",
    "ethiopic-halehame-gez",
    "ethiopic-halehame-om-et",
    "ethiopic-halehame-sid-et",
    "ethiopic-halehame-so-et",
    "ethiopic-halehame-ti-er",
    "ethiopic-halehame-ti-et",
    "ethiopic-halehame-tig",
    "ethiopic-numeric",
    "footnotes",
    "georgian",
    "gujarati",
    "gurmukhi",
    "hangul",
    "hangul-consonant",
    "hebrew",
    "hiragana",
    "hiragana-iroha",
    "japanese-formal",
    "japanese-informal",
    "kannada",
    "katakana",
    "katakana-iroha",
    "khmer",
    "korean-hangul-formal",
    "korean-hanja-formal",
    "korean-hanja-informal",
    "lao",
    "lower-alpha",
    "lower-armenian",
    "lower-greek",
    "lower-hexadecimal",
    "lower-latin",
    "lower-norwegian",
    "lower-roman",
    "malayalam",
    "mongolian",
    "myanmar",
    "none",
    "octal",
    "oriya",
    "oromo",
    "persian",
    "sidama",
    "simp-chinese-formal",
    "simp-chinese-informal",
    "somali",
    "square",
    "tamil",
    "telugu",
    "thai",
    "tibetan",
    "tigre",
    "tigrinya-er",
    "tigrinya-er-abegede",
    "tigrinya-et",
    "tigrinya-et-abegede",
    "trad-chinese-formal",
    "trad-chinese-informal",
    "upper-alpha",
    "upper-armenian",
    "upper-greek",
    "upper-hexadecimal",
    "upper-latin",
    "upper-norwegian",
    "upper-roman",
    "urdu",
];

pub const LIST_STYLE_POSITION_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "inside",
    "outside",
];

pub const LIST_STYLE_IMAGE_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "none",
];

pub const LIST_STYLE_SHORTHAND_KEYWORDS: &[&str] = &[
    "initial",
    "inherit",
    "revert",
    "revert-layer",
    "unset",
    "inside",
    "outside",
    "afar",
    "amharic",
    "amharic-abegede",
    "arabic-indic",
    "armenian",
    "asterisks",
    "bengali",
    "binary",
    "cambodian",
    "circle",
    "cjk-decimal",
    "cjk-earthly-branch",
    "cjk-heavenly-stem",
    "cjk-ideographic",
    "decimal",
    "decimal-leading-zero",
    "devanagari",
    "disc",
    "disclosure-closed",
    "disclosure-open",
    "ethiopic-abegede",
    "ethiopic-abegede-am-et",
    "ethiopic-abegede-gez",
    "ethiopic-abegede-ti-er",
    "ethiopic-abegede-ti-et",
    "ethiopic-halehame",
    "ethiopic-halehame-aa-er",
    "ethiopic-halehame-aa-et",
    "ethiopic-halehame-am",
    "ethiopic-halehame-am-et",
    "ethiopic-halehame-gez",
    "ethiopic-halehame-om-et",
    "ethiopic-halehame-sid-et",
    "ethiopic-halehame-so-et",
    "ethiopic-halehame-ti-er",
    "ethiopic-halehame-ti-et",
    "ethiopic-halehame-tig",
    "ethiopic-numeric",
    "footnotes",
    "georgian",
    "gujarati",
    "gurmukhi",
    "hangul",
    "hangul-consonant",
    "hebrew",
    "hiragana",
    "hiragana-iroha",
    "japanese-formal",
    "japanese-informal",
    "kannada",
    "katakana",
    "katakana-iroha",
    "khmer",
    "korean-hangul-formal",
    "korean-hanja-formal",
    "korean-hanja-informal",
    "lao",
    "lower-alpha",
    "lower-armenian",
    "lower-greek",
    "lower-hexadecimal",
    "lower-latin",
    "lower-norwegian",
    "lower-roman",
    "malayalam",
    "mongolian",
    "myanmar",
    "none",
    "octal",
    "oriya",
    "oromo",
    "persian",
    "sidama",
    "simp-chinese-formal",
    "simp-chinese-informal",
    "somali",
    "square",
    "tamil",
    "telugu",
    "thai",
    "tibetan",
    "tigre",
    "tigrinya-er",
    "tigrinya-er-abegede",
    "tigrinya-et",
    "tigrinya-et-abegede",
    "trad-chinese-formal",
    "trad-chinese-informal",
    "upper-alpha",
    "upper-armenian",
    "upper-greek",
    "upper-hexadecimal",
    "upper-latin",
    "upper-norwegian",
    "upper-roman",
    "urdu",
];

pub const CAMEL_CASE_KEYWORDS: &[&str] = &[
    "optimizeSpeed",
    "optimizeQuality",
    "optimizeLegibility",
    "geometricPrecision",
    "currentColor",
    "crispEdges",
    "visiblePainted",
    "visibleFill",
    "visibleStroke",
    "sRGB",
    "linearRGB",
];

pub const KEYFRAME_SELECTOR_KEYWORDS: &[&str] = &["from", "to"];

pub const PREFIXED_SYSTEM_COLOR_KEYWORDS: &[&str] = &[
    "-moz-buttondefault",
    "-moz-buttonhoverface",
    "-moz-buttonhovertext",
    "-moz-cellhighlight",
    "-moz-cellhighlighttext",
    "-moz-combobox",
    "-moz-comboboxtext",
    "-moz-dialog",
    "-moz-dialogtext",
    "-moz-dragtargetzone",
    "-moz-eventreerow",
    "-moz-field",
    "-moz-fieldtext",
    "-moz-html-cellhighlight",
    "-moz-html-cellhighlighttext",
    "-moz-mac-accentdarkestshadow",
    "-moz-mac-accentdarkshadow",
    "-moz-mac-accentface",
    "-moz-mac-accentlightesthighlight",
    "-moz-mac-accentlightshadow",
    "-moz-mac-accentregularhighlight",
    "-moz-mac-accentregularshadow",
    "-moz-mac-chrome-active",
    "-moz-mac-chrome-inactive",
    "-moz-mac-focusring",
    "-moz-mac-menuselect",
    "-moz-mac-menushadow",
    "-moz-mac-menutextselect",
    "-moz-menubarhovertext",
    "-moz-menubartext",
    "-moz-menuhover",
    "-moz-menuhovertext",
    "-moz-nativehyperlinktext",
    "-moz-oddtreerow",
    "-moz-win-accentcolor",
    "-moz-win-accentcolortext",
    "-moz-win-communicationstext",
    "-moz-win-mediatext",
    "-ms-hotlight",
];

pub const DEPRECATED_SYSTEM_COLOR_KEYWORDS: &[&str] = &[
    "activeborder",
    "activecaption",
    "appworkspace",
    "background",
    "buttonhighlight",
    "buttonshadow",
    "captiontext",
    "inactiveborder",
    "inactivecaption",
    "inactivecaptiontext",
    "infobackground",
    "infotext",
    "menu",
    "menutext",
    "scrollbar",
    "threeddarkshadow",
    "threedface",
    "threedhighlight",
    "threedlightshadow",
    "threedshadow",
    "window",
    "windowframe",
    "windowtext",
];

pub const SYSTEM_COLORS_KEYWORDS: &[&str] = &[
    // prefixedSystemColorKeywords
    "-moz-buttondefault",
    "-moz-buttonhoverface",
    "-moz-buttonhovertext",
    "-moz-cellhighlight",
    "-moz-cellhighlighttext",
    "-moz-combobox",
    "-moz-comboboxtext",
    "-moz-dialog",
    "-moz-dialogtext",
    "-moz-dragtargetzone",
    "-moz-eventreerow",
    "-moz-field",
    "-moz-fieldtext",
    "-moz-html-cellhighlight",
    "-moz-html-cellhighlighttext",
    "-moz-mac-accentdarkestshadow",
    "-moz-mac-accentdarkshadow",
    "-moz-mac-accentface",
    "-moz-mac-accentlightesthighlight",
    "-moz-mac-accentlightshadow",
    "-moz-mac-accentregularhighlight",
    "-moz-mac-accentregularshadow",
    "-moz-mac-chrome-active",
    "-moz-mac-chrome-inactive",
    "-moz-mac-focusring",
    "-moz-mac-menuselect",
    "-moz-mac-menushadow",
    "-moz-mac-menutextselect",
    "-moz-menubarhovertext",
    "-moz-menubartext",
    "-moz-menuhover",
    "-moz-menuhovertext",
    "-moz-nativehyperlinktext",
    "-moz-oddtreerow",
    "-moz-win-accentcolor",
    "-moz-win-accentcolortext",
    "-moz-win-communicationstext",
    "-moz-win-mediatext",
    "-ms-hotlight",
    // deprecatedSystemColorKeywords
    "activeborder",
    "activecaption",
    "appworkspace",
    "background",
    "buttonhighlight",
    "buttonshadow",
    "captiontext",
    "inactiveborder",
    "inactivecaption",
    "inactivecaptiontext",
    "infobackground",
    "infotext",
    "menu",
    "menutext",
    "scrollbar",
    "threeddarkshadow",
    "threedface",
    "threedhighlight",
    "threedlightshadow",
    "threedshadow",
    "window",
    "windowframe",
    "windowtext",
    // standard system colors
    "accentcolor",
    "accentcolortext",
    "activetext",
    "buttonborder",
    "buttonface",
    "buttontext",
    "canvas",
    "canvastext",
    "field",
    "fieldtext",
    "graytext",
    "highlight",
    "highlighttext",
    "linktext",
    "mark",
    "marktext",
    "selecteditem",
    "selecteditemtext",
    "visitedtext",
];

pub const NAMED_COLORS_KEYWORDS: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];

pub fn physical_to_flow_relative_value_keyword(
    property: &str,
    keyword: &str,
) -> Option<&'static str> {
    match property {
        "clear" | "float" => match keyword {
            "left" => Some("inline-start"),
            "right" => Some("inline-end"),
            _ => None,
        },
        "offset-anchor" | "offset-position" => match keyword {
            "top" => Some("block-start"),
            "bottom" => Some("block-end"),
            "left" => Some("inline-start"),
            "right" => Some("inline-end"),
            _ => None,
        },
        "resize" => match keyword {
            "horizontal" => Some("inline"),
            "vertical" => Some("block"),
            _ => None,
        },
        "text-align" | "text-align-last" => match keyword {
            "left" => Some("start"),
            "right" => Some("end"),
            _ => None,
        },
        _ => None,
    }
}

pub fn flow_relative_value_keywords_for_property(
    property: &str,
) -> Option<&'static [&'static str]> {
    match property {
        "clear" | "float" => Some(&["inline-start", "inline-end"]),
        "offset-anchor" | "offset-position" => {
            Some(&["block-start", "block-end", "inline-start", "inline-end"])
        }
        "resize" => Some(&["inline", "block"]),
        "text-align" | "text-align-last" => Some(&["start", "end"]),
        _ => None,
    }
}

pub const ACCEPT_CUSTOM_IDENTS_PROPERTIES: &[&str] = &[
    "animation",
    "animation-name",
    "font",
    "font-family",
    "counter-increment",
    "grid-row",
    "grid-column",
    "grid-area",
    "list-style",
    "list-style-type",
];

pub const ACCEPT_PROPERTY_NAMES_PROPERTIES: &[&str] =
    &["transition", "transition-property", "will-change"];

pub fn shorthand_to_reset_to_initial_property(shorthand: &str) -> Option<&'static [&'static str]> {
    match shorthand {
        "border" => Some(&[
            "border-image",
            "border-image-outset",
            "border-image-repeat",
            "border-image-slice",
            "border-image-source",
            "border-image-width",
        ]),
        "font" => Some(&[
            "font-feature-settings",
            "font-kerning",
            "font-language-override",
            "font-optical-sizing",
            "font-size-adjust",
            "font-variant-alternates",
            "font-variant-caps",
            "font-variant-east-asian",
            "font-variant-emoji",
            "font-variant-ligatures",
            "font-variant-numeric",
            "font-variant-position",
            "font-variation-settings",
        ]),
        _ => None,
    }
}

pub fn physical_shorthand_to_flow_relative_properties(
    shorthand: &str,
) -> Option<&'static [&'static str]> {
    match shorthand {
        "border" => Some(&[
            "border-block",
            "border-block-start",
            "border-block-end",
            "border-inline",
            "border-inline-start",
            "border-inline-end",
            "border-block-width",
            "border-block-start-width",
            "border-block-end-width",
            "border-inline-width",
            "border-inline-start-width",
            "border-inline-end-width",
            "border-block-style",
            "border-block-start-style",
            "border-block-end-style",
            "border-inline-style",
            "border-inline-start-style",
            "border-inline-end-style",
            "border-block-color",
            "border-block-start-color",
            "border-block-end-color",
            "border-inline-color",
            "border-inline-start-color",
            "border-inline-end-color",
        ]),
        "border-color" => Some(&[
            "border-block-color",
            "border-block-start-color",
            "border-block-end-color",
            "border-inline-color",
            "border-inline-start-color",
            "border-inline-end-color",
        ]),
        "border-radius" => Some(&[
            "border-start-start-radius",
            "border-start-end-radius",
            "border-end-start-radius",
            "border-end-end-radius",
        ]),
        "border-style" => Some(&[
            "border-block-style",
            "border-block-start-style",
            "border-block-end-style",
            "border-inline-style",
            "border-inline-start-style",
            "border-inline-end-style",
        ]),
        "border-width" => Some(&[
            "border-block-width",
            "border-block-start-width",
            "border-block-end-width",
            "border-inline-width",
            "border-inline-start-width",
            "border-inline-end-width",
        ]),
        "contain-intrinsic-size" => Some(&[
            "contain-intrinsic-block-size",
            "contain-intrinsic-inline-size",
        ]),
        "inset" => Some(&[
            "inset-block",
            "inset-block-start",
            "inset-block-end",
            "inset-inline",
            "inset-inline-start",
            "inset-inline-end",
        ]),
        "margin" => Some(&[
            "margin-block",
            "margin-block-start",
            "margin-block-end",
            "margin-inline",
            "margin-inline-start",
            "margin-inline-end",
        ]),
        "overflow" => Some(&["overflow-block", "overflow-inline"]),
        "overscroll-behavior" => Some(&["overscroll-behavior-block", "overscroll-behavior-inline"]),
        "padding" => Some(&[
            "padding-block",
            "padding-block-start",
            "padding-block-end",
            "padding-inline",
            "padding-inline-start",
            "padding-inline-end",
        ]),
        "scroll-margin" => Some(&[
            "scroll-margin-block",
            "scroll-margin-block-start",
            "scroll-margin-block-end",
            "scroll-margin-inline",
            "scroll-margin-inline-start",
            "scroll-margin-inline-end",
        ]),
        "scroll-padding" => Some(&[
            "scroll-padding-block",
            "scroll-padding-block-start",
            "scroll-padding-block-end",
            "scroll-padding-inline",
            "scroll-padding-inline-start",
            "scroll-padding-inline-end",
        ]),
        _ => None,
    }
}

pub const SHORTHAND_PROPERTIES: &[&str] = &[
    "animation",
    "background",
    "border",
    "border-block",
    "border-block-end",
    "border-block-start",
    "border-bottom",
    "border-color",
    "border-image",
    "border-inline",
    "border-inline-end",
    "border-inline-start",
    "border-left",
    "border-radius",
    "border-right",
    "border-style",
    "border-top",
    "border-width",
    "column-rule",
    "columns",
    "contain-intrinsic-size",
    "flex",
    "flex-flow",
    "font",
    "font-synthesis",
    "font-variant",
    "gap",
    "grid",
    "grid-area",
    "grid-column",
    "grid-gap",
    "grid-row",
    "grid-template",
    "inset",
    "inset-block",
    "inset-inline",
    "list-style",
    "margin",
    "margin-block",
    "margin-inline",
    "mask",
    "outline",
    "overflow",
    "overscroll-behavior",
    "padding",
    "padding-block",
    "padding-inline",
    "place-content",
    "place-items",
    "place-self",
    "scroll-margin",
    "scroll-margin-block",
    "scroll-margin-inline",
    "scroll-padding",
    "scroll-padding-block",
    "scroll-padding-inline",
    "text-decoration",
    "text-emphasis",
    "transition",
];

pub fn longhand_sub_properties_of_shorthand(shorthand: &str) -> Option<&'static [&'static str]> {
    match shorthand {
        "animation" => Some(&[
            "animation-name",
            "animation-duration",
            "animation-timing-function",
            "animation-delay",
            "animation-iteration-count",
            "animation-direction",
            "animation-fill-mode",
            "animation-play-state",
        ]),
        "background" => Some(&[
            "background-image",
            "background-size",
            "background-position",
            "background-repeat",
            "background-origin",
            "background-clip",
            "background-attachment",
            "background-color",
        ]),
        "border" => Some(&[
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
            "border-width",
            "border-style",
            "border-color",
        ]),
        "border-block" => Some(&[
            "border-block-width",
            "border-block-style",
            "border-block-color",
        ]),
        "border-block-end" => Some(&[
            "border-block-end-width",
            "border-block-end-style",
            "border-block-end-color",
        ]),
        "border-block-start" => Some(&[
            "border-block-start-width",
            "border-block-start-style",
            "border-block-start-color",
        ]),
        "border-bottom" => Some(&[
            "border-bottom-width",
            "border-bottom-style",
            "border-bottom-color",
        ]),
        "border-color" => Some(&[
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
        ]),
        "border-image" => Some(&[
            "border-image-source",
            "border-image-slice",
            "border-image-width",
            "border-image-outset",
            "border-image-repeat",
        ]),
        "border-inline" => Some(&[
            "border-inline-width",
            "border-inline-style",
            "border-inline-color",
        ]),
        "border-inline-end" => Some(&[
            "border-inline-end-width",
            "border-inline-end-style",
            "border-inline-end-color",
        ]),
        "border-inline-start" => Some(&[
            "border-inline-start-width",
            "border-inline-start-style",
            "border-inline-start-color",
        ]),
        "border-left" => Some(&[
            "border-left-width",
            "border-left-style",
            "border-left-color",
        ]),
        "border-radius" => Some(&[
            "border-top-left-radius",
            "border-top-right-radius",
            "border-bottom-right-radius",
            "border-bottom-left-radius",
        ]),
        "border-right" => Some(&[
            "border-right-width",
            "border-right-style",
            "border-right-color",
        ]),
        "border-style" => Some(&[
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
        ]),
        "border-top" => Some(&["border-top-width", "border-top-style", "border-top-color"]),
        "border-width" => Some(&[
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
        ]),
        "column-rule" => Some(&[
            "column-rule-width",
            "column-rule-style",
            "column-rule-color",
        ]),
        "columns" => Some(&["column-width", "column-count"]),
        "contain-intrinsic-size" => Some(&["contain-intrinsic-width", "contain-intrinsic-height"]),
        "flex" => Some(&["flex-grow", "flex-shrink", "flex-basis"]),
        "flex-flow" => Some(&["flex-direction", "flex-wrap"]),
        "font" => Some(&[
            "font-style",
            "font-variant",
            "font-weight",
            "font-stretch",
            "font-size",
            "line-height",
            "font-family",
        ]),
        "font-synthesis" => Some(&[
            "font-synthesis-weight",
            "font-synthesis-style",
            "font-synthesis-small-caps",
        ]),
        "font-variant" => Some(&[
            "font-variant-ligatures",
            "font-variant-position",
            "font-variant-caps",
            "font-variant-numeric",
            "font-variant-alternates",
            "font-variant-east-asian",
            "font-variant-emoji",
        ]),
        "gap" => Some(&["row-gap", "column-gap"]),
        "grid" => Some(&[
            "grid-template-rows",
            "grid-template-columns",
            "grid-template-areas",
            "grid-auto-rows",
            "grid-auto-columns",
            "grid-auto-flow",
            "grid-column-gap",
            "grid-row-gap",
        ]),
        "grid-area" => Some(&[
            "grid-row-start",
            "grid-column-start",
            "grid-row-end",
            "grid-column-end",
        ]),
        "grid-column" => Some(&["grid-column-start", "grid-column-end"]),
        "grid-gap" => Some(&["grid-row-gap", "grid-column-gap"]),
        "grid-row" => Some(&["grid-row-start", "grid-row-end"]),
        "grid-template" => Some(&[
            "grid-template-columns",
            "grid-template-rows",
            "grid-template-areas",
        ]),
        "inset" => Some(&["top", "right", "bottom", "left"]),
        "inset-block" => Some(&["inset-block-start", "inset-block-end"]),
        "inset-inline" => Some(&["inset-inline-start", "inset-inline-end"]),
        "list-style" => Some(&["list-style-type", "list-style-position", "list-style-image"]),
        "margin" => Some(&["margin-top", "margin-right", "margin-bottom", "margin-left"]),
        "margin-block" => Some(&["margin-block-start", "margin-block-end"]),
        "margin-inline" => Some(&["margin-inline-start", "margin-inline-end"]),
        "mask" => Some(&[
            "mask-image",
            "mask-mode",
            "mask-position",
            "mask-size",
            "mask-repeat",
            "mask-origin",
            "mask-clip",
            "mask-composite",
        ]),
        "outline" => Some(&["outline-color", "outline-style", "outline-width"]),
        "overflow" => Some(&["overflow-x", "overflow-y"]),
        "overscroll-behavior" => Some(&["overscroll-behavior-x", "overscroll-behavior-y"]),
        "padding" => Some(&[
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
        ]),
        "padding-block" => Some(&["padding-block-start", "padding-block-end"]),
        "padding-inline" => Some(&["padding-inline-start", "padding-inline-end"]),
        "place-content" => Some(&["align-content", "justify-content"]),
        "place-items" => Some(&["align-items", "justify-items"]),
        "place-self" => Some(&["align-self", "justify-self"]),
        "scroll-margin" => Some(&[
            "scroll-margin-top",
            "scroll-margin-right",
            "scroll-margin-bottom",
            "scroll-margin-left",
        ]),
        "scroll-margin-block" => Some(&["scroll-margin-block-start", "scroll-margin-block-end"]),
        "scroll-margin-inline" => Some(&["scroll-margin-inline-start", "scroll-margin-inline-end"]),
        "scroll-padding" => Some(&[
            "scroll-padding-top",
            "scroll-padding-right",
            "scroll-padding-bottom",
            "scroll-padding-left",
        ]),
        "scroll-padding-block" => Some(&["scroll-padding-block-start", "scroll-padding-block-end"]),
        "scroll-padding-inline" => {
            Some(&["scroll-padding-inline-start", "scroll-padding-inline-end"])
        }
        "text-decoration" => Some(&[
            "text-decoration-line",
            "text-decoration-style",
            "text-decoration-color",
            "text-decoration-thickness",
        ]),
        "text-emphasis" => Some(&["text-emphasis-style", "text-emphasis-color"]),
        "transition" => Some(&[
            "transition-property",
            "transition-duration",
            "transition-timing-function",
            "transition-delay",
        ]),
        _ => None,
    }
}

pub const SINGLE_VALUE_COLOR_PROPERTIES: &[&str] = &[
    "accent-color",
    "background-color",
    "border-block-color",
    "border-block-end-color",
    "border-block-start-color",
    "border-bottom-color",
    "border-inline-color",
    "border-inline-end-color",
    "border-inline-start-color",
    "border-left-color",
    "border-right-color",
    "border-top-color",
    "caret-color",
    "color",
    "column-rule-color",
    "outline-color",
    "text-decoration-color",
    "text-emphasis-color",
    // SVG
    "flood-color",
    "lighting-color",
    "stop-color",
];

pub const MULTI_VALUE_COLOR_PROPERTIES: &[&str] = &["border-color", "scrollbar-color"];

pub const COLOR_PROPERTIES: &[&str] = &[
    "accent-color",
    "background-color",
    "border-block-color",
    "border-block-end-color",
    "border-block-start-color",
    "border-bottom-color",
    "border-inline-color",
    "border-inline-end-color",
    "border-inline-start-color",
    "border-left-color",
    "border-right-color",
    "border-top-color",
    "caret-color",
    "color",
    "column-rule-color",
    "outline-color",
    "text-decoration-color",
    "text-emphasis-color",
    "flood-color",
    "lighting-color",
    "stop-color",
    "border-color",
    "scrollbar-color",
];

pub const LONGHAND_TIME_PROPERTIES: &[&str] = &[
    "transition-duration",
    "transition-delay",
    "animation-duration",
    "animation-delay",
];

pub const SHORTHAND_TIME_PROPERTIES: &[&str] = &["transition", "animation"];

pub const PAGE_CONTEXT_PROPERTIES: &[&str] = &[
    "direction",
    "background-color",
    "background-image",
    "background-repeat",
    "background-attachment",
    "background-position",
    "background",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "border-width",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "border-color",
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
    "border-style",
    "border-top",
    "border-right",
    "border-bottom",
    "border-left",
    "border",
    "counter-reset",
    "counter-increment",
    "color",
    "font-family",
    "font-size",
    "font-style",
    "font-variant",
    "font-weight",
    "font",
    "height",
    "min-height",
    "max-height",
    "line-height",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "margin",
    "outline-width",
    "outline-style",
    "outline-color",
    "outline",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "padding",
    "quotes",
    "letter-spacing",
    "text-align",
    "text-decoration",
    "text-indent",
    "text-transform",
    "white-space",
    "word-spacing",
    "visibility",
    "width",
    "min-width",
    "max-width",
];

pub const MARGIN_CONTEXT_PROPERTIES: &[&str] = &[
    "direction",
    "unicode-bidi",
    "background-color",
    "background-image",
    "background-repeat",
    "background-attachment",
    "background-position",
    "background",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "border-width",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "border-color",
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
    "border-style",
    "border-top",
    "border-right",
    "border-bottom",
    "border-left",
    "border",
    "counter-reset",
    "counter-increment",
    "content",
    "color",
    "font-family",
    "font-size",
    "font-style",
    "font-variant",
    "font-weight",
    "font",
    "height",
    "min-height",
    "max-height",
    "line-height",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "margin",
    "outline-width",
    "outline-style",
    "outline-color",
    "outline",
    "overflow",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "padding",
    "quotes",
    "letter-spacing",
    "text-align",
    "text-decoration",
    "text-indent",
    "text-transform",
    "white-space",
    "word-spacing",
    "vertical-align",
    "visibility",
    "width",
    "min-width",
    "max-width",
    "z-index",
];

pub const PREVIOUSLY_KNOWN_PROPERTIES: &[&str] = &[
    "accelerator",
    "alt",
    "animation-trigger-behavior",
    "animation-trigger-exit-range",
    "animation-trigger-exit-range-end",
    "animation-trigger-exit-range-start",
    "animation-trigger-range",
    "animation-trigger-range-end",
    "animation-trigger-range-start",
    "animation-trigger-timeline",
    "app-region",
    "audio-level",
    "buffered-rendering",
    "chains",
    "color-profile",
    "color-rendering",
    "column-progression",
    "column-rule-outset",
    "display-align",
    "elevation",
    "enable-background",
    "epub-caption-side",
    "epub-hyphens",
    "epub-text-combine",
    "epub-text-emphasis",
    "epub-text-emphasis-color",
    "epub-text-emphasis-style",
    "epub-text-orientation",
    "epub-text-transform",
    "epub-word-break",
    "epub-writing-mode",
    "flow",
    "input-format",
    "inset-area",
    "item-slack",
    "layout-flow",
    "layout-grid",
    "layout-grid-char",
    "layout-grid-line",
    "layout-grid-mode",
    "layout-grid-type",
    "line-increment",
    "marker-knockout-left",
    "marker-knockout-right",
    "marker-offset",
    "marker-pattern",
    "marker-segment",
    "mask-position-x",
    "mask-position-y",
    "mask-source-type",
    "motion",
    "motion-offset",
    "motion-path",
    "motion-rotation",
    "nav-down",
    "nav-index",
    "nav-left",
    "nav-right",
    "nav-up",
    "navigation",
    "offset-block-end",
    "offset-block-start",
    "offset-inline-end",
    "offset-inline-start",
    "offset-rotation",
    "pen-action",
    "perspective-origin-x",
    "perspective-origin-y",
    "pitch",
    "pitch-range",
    "play-during",
    "position-try-options",
    "property-name",
    "richness",
    "row-rule-outset",
    "rule-outset",
    "rule-paint-order",
    "running",
    "scroll-snap-margin",
    "scroll-snap-margin-bottom",
    "scroll-snap-margin-left",
    "scroll-snap-margin-right",
    "scroll-snap-margin-top",
    "scroll-start-target",
    "scrollbar-arrow-color",
    "scrollbar-base-color",
    "scrollbar-dark-shadow-color",
    "scrollbar-darkshadow-color",
    "scrollbar-face-color",
    "scrollbar-highlight-color",
    "scrollbar-shadow-color",
    "scrollbar-track-color",
    "scrollbar3d-light-color",
    "scrollbar3dlight-color",
    "snap-height",
    "solid-color",
    "solid-opacity",
    "speak-header",
    "speak-numeral",
    "speak-punctuation",
    "speech-rate",
    "stress",
    "supported-color-schemes",
    "text-decoration-blink",
    "text-decoration-line-through",
    "text-decoration-none",
    "text-decoration-overline",
    "text-decoration-skip-inset",
    "text-decoration-trim",
    "text-decoration-underline",
    "text-justify-trim",
    "text-kashida",
    "text-kashida-space",
    "text-line-through",
    "text-line-through-color",
    "text-line-through-mode",
    "text-line-through-style",
    "text-line-through-width",
    "text-overline",
    "text-overline-color",
    "text-overline-mode",
    "text-overline-style",
    "text-overline-width",
    "text-underline",
    "text-underline-color",
    "text-underline-mode",
    "text-underline-style",
    "text-underline-width",
    "touch-action-delay",
    "transform-origin-x",
    "transform-origin-y",
    "transform-origin-z",
    "uc-alt-skin",
    "uc-skin",
    "viewport-fill",
    "viewport-fill-opacity",
    "volume",
];

pub const PREVIOUSLY_KNOWN_PREFIXED_PROPERTIES: &[&str] = &[
    "-epub-text-combine",
    "-internal-text-autosizing-status",
    "-wap-accesskey",
    "-webkit-alt",
    "-webkit-app-region",
    "-webkit-background-composite",
    "-moz-background-inline-policy",
    "-khtml-binding",
    "-webkit-border-after-color",
    "-webkit-border-after-style",
    "-webkit-border-after",
    "-webkit-border-after-width",
    "-moz-border-end-color",
    "-webkit-border-end-color",
    "-moz-border-end",
    "-moz-border-end-style",
    "-webkit-border-end-style",
    "-webkit-border-end",
    "-moz-border-end-width",
    "-webkit-border-end-width",
    "-webkit-border-fit",
    "-khtml-border-horizontal-spacing",
    "-webkit-border-horizontal-spacing",
    "-moz-border-start-color",
    "-webkit-border-start-color",
    "-moz-border-start",
    "-moz-border-start-style",
    "-webkit-border-start-style",
    "-webkit-border-start",
    "-moz-border-start-width",
    "-webkit-border-start-width",
    "-khtml-border-vertical-spacing",
    "-webkit-border-vertical-spacing",
    "-khtml-box-flex-group-transition",
    "-webkit-color-correction",
    "-apple-color-filter",
    "-webkit-column-axis",
    "-webkit-column-progression",
    "-webkit-composition-fill-color",
    "-webkit-composition-frame-color",
    "-webkit-cursor-visibility",
    "-apple-dashboard-region",
    "-khtml-dashboard-region",
    "-webkit-dashboard-region",
    "-webkit-flex-align",
    "-webkit-flex-item-align",
    "-webkit-flex-line-pack",
    "-webkit-flex-order",
    "-webkit-flex-pack",
    "-khtml-flow-mode",
    "-konq-flow-mode",
    "-khtml-font-size-delta",
    "-webkit-font-size-delta",
    "-webkit-grid-after",
    "-webkit-grid-before",
    "-ms-grid-column-span",
    "-webkit-grid-columns",
    "-webkit-grid-end",
    "-ms-grid-row-span",
    "-webkit-grid-rows",
    "-webkit-grid-start",
    "-webkit-highlight",
    "-khtml-horizontal-border-spacing",
    "-webkit-hyphenate-limit-after",
    "-webkit-hyphenate-limit-before",
    "-wap-input-format",
    "-wap-input-required",
    "-konq-js-clip",
    "-webkit-line-align",
    "-webkit-line-box-contain",
    "-webkit-line-grid-snap",
    "-o-link",
    "-o-link-source",
    "-webkit-locale",
    "-webkit-logical-height",
    "-webkit-logical-width",
    "-webkit-margin-after-collapse",
    "-webkit-margin-after",
    "-webkit-margin-before-collapse",
    "-webkit-margin-before",
    "-khtml-margin-bottom-collapse",
    "-webkit-margin-bottom-collapse",
    "-khtml-margin-collapse",
    "-webkit-margin-collapse",
    "-moz-margin-end",
    "-webkit-margin-end",
    "-khtml-margin-start",
    "-moz-margin-start",
    "-webkit-margin-start",
    "-khtml-margin-top-collapse",
    "-webkit-margin-top-collapse",
    "-wap-marquee-dir",
    "-khtml-marquee-direction",
    "-webkit-marquee-direction",
    "-khtml-marquee-increment",
    "-webkit-marquee-increment",
    "-khtml-marquee",
    "-wap-marquee-loop",
    "-khtml-marquee-repetition",
    "-webkit-marquee-repetition",
    "-khtml-marquee-speed",
    "-wap-marquee-speed",
    "-webkit-marquee-speed",
    "-khtml-marquee-style",
    "-wap-marquee-style",
    "-webkit-marquee-style",
    "-webkit-marquee",
    "-webkit-mask-box-image-outset",
    "-webkit-mask-box-image-repeat",
    "-webkit-mask-box-image-slice",
    "-webkit-mask-box-image-source",
    "-webkit-mask-box-image-width",
    "-webkit-mask-source-type",
    "-khtml-match-nearest-mail-blockquote-color",
    "-webkit-match-nearest-mail-blockquote-color",
    "-webkit-max-logical-height",
    "-webkit-max-logical-width",
    "-webkit-min-logical-height",
    "-webkit-min-logical-width",
    "-khtml-nbsp-mode",
    "-webkit-nbsp-mode",
    "-webkit-padding-after",
    "-webkit-padding-before",
    "-moz-padding-end",
    "-webkit-padding-end",
    "-khtml-padding-start",
    "-moz-padding-start",
    "-webkit-padding-start",
    "-apple-pay-button-style",
    "-apple-pay-button-type",
    "-webkit-perspective-origin-x",
    "-webkit-perspective-origin-y",
    "-webkit-region-break-after",
    "-webkit-region-break-before",
    "-webkit-region-break-inside",
    "-webkit-region-overflow",
    "-khtml-rtl-ordering",
    "-webkit-rtl-ordering",
    "-webkit-svg-shadow",
    "-o-table-baseline",
    "-ms-text-combine-horizontal",
    "-webkit-text-combine",
    "-khtml-text-decorations-in-effect",
    "-webkit-text-decorations-in-effect",
    "-webkit-text-zoom",
    "-apple-trailing-word",
    "-webkit-transform-origin-x",
    "-webkit-transform-origin-y",
    "-webkit-transform-origin-z",
    "-khtml-user-drag",
    "-khtml-user-modify",
    "-khtml-vertical-border-spacing",
    "-webkit-widget-region",
    "-webkit-wrap-margin",
    "-webkit-wrap-padding",
    "-webkit-wrap-shape-inside",
    "-webkit-wrap-shape-outside",
    "-webkit-wrap",
];

pub fn physical_to_flow_relative_property(prop: &str) -> Option<&'static str> {
    match prop {
        "margin-left" => Some("margin-inline-start"),
        "margin-right" => Some("margin-inline-end"),
        "margin-top" => Some("margin-block-start"),
        "margin-bottom" => Some("margin-block-end"),
        "padding-left" => Some("padding-inline-start"),
        "padding-right" => Some("padding-inline-end"),
        "padding-top" => Some("padding-block-start"),
        "padding-bottom" => Some("padding-block-end"),
        "border-left" => Some("border-inline-start"),
        "border-right" => Some("border-inline-end"),
        "border-top" => Some("border-block-start"),
        "border-bottom" => Some("border-block-end"),
        "border-left-width" => Some("border-inline-start-width"),
        "border-right-width" => Some("border-inline-end-width"),
        "border-top-width" => Some("border-block-start-width"),
        "border-bottom-width" => Some("border-block-end-width"),
        "border-left-style" => Some("border-inline-start-style"),
        "border-right-style" => Some("border-inline-end-style"),
        "border-top-style" => Some("border-block-start-style"),
        "border-bottom-style" => Some("border-block-end-style"),
        "border-left-color" => Some("border-inline-start-color"),
        "border-right-color" => Some("border-inline-end-color"),
        "border-top-color" => Some("border-block-start-color"),
        "border-bottom-color" => Some("border-block-end-color"),
        "left" => Some("inset-inline-start"),
        "right" => Some("inset-inline-end"),
        "top" => Some("inset-block-start"),
        "bottom" => Some("inset-block-end"),
        "width" => Some("inline-size"),
        "height" => Some("block-size"),
        "min-width" => Some("min-inline-size"),
        "min-height" => Some("min-block-size"),
        "max-width" => Some("max-inline-size"),
        "max-height" => Some("max-block-size"),
        "scroll-margin-left" => Some("scroll-margin-inline-start"),
        "scroll-margin-right" => Some("scroll-margin-inline-end"),
        "scroll-margin-top" => Some("scroll-margin-block-start"),
        "scroll-margin-bottom" => Some("scroll-margin-block-end"),
        "scroll-padding-left" => Some("scroll-padding-inline-start"),
        "scroll-padding-right" => Some("scroll-padding-inline-end"),
        "scroll-padding-top" => Some("scroll-padding-block-start"),
        "scroll-padding-bottom" => Some("scroll-padding-block-end"),
        "border-top-left-radius" => Some("border-start-start-radius"),
        "border-top-right-radius" => Some("border-start-end-radius"),
        "border-bottom-left-radius" => Some("border-end-start-radius"),
        "border-bottom-right-radius" => Some("border-end-end-radius"),
        "overflow-x" => Some("overflow-inline"),
        "overflow-y" => Some("overflow-block"),
        "overscroll-behavior-x" => Some("overscroll-behavior-inline"),
        "overscroll-behavior-y" => Some("overscroll-behavior-block"),
        "contain-intrinsic-width" => Some("contain-intrinsic-inline-size"),
        "contain-intrinsic-height" => Some("contain-intrinsic-block-size"),
        _ => None,
    }
}

/// Returns `(block_prop, inline_prop, is_axes)` for a physical shorthand.
pub fn physical_shorthand_to_flow_relative_by_axis(
    prop: &str,
) -> Option<(&'static str, &'static str, bool)> {
    match prop {
        "border-color" => Some(("border-block-color", "border-inline-color", false)),
        "border-style" => Some(("border-block-style", "border-inline-style", false)),
        "border-width" => Some(("border-block-width", "border-inline-width", false)),
        "inset" => Some(("inset-block", "inset-inline", false)),
        "margin" => Some(("margin-block", "margin-inline", false)),
        "overflow" => Some(("overflow-block", "overflow-inline", true)),
        "overscroll-behavior" => Some((
            "overscroll-behavior-block",
            "overscroll-behavior-inline",
            true,
        )),
        "padding" => Some(("padding-block", "padding-inline", false)),
        "scroll-margin" => Some(("scroll-margin-block", "scroll-margin-inline", false)),
        "scroll-padding" => Some(("scroll-padding-block", "scroll-padding-inline", false)),
        _ => None,
    }
}

pub const FLOW_RELATIVE_PROPERTIES: &[&str] = &[
    "margin-inline-start",
    "margin-inline-end",
    "margin-block-start",
    "margin-block-end",
    "padding-inline-start",
    "padding-inline-end",
    "padding-block-start",
    "padding-block-end",
    "border-inline-start",
    "border-inline-end",
    "border-block-start",
    "border-block-end",
    "border-inline-start-width",
    "border-inline-end-width",
    "border-block-start-width",
    "border-block-end-width",
    "border-inline-start-style",
    "border-inline-end-style",
    "border-block-start-style",
    "border-block-end-style",
    "border-inline-start-color",
    "border-inline-end-color",
    "border-block-start-color",
    "border-block-end-color",
    "inset-inline-start",
    "inset-inline-end",
    "inset-block-start",
    "inset-block-end",
    "inline-size",
    "block-size",
    "min-inline-size",
    "min-block-size",
    "max-inline-size",
    "max-block-size",
    "scroll-margin-inline-start",
    "scroll-margin-inline-end",
    "scroll-margin-block-start",
    "scroll-margin-block-end",
    "scroll-padding-inline-start",
    "scroll-padding-inline-end",
    "scroll-padding-block-start",
    "scroll-padding-block-end",
    "border-start-start-radius",
    "border-start-end-radius",
    "border-end-start-radius",
    "border-end-end-radius",
    "overflow-inline",
    "overflow-block",
    "overscroll-behavior-inline",
    "overscroll-behavior-block",
    "contain-intrinsic-inline-size",
    "contain-intrinsic-block-size",
    "margin-inline",
    "margin-block",
    "padding-inline",
    "padding-block",
    "border-inline",
    "border-block",
    "border-inline-width",
    "border-block-width",
    "border-inline-style",
    "border-block-style",
    "border-inline-color",
    "border-block-color",
    "inset",
    "inset-inline",
    "inset-block",
    "scroll-margin-inline",
    "scroll-margin-block",
    "scroll-padding-inline",
    "scroll-padding-block",
];

pub const DEPRECATED_HTML_TYPE_SELECTORS: &[&str] = &[
    "acronym",
    "applet",
    "basefont",
    "big",
    "bgsound",
    "blink",
    "center",
    "content",
    "dir",
    "font",
    "frame",
    "frameset",
    "isindex",
    "keygen",
    "listing",
    "marquee",
    "menuitem",
    "multicol",
    "nextid",
    "nobr",
    "noembed",
    "noframes",
    "plaintext",
    "param",
    "popup",
    "rb",
    "rtc",
    "selectmenu",
    "shadow",
    "spacer",
    "strike",
    "tt",
    "xmp",
];

pub const STANDARD_HTML_TYPE_SELECTORS: &[&str] = &[
    "a",
    "abbr",
    "address",
    "area",
    "article",
    "aside",
    "audio",
    "b",
    "base",
    "bdi",
    "bdo",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "data",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "map",
    "mark",
    "math",
    "menu",
    "meta",
    "meter",
    "nav",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "script",
    "search",
    "section",
    "select",
    "selectedcontent",
    "slot",
    "small",
    "source",
    "span",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "svg",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "track",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

pub const EXPERIMENTAL_HTML_TYPE_SELECTORS: &[&str] = &[
    "fencedframe",
    "listbox",
    "model",
    "portal",
    "selectlist",
    "geolocation",
    "usermedia",
    "install",
];

pub const HTML_TYPE_SELECTORS: &[&str] = &[
    // deprecatedHtmlTypeSelectors
    "acronym",
    "applet",
    "basefont",
    "big",
    "bgsound",
    "blink",
    "center",
    "content",
    "dir",
    "font",
    "frame",
    "frameset",
    "isindex",
    "keygen",
    "listing",
    "marquee",
    "menuitem",
    "multicol",
    "nextid",
    "nobr",
    "noembed",
    "noframes",
    "plaintext",
    "param",
    "popup",
    "rb",
    "rtc",
    "selectmenu",
    "shadow",
    "spacer",
    "strike",
    "tt",
    "xmp",
    // standardHtmlTypeSelectors
    "a",
    "abbr",
    "address",
    "area",
    "article",
    "aside",
    "audio",
    "b",
    "base",
    "bdi",
    "bdo",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "data",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "map",
    "mark",
    "math",
    "menu",
    "meta",
    "meter",
    "nav",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "script",
    "search",
    "section",
    "select",
    "selectedcontent",
    "slot",
    "small",
    "source",
    "span",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "svg",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "track",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
    // experimentalHtmlTypeSelectors
    "fencedframe",
    "listbox",
    "model",
    "portal",
    "selectlist",
    "geolocation",
    "usermedia",
    "install",
];

pub const DEPRECATED_SVG_TYPE_SELECTORS: &[&str] = &[
    "altGlyph",
    "altGlyphDef",
    "altGlyphItem",
    "animateColor",
    "color-profile",
    "cursor",
    "font",
    "font-face",
    "font-face-format",
    "font-face-name",
    "font-face-src",
    "font-face-uri",
    "glyph",
    "glyphRef",
    "hatchPath",
    "hkern",
    "missing-glyph",
    "tref",
    "vkern",
];

pub const STANDARD_SVG_TYPE_SELECTORS: &[&str] = &[
    "a",
    "animate",
    "animateMotion",
    "animateTransform",
    "circle",
    "clipPath",
    "defs",
    "desc",
    "ellipse",
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
    "filter",
    "foreignObject",
    "g",
    "image",
    "line",
    "linearGradient",
    "marker",
    "mask",
    "metadata",
    "mpath",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialGradient",
    "rect",
    "script",
    "set",
    "stop",
    "style",
    "svg",
    "switch",
    "symbol",
    "text",
    "textPath",
    "title",
    "tspan",
    "use",
    "view",
];

pub const EXPERIMENTAL_SVG_TYPE_SELECTORS: &[&str] = &["hatch", "hatchpath"];

pub const SVG_TYPE_SELECTORS: &[&str] = &[
    // deprecatedSvgTypeSelectors
    "altGlyph",
    "altGlyphDef",
    "altGlyphItem",
    "animateColor",
    "color-profile",
    "cursor",
    "font",
    "font-face",
    "font-face-format",
    "font-face-name",
    "font-face-src",
    "font-face-uri",
    "glyph",
    "glyphRef",
    "hatchPath",
    "hkern",
    "missing-glyph",
    "tref",
    "vkern",
    // standardSvgTypeSelectors
    "a",
    "animate",
    "animateMotion",
    "animateTransform",
    "circle",
    "clipPath",
    "defs",
    "desc",
    "ellipse",
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
    "filter",
    "foreignObject",
    "g",
    "image",
    "line",
    "linearGradient",
    "marker",
    "mask",
    "metadata",
    "mpath",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialGradient",
    "rect",
    "script",
    "set",
    "stop",
    "style",
    "svg",
    "switch",
    "symbol",
    "text",
    "textPath",
    "title",
    "tspan",
    "use",
    "view",
    // experimentalSvgTypeSelectors
    "hatch",
    "hatchpath",
];

pub const MIXED_CASE_SVG_TYPE_SELECTORS: &[&str] = &[
    "altGlyph",
    "altGlyphDef",
    "altGlyphItem",
    "animateColor",
    "glyphRef",
    "hatchPath",
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
    "linearGradient",
    "radialGradient",
    "textPath",
];

pub const MATHML_TYPE_SELECTORS: &[&str] = &[
    "abs",
    "and",
    "annotation",
    "annotation-xml",
    "apply",
    "approx",
    "arccos",
    "arccosh",
    "arccot",
    "arccoth",
    "arccsc",
    "arccsch",
    "arcsec",
    "arcsech",
    "arcsin",
    "arcsinh",
    "arctan",
    "arctanh",
    "arg",
    "bind",
    "bvar",
    "card",
    "cartesianproduct",
    "cbytes",
    "ceiling",
    "cerror",
    "ci",
    "cn",
    "codomain",
    "complexes",
    "compose",
    "condition",
    "conjugate",
    "cos",
    "cosh",
    "cot",
    "coth",
    "cs",
    "csc",
    "csch",
    "csymbol",
    "curl",
    "declare",
    "degree",
    "determinant",
    "diff",
    "divergence",
    "divide",
    "domain",
    "domainofapplication",
    "emptyset",
    "encoding",
    "eq",
    "equivalent",
    "eulergamma",
    "exists",
    "exp",
    "exponentiale",
    "factorial",
    "factorof",
    "false",
    "floor",
    "fn",
    "forall",
    "function",
    "gcd",
    "geq",
    "grad",
    "gt",
    "ident",
    "image",
    "imaginary",
    "imaginaryi",
    "implies",
    "in",
    "infinity",
    "int",
    "integers",
    "intersect",
    "interval",
    "inverse",
    "lambda",
    "laplacian",
    "lcm",
    "leq",
    "limit",
    "list",
    "ln",
    "log",
    "logbase",
    "lowlimit",
    "lt",
    "maction",
    "malign",
    "maligngroup",
    "malignmark",
    "malignscope",
    "math",
    "matrix",
    "matrixrow",
    "max",
    "mean",
    "median",
    "menclose",
    "merror",
    "mfenced",
    "mfrac",
    "mfraction",
    "mglyph",
    "mi",
    "min",
    "minus",
    "mlabeledtr",
    "mlongdiv",
    "mmultiscripts",
    "mn",
    "mo",
    "mode",
    "moment",
    "momentabout",
    "mover",
    "mpadded",
    "mphantom",
    "mprescripts",
    "mroot",
    "mrow",
    "ms",
    "mscarries",
    "mscarry",
    "msgroup",
    "msline",
    "mspace",
    "msqrt",
    "msrow",
    "mstack",
    "mstyle",
    "msub",
    "msubsup",
    "msup",
    "mtable",
    "mtd",
    "mtext",
    "mtr",
    "munder",
    "munderover",
    "naturalnumbers",
    "neq",
    "none",
    "not",
    "notanumber",
    "notin",
    "notprsubset",
    "notsubset",
    "or",
    "otherwise",
    "outerproduct",
    "partialdiff",
    "pi",
    "piece",
    "piecewise",
    "plus",
    "power",
    "primes",
    "product",
    "prsubset",
    "quotient",
    "rationals",
    "real",
    "reals",
    "reln",
    "rem",
    "root",
    "scalarproduct",
    "sdev",
    "sec",
    "sech",
    "select",
    "selector",
    "semantics",
    "sep",
    "set",
    "setdiff",
    "share",
    "sin",
    "sinh",
    "subset",
    "sum",
    "tan",
    "tanh",
    "tendsto",
    "times",
    "transpose",
    "true",
    "union",
    "uplimit",
    "var",
    "variance",
    "vector",
    "vectorproduct",
    "xor",
];

pub const HYPHEN_CONTAINING_TYPE_SELECTORS: &[&str] = &[
    "annotation-xml",
    "color-profile",
    "font-face",
    "font-face-format",
    "font-face-name",
    "font-face-src",
    "font-face-uri",
    "missing-glyph",
];

pub const LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS: &[&str] =
    &["before", "after", "first-line", "first-letter"];

pub const SHADOW_TREE_PSEUDO_ELEMENTS: &[&str] = &["part"];

pub const ELEMENT_BACKED_PSEUDO_ELEMENTS: &[&str] =
    &["part", "details-content", "file-selector-button"];

pub const SUB_PSEUDO_ELEMENTS: &[&str] = &["marker", "scroll-marker"];

pub const ELEMENT_REPRESENTING_PSEUDO_ELEMENTS: &[&str] =
    &["part", "details-content", "file-selector-button", "slotted"];

pub const WEBKIT_SCROLLBAR_PSEUDO_ELEMENTS: &[&str] = &[
    "-webkit-resizer",
    "-webkit-scrollbar",
    "-webkit-scrollbar-button",
    "-webkit-scrollbar-corner",
    "-webkit-scrollbar-thumb",
    "-webkit-scrollbar-track",
    "-webkit-scrollbar-track-piece",
];

pub const VENDOR_SPECIFIC_PSEUDO_ELEMENTS: &[&str] = &[
    "-webkit-resizer",
    "-webkit-scrollbar",
    "-webkit-scrollbar-button",
    "-webkit-scrollbar-corner",
    "-webkit-scrollbar-thumb",
    "-webkit-scrollbar-track",
    "-webkit-scrollbar-track-piece",
    "-moz-focus-inner",
    "-moz-focus-outer",
    "-moz-list-bullet",
    "-moz-meter-bar",
    "-moz-placeholder",
    "-moz-progress-bar",
    "-moz-range-progress",
    "-moz-range-thumb",
    "-moz-range-track",
    "-ms-browse",
    "-ms-check",
    "-ms-clear",
    "-ms-expand",
    "-ms-fill",
    "-ms-fill-lower",
    "-ms-fill-upper",
    "-ms-reveal",
    "-ms-thumb",
    "-ms-ticks-after",
    "-ms-ticks-before",
    "-ms-tooltip",
    "-ms-track",
    "-ms-value",
    "-webkit-calendar-picker-indicator",
    "-webkit-clear-button",
    "-webkit-color-swatch",
    "-webkit-color-swatch-wrapper",
    "-webkit-date-and-time-value",
    "-webkit-datetime-edit",
    "-webkit-datetime-edit-ampm-field",
    "-webkit-datetime-edit-day-field",
    "-webkit-datetime-edit-fields-wrapper",
    "-webkit-datetime-edit-hour-field",
    "-webkit-datetime-edit-millisecond-field",
    "-webkit-datetime-edit-minute-field",
    "-webkit-datetime-edit-month-field",
    "-webkit-datetime-edit-second-field",
    "-webkit-datetime-edit-text",
    "-webkit-datetime-edit-week-field",
    "-webkit-datetime-edit-year-field",
    "-webkit-details-marker",
    "-webkit-distributed",
    "-webkit-file-upload-button",
    "-webkit-input-placeholder",
    "-webkit-keygen-select",
    "-webkit-meter-bar",
    "-webkit-meter-even-less-good-value",
    "-webkit-meter-horizontal-bar",
    "-webkit-meter-horizontal-even-less-good-value",
    "-webkit-meter-horizontal-optimum-value",
    "-webkit-meter-horizontal-suboptimal-value",
    "-webkit-meter-inner-element",
    "-webkit-meter-optimum-value",
    "-webkit-meter-suboptimum-value",
    "-webkit-meter-vertical-bar",
    "-webkit-meter-vertical-even-less-good-value",
    "-webkit-meter-vertical-optimum-value",
    "-webkit-meter-vertical-suboptimal-value",
    "-webkit-progress-bar",
    "-webkit-progress-inner-element",
    "-webkit-progress-value",
    "-webkit-search-cancel-button",
    "-webkit-search-decoration",
    "-webkit-search-results-button",
    "-webkit-search-results-decoration",
    "-webkit-slider-runnable-track",
    "-webkit-slider-thumb",
    "-webkit-textfield-decoration-container",
    "-webkit-validation-bubble",
    "-webkit-validation-bubble-arrow",
    "-webkit-validation-bubble-arrow-clipper",
    "-webkit-validation-bubble-heading",
    "-webkit-validation-bubble-message",
    "-webkit-validation-bubble-text-block",
];

pub const DEPRECATED_PSEUDO_ELEMENTS: &[&str] = &["content", "shadow"];

pub const PSEUDO_ELEMENTS: &[&str] = &[
    // levelOneAndTwoPseudoElements
    "before",
    "after",
    "first-line",
    "first-letter",
    // vendorSpecificPseudoElements
    "-webkit-resizer",
    "-webkit-scrollbar",
    "-webkit-scrollbar-button",
    "-webkit-scrollbar-corner",
    "-webkit-scrollbar-thumb",
    "-webkit-scrollbar-track",
    "-webkit-scrollbar-track-piece",
    "-moz-focus-inner",
    "-moz-focus-outer",
    "-moz-list-bullet",
    "-moz-meter-bar",
    "-moz-placeholder",
    "-moz-progress-bar",
    "-moz-range-progress",
    "-moz-range-thumb",
    "-moz-range-track",
    "-ms-browse",
    "-ms-check",
    "-ms-clear",
    "-ms-expand",
    "-ms-fill",
    "-ms-fill-lower",
    "-ms-fill-upper",
    "-ms-reveal",
    "-ms-thumb",
    "-ms-ticks-after",
    "-ms-ticks-before",
    "-ms-tooltip",
    "-ms-track",
    "-ms-value",
    "-webkit-calendar-picker-indicator",
    "-webkit-clear-button",
    "-webkit-color-swatch",
    "-webkit-color-swatch-wrapper",
    "-webkit-date-and-time-value",
    "-webkit-datetime-edit",
    "-webkit-datetime-edit-ampm-field",
    "-webkit-datetime-edit-day-field",
    "-webkit-datetime-edit-fields-wrapper",
    "-webkit-datetime-edit-hour-field",
    "-webkit-datetime-edit-millisecond-field",
    "-webkit-datetime-edit-minute-field",
    "-webkit-datetime-edit-month-field",
    "-webkit-datetime-edit-second-field",
    "-webkit-datetime-edit-text",
    "-webkit-datetime-edit-week-field",
    "-webkit-datetime-edit-year-field",
    "-webkit-details-marker",
    "-webkit-distributed",
    "-webkit-file-upload-button",
    "-webkit-input-placeholder",
    "-webkit-keygen-select",
    "-webkit-meter-bar",
    "-webkit-meter-even-less-good-value",
    "-webkit-meter-horizontal-bar",
    "-webkit-meter-horizontal-even-less-good-value",
    "-webkit-meter-horizontal-optimum-value",
    "-webkit-meter-horizontal-suboptimal-value",
    "-webkit-meter-inner-element",
    "-webkit-meter-optimum-value",
    "-webkit-meter-suboptimum-value",
    "-webkit-meter-vertical-bar",
    "-webkit-meter-vertical-even-less-good-value",
    "-webkit-meter-vertical-optimum-value",
    "-webkit-meter-vertical-suboptimal-value",
    "-webkit-progress-bar",
    "-webkit-progress-inner-element",
    "-webkit-progress-value",
    "-webkit-search-cancel-button",
    "-webkit-search-decoration",
    "-webkit-search-results-button",
    "-webkit-search-results-decoration",
    "-webkit-slider-runnable-track",
    "-webkit-slider-thumb",
    "-webkit-textfield-decoration-container",
    "-webkit-validation-bubble",
    "-webkit-validation-bubble-arrow",
    "-webkit-validation-bubble-arrow-clipper",
    "-webkit-validation-bubble-heading",
    "-webkit-validation-bubble-message",
    "-webkit-validation-bubble-text-block",
    // elementRepresentingPseudoElements (includes elementBackedPseudoElements)
    "part",
    "details-content",
    "file-selector-button",
    "slotted",
    // subPseudoElements
    "marker",
    "scroll-marker",
    // deprecatedPseudoElements
    "content",
    "shadow",
    // standard
    "backdrop",
    "checkmark",
    "clear-icon",
    "color-swatch",
    "cue",
    "field-component",
    "field-separator",
    "field-text",
    "grammar-error",
    "highlight",
    "picker",
    "picker-icon",
    "placeholder",
    "scroll-button",
    "scroll-marker-group",
    "search-text",
    "selection",
    "slider-fill",
    "slider-thumb",
    "slider-track",
    "spelling-error",
    "step-control",
    "step-down",
    "step-up",
    "target-text",
    "view-transition",
    "view-transition-group",
    "view-transition-image-pair",
    "view-transition-new",
    "view-transition-old",
];

pub const PERCENTAGE_KEYFRAME_WITH_NAMED_EQUIVALENT: &[&str] = &["0%", "100%"];

pub const AN_PLUS_B_NOTATION_PSEUDO_CLASSES: &[&str] = &[
    "nth-column",
    "nth-last-column",
    "nth-last-of-type",
    "nth-of-type",
];

pub const AN_PLUS_B_OF_S_NOTATION_PSEUDO_CLASSES: &[&str] = &["nth-child", "nth-last-child"];

pub const TREE_STRUCTURAL_PSEUDO_CLASSES: &[&str] = &[
    "nth-child",
    "nth-last-child",
    "empty",
    "first-child",
    "first-of-type",
    "last-child",
    "last-of-type",
    "nth-last-of-type",
    "nth-of-type",
    "only-child",
    "only-of-type",
    "root",
];

pub const AT_RULE_PAGE_PSEUDO_CLASSES: &[&str] =
    &["first", "right", "left", "blank", "recto", "verso", "nth"];

pub const LINGUISTIC_PSEUDO_CLASSES: &[&str] = &["dir", "lang"];
pub const DIR_IDENTIFIERS: &[&str] = &["ltr", "rtl"];
pub const FORGIVING_PSEUDO_CLASSES: &[&str] = &["is", "where"];
pub const LOGICAL_COMBINATIONS_PSEUDO_CLASSES: &[&str] = &["has", "is", "matches", "not", "where"];

pub const DEPRECATED_PSEUDO_CLASSES: &[&str] = &[
    "-moz-any",
    "-webkit-any",
    "contains",
    "drop",
    "focus-ring",
    "fullscreen-ancestor",
    "matches",
    "popup-open",
    "top-layer",
    "user-error",
];

pub const VENDOR_SPECIFIC_PSEUDO_CLASSES: &[&str] = &[
    "-khtml-drag",
    "-moz-any",
    "-moz-any-link",
    "-moz-bound-element",
    "-moz-broken",
    "-moz-dir",
    "-moz-drag-over",
    "-moz-empty-except-children-with-localname",
    "-moz-first-node",
    "-moz-focusring",
    "-moz-full-screen",
    "-moz-full-screen-ancestor",
    "-moz-is-html",
    "-moz-last-node",
    "-moz-loading",
    "-moz-meter-optimum",
    "-moz-meter-sub-optimum",
    "-moz-meter-sub-sub-optimum",
    "-moz-only-whitespace",
    "-moz-placeholder",
    "-moz-read-only",
    "-moz-read-write",
    "-moz-submit-invalid",
    "-moz-suppressed",
    "-moz-table-border-nonzero",
    "-moz-ui-invalid",
    "-moz-ui-valid",
    "-moz-user-disabled",
    "-moz-window-inactive",
    "-ms-fullscreen",
    "-ms-input-placeholder",
    "-webkit-drag",
    "-webkit-any",
    "-webkit-any-link",
    "-webkit-autofill",
    "-webkit-full-screen",
    "-webkit-full-screen-ancestor",
];

pub const WEBKIT_SCROLLBAR_PSEUDO_CLASSES: &[&str] = &[
    "horizontal",
    "vertical",
    "decrement",
    "increment",
    "start",
    "end",
    "double-button",
    "single-button",
    "no-button",
    "corner-present",
    "window-inactive",
];

pub const RESOURCE_STATE_PSEUDO_CLASSES: &[&str] = &[
    "buffering",
    "muted",
    "paused",
    "playing",
    "seeking",
    "stalled",
    "volume-locked",
];

pub const PSEUDO_CLASSES: &[&str] = &[
    // aNPlusBNotationPseudoClasses
    "nth-column",
    "nth-last-column",
    "nth-last-of-type",
    "nth-of-type",
    // linguisticPseudoClasses
    "dir",
    "lang",
    // logicalCombinationsPseudoClasses
    "has",
    "is",
    "matches",
    "not",
    "where",
    // treeStructuralPseudoClasses
    "nth-child",
    "nth-last-child",
    "empty",
    "first-child",
    "first-of-type",
    "last-child",
    "last-of-type",
    "only-child",
    "only-of-type",
    "root",
    // resourceStatePseudoClasses
    "buffering",
    "muted",
    "paused",
    "playing",
    "seeking",
    "stalled",
    "volume-locked",
    // vendorSpecificPseudoClasses
    "-khtml-drag",
    "-moz-any",
    "-moz-any-link",
    "-moz-bound-element",
    "-moz-broken",
    "-moz-dir",
    "-moz-drag-over",
    "-moz-empty-except-children-with-localname",
    "-moz-first-node",
    "-moz-focusring",
    "-moz-full-screen",
    "-moz-full-screen-ancestor",
    "-moz-is-html",
    "-moz-last-node",
    "-moz-loading",
    "-moz-meter-optimum",
    "-moz-meter-sub-optimum",
    "-moz-meter-sub-sub-optimum",
    "-moz-only-whitespace",
    "-moz-placeholder",
    "-moz-read-only",
    "-moz-read-write",
    "-moz-submit-invalid",
    "-moz-suppressed",
    "-moz-table-border-nonzero",
    "-moz-ui-invalid",
    "-moz-ui-valid",
    "-moz-user-disabled",
    "-moz-window-inactive",
    "-ms-fullscreen",
    "-ms-input-placeholder",
    "-webkit-drag",
    "-webkit-any",
    "-webkit-any-link",
    "-webkit-autofill",
    "-webkit-full-screen",
    "-webkit-full-screen-ancestor",
    // deprecatedPseudoClasses
    "contains",
    "drop",
    "focus-ring",
    "fullscreen-ancestor",
    "popup-open",
    "top-layer",
    "user-error",
    // standard
    "active-view-transition-type",
    "active-view-transition",
    "active",
    "any-link",
    "autofill",
    "blank",
    "checked",
    "current",
    "default",
    "defined",
    "disabled",
    "enabled",
    "even-less-good-value",
    "focus-visible",
    "focus-within",
    "focus",
    "fullscreen",
    "future",
    "has-slotted",
    "heading",
    "high-value",
    "host-context",
    "host",
    "hover",
    "in-range",
    "indeterminate",
    "invalid",
    "link",
    "low-value",
    "modal",
    "open",
    "optimal-value",
    "optional",
    "out-of-range",
    "past",
    "picture-in-picture",
    "placeholder-shown",
    "popover-open",
    "read-only",
    "read-write",
    "required",
    "scope",
    "state",
    "suboptimal-value",
    "target",
    "unchecked",
    "unresolved",
    "user-invalid",
    "user-valid",
    "valid",
    "visited",
    "window-inactive",
];

pub const DISABLEABLE_ELEMENTS: &[&str] = &[
    "button", "fieldset", "input", "optgroup", "option", "select", "textarea",
];
pub const CHECKABLE_ELEMENTS: &[&str] = &["input", "option"];
pub const REQUIRABLE_ELEMENTS: &[&str] = &["input", "select", "textarea"];
pub const VALIDATABLE_ELEMENTS: &[&str] =
    &["button", "fieldset", "form", "input", "select", "textarea"];
pub const RANGEABLE_ELEMENTS: &[&str] = &["input"];
pub const PLACEHOLDER_SHOWABLE_ELEMENTS: &[&str] = &["input", "textarea"];
pub const DEFAULTABLE_ELEMENTS: &[&str] = &["button", "input", "option"];
pub const AUTOFILLABLE_ELEMENTS: &[&str] = &["input", "select", "textarea"];
pub const LINK_ELEMENTS: &[&str] = &["a", "area"];

pub fn applicable_elements_by_pseudo_class(pseudo_class: &str) -> Option<&'static [&'static str]> {
    match pseudo_class {
        "any-link" | "link" | "visited" => Some(LINK_ELEMENTS),
        "autofill" => Some(AUTOFILLABLE_ELEMENTS),
        "checked" => Some(CHECKABLE_ELEMENTS),
        "default" => Some(DEFAULTABLE_ELEMENTS),
        "disabled" | "enabled" => Some(DISABLEABLE_ELEMENTS),
        "in-range" | "out-of-range" => Some(RANGEABLE_ELEMENTS),
        "invalid" | "user-invalid" | "user-valid" | "valid" => Some(VALIDATABLE_ELEMENTS),
        "optional" | "required" => Some(REQUIRABLE_ELEMENTS),
        "placeholder-shown" => Some(PLACEHOLDER_SHOWABLE_ELEMENTS),
        _ => None,
    }
}

pub const COLOR_PROFILE_DESCRIPTORS: &[&str] = &["components", "rendering-intent", "src"];

pub const COUNTER_STYLE_DESCRIPTORS: &[&str] = &[
    "additive-symbols",
    "fallback",
    "negative",
    "pad",
    "prefix",
    "range",
    "speak-as",
    "suffix",
    "symbols",
    "system",
];

pub const FONT_FACE_DESCRIPTORS: &[&str] = &[
    "-moz-font-feature-settings",
    "ascent-override",
    "descent-override",
    "font-display",
    "font-family",
    "font-feature-settings",
    "font-language-override",
    "font-named-instance",
    "font-stretch",
    "font-style",
    "font-variation-settings",
    "font-weight",
    "font-width",
    "line-gap-override",
    "size-adjust",
    "src",
    "unicode-range",
];

pub const FONT_FEATURE_VALUES_DESCRIPTORS: &[&str] = &["font-display"];

pub const FONT_PALETTE_VALUES_DESCRIPTORS: &[&str] =
    &["base-palette", "font-family", "override-colors"];

pub const PAGE_DESCRIPTORS: &[&str] =
    &["bleed", "marks", "orientation", "page-orientation", "size"];

pub const POSITION_TRY_DESCRIPTORS: &[&str] = &[
    "align-self",
    "block-size",
    "bottom",
    "height",
    "inline-size",
    "inset",
    "inset-area",
    "inset-block",
    "inset-block-end",
    "inset-block-start",
    "inset-inline",
    "inset-inline-end",
    "inset-inline-start",
    "justify-self",
    "left",
    "margin",
    "margin-block",
    "margin-block-end",
    "margin-block-start",
    "margin-bottom",
    "margin-inline",
    "margin-inline-end",
    "margin-inline-start",
    "margin-left",
    "margin-right",
    "margin-top",
    "max-block-size",
    "max-height",
    "max-inline-size",
    "max-width",
    "min-block-size",
    "min-height",
    "min-inline-size",
    "min-width",
    "place-self",
    "position-anchor",
    "position-area",
    "right",
    "top",
    "width",
];

pub const PROPERTY_DESCRIPTORS: &[&str] = &["inherits", "initial-value", "syntax"];

pub const VIEW_TRANSITION_DESCRIPTORS: &[&str] = &["navigation", "types"];

pub const VIEWPORT_DESCRIPTORS: &[&str] = &[
    "height",
    "max-height",
    "max-width",
    "max-zoom",
    "min-height",
    "min-width",
    "min-zoom",
    "orientation",
    "user-zoom",
    "viewport-fit",
    "width",
    "zoom",
];

pub fn at_rule_descriptors(at_rule_name: &str) -> Option<&'static [&'static str]> {
    match at_rule_name.to_ascii_lowercase().as_str() {
        "color-profile" => Some(COLOR_PROFILE_DESCRIPTORS),
        "counter-style" => Some(COUNTER_STYLE_DESCRIPTORS),
        "font-face" => Some(FONT_FACE_DESCRIPTORS),
        "font-feature-values" => Some(FONT_FEATURE_VALUES_DESCRIPTORS),
        "font-palette-values" => Some(FONT_PALETTE_VALUES_DESCRIPTORS),
        "page" => Some(PAGE_DESCRIPTORS),
        "position-try" => Some(POSITION_TRY_DESCRIPTORS),
        "property" => Some(PROPERTY_DESCRIPTORS),
        "view-transition" => Some(VIEW_TRANSITION_DESCRIPTORS),
        "viewport" | "-ms-viewport" | "-o-viewport" => Some(VIEWPORT_DESCRIPTORS),
        _ => None,
    }
}

/// Vendor-prefixed at-rules that can be automatically managed by Autoprefixer.
pub const AUTOPREFIXABLE_AT_RULES: &[&str] = &[
    "@-moz-document",
    "@-moz-keyframes",
    "@-ms-keyframes",
    "@-ms-viewport",
    "@-o-keyframes",
    "@-o-viewport",
    "@-webkit-keyframes",
    "@-webkit-viewport",
];

/// Vendor-prefixed pseudo-classes and pseudo-elements managed by Autoprefixer.
pub const AUTOPREFIXABLE_SELECTORS: &[&str] = &[
    "::-moz-placeholder",
    "::-moz-selection",
    "::-ms-input-placeholder",
    "::-webkit-backdrop",
    "::-webkit-input-placeholder",
    ":-moz-any-link",
    ":-moz-full-screen",
    ":-moz-placeholder",
    ":-moz-placeholder-shown",
    ":-moz-read-only",
    ":-moz-read-write",
    ":-ms-fullscreen",
    ":-ms-input-placeholder",
    ":-webkit-any-link",
    ":-webkit-full-screen",
];

/// Unprefixed standard CSS property names whose vendor prefixes are managed by Autoprefixer.
pub const AUTOPREFIXABLE_PROPERTIES: &[&str] = &[
    "align-content",
    "align-items",
    "align-self",
    "animation",
    "animation-delay",
    "animation-direction",
    "animation-duration",
    "animation-fill-mode",
    "animation-iteration-count",
    "animation-name",
    "animation-play-state",
    "animation-timing-function",
    "appearance",
    "backdrop-filter",
    "backface-visibility",
    "background-clip",
    "background-origin",
    "background-size",
    "border-block-end",
    "border-block-start",
    "border-bottom-left-radius",
    "border-bottom-right-radius",
    "border-image",
    "border-inline-end",
    "border-inline-start",
    "border-radius",
    "border-top-left-radius",
    "border-top-right-radius",
    "box-decoration-break",
    "box-shadow",
    "box-sizing",
    "break-after",
    "break-before",
    "break-inside",
    "clip-path",
    "color-adjust",
    "column-count",
    "column-fill",
    "column-gap",
    "column-rule",
    "column-rule-color",
    "column-rule-style",
    "column-rule-width",
    "column-span",
    "column-width",
    "columns",
    "filter",
    "flex",
    "flex-basis",
    "flex-direction",
    "flex-flow",
    "flex-grow",
    "flex-shrink",
    "flex-wrap",
    "flow-from",
    "flow-into",
    "font-feature-settings",
    "font-kerning",
    "font-language-override",
    "font-variant-ligatures",
    "grid-area",
    "grid-column",
    "grid-column-align",
    "grid-column-end",
    "grid-column-start",
    "grid-row",
    "grid-row-align",
    "grid-row-end",
    "grid-row-start",
    "grid-template",
    "grid-template-areas",
    "grid-template-columns",
    "grid-template-rows",
    "hyphens",
    "image-rendering",
    "justify-content",
    "margin-block-end",
    "margin-block-start",
    "margin-inline-end",
    "margin-inline-start",
    "mask",
    "mask-border",
    "mask-border-outset",
    "mask-border-repeat",
    "mask-border-slice",
    "mask-border-source",
    "mask-border-width",
    "mask-clip",
    "mask-composite",
    "mask-image",
    "mask-origin",
    "mask-position",
    "mask-repeat",
    "mask-size",
    "object-fit",
    "object-position",
    "order",
    "overscroll-behavior",
    "padding-block-end",
    "padding-block-start",
    "padding-inline-end",
    "padding-inline-start",
    "perspective",
    "perspective-origin",
    "place-self",
    "region-fragment",
    "scroll-snap-coordinate",
    "scroll-snap-destination",
    "scroll-snap-points-x",
    "scroll-snap-points-y",
    "scroll-snap-type",
    "shape-image-threshold",
    "shape-margin",
    "shape-outside",
    "tab-size",
    "text-align-last",
    "text-decoration",
    "text-decoration-color",
    "text-decoration-line",
    "text-decoration-skip",
    "text-decoration-skip-ink",
    "text-decoration-style",
    "text-emphasis",
    "text-emphasis-color",
    "text-emphasis-position",
    "text-emphasis-style",
    "text-orientation",
    "text-overflow",
    "text-size-adjust",
    "text-spacing",
    "touch-action",
    "transform",
    "transform-origin",
    "transform-style",
    "transition",
    "transition-delay",
    "transition-duration",
    "transition-property",
    "transition-timing-function",
    "user-select",
    "writing-mode",
];

/// Vendor-prefixed property values and functions that can be safely unprefixed.
pub const AUTOPREFIXABLE_PROPERTY_VALUES: &[&str] = &[
    "-moz-all",
    "-moz-arabic-indic",
    "-moz-bengali",
    "-moz-calc",
    "-moz-cjk-earthly-branch",
    "-moz-cjk-heavenly-stem",
    "-moz-crisp-edges",
    "-moz-devanagari",
    "-moz-element",
    "-moz-ethiopic-numeric",
    "-moz-fit-content",
    "-moz-grab",
    "-moz-grabbing",
    "-moz-gujarati",
    "-moz-gurmukhi",
    "-moz-hangul",
    "-moz-hangul-consonant",
    "-moz-initial",
    "-moz-isolate",
    "-moz-isolate-override",
    "-moz-japanese-formal",
    "-moz-japanese-informal",
    "-moz-kannada",
    "-moz-khmer",
    "-moz-lao",
    "-moz-linear-gradient",
    "-moz-malayalam",
    "-moz-max-content",
    "-moz-min-content",
    "-moz-myanmar",
    "-moz-oriya",
    "-moz-persian",
    "-moz-plaintext",
    "-moz-pre-wrap",
    "-moz-radial-gradient",
    "-moz-repeating-linear-gradient",
    "-moz-repeating-radial-gradient",
    "-moz-simp-chinese-formal",
    "-moz-simp-chinese-informal",
    "-moz-tamil",
    "-moz-telugu",
    "-moz-thai",
    "-moz-trad-chinese-formal",
    "-moz-trad-chinese-informal",
    "-moz-zoom-in",
    "-moz-zoom-out",
    "-ms-flexbox",
    "-ms-grid",
    "-ms-inline-grid",
    "-ms-linear-gradient",
    "-ms-radial-gradient",
    "-ms-repeating-linear-gradient",
    "-ms-repeating-radial-gradient",
    "-o-crisp-edges",
    "-o-linear-gradient",
    "-o-pre-wrap",
    "-o-radial-gradient",
    "-o-repeating-linear-gradient",
    "-o-repeating-radial-gradient",
    "-webkit-calc",
    "-webkit-cross-fade",
    "-webkit-filter",
    "-webkit-fit-content",
    "-webkit-flex",
    "-webkit-grab",
    "-webkit-grabbing",
    "-webkit-image-set",
    "-webkit-inline-flex",
    "-webkit-isolate",
    "-webkit-linear-gradient",
    "-webkit-max-content",
    "-webkit-min-content",
    "-webkit-plaintext",
    "-webkit-radial-gradient",
    "-webkit-repeating-linear-gradient",
    "-webkit-repeating-radial-gradient",
    "-webkit-sticky",
    "-webkit-zoom-in",
    "-webkit-zoom-out",
    "-xv-digits",
    "-xv-literal-punctuation",
    "-xv-no-punctuation",
];

/// Returns `true` if `name` is a vendor-prefixed at-rule name supported by Autoprefixer.
pub fn is_autoprefixable_at_rule_name(name: &str) -> bool {
    if name.starts_with('@') {
        return false;
    }
    let mut buf = String::with_capacity(name.len() + 1);
    buf.push('@');
    buf.push_str(&name.to_ascii_lowercase());
    in_set_ignore_case(AUTOPREFIXABLE_AT_RULES, &buf)
}

/// Returns `true` if `identifier` is a vendor-prefixed pseudo-class or pseudo-element
/// supported by Autoprefixer.
pub fn is_autoprefixable_selector(identifier: &str) -> bool {
    in_set_ignore_case(AUTOPREFIXABLE_SELECTORS, identifier)
}

/// Returns `true` if `media_feature` is a vendor-prefixed media feature name
/// (`-webkit-min-device-pixel-ratio`, `-moz-max-device-pixel-ratio`, etc.) supported by Autoprefixer.
pub fn is_autoprefixable_media_feature_name(media_feature: &str) -> bool {
    media_feature
        .to_ascii_lowercase()
        .contains("device-pixel-ratio")
}

/// Returns `true` if `identifier` is a vendor-prefixed CSS property name supported by Autoprefixer.
pub fn is_autoprefixable_property(identifier: &str) -> bool {
    let Some(prefix) = crate::utils::text::vendor_prefix(identifier) else {
        return false;
    };
    in_set_ignore_case(AUTOPREFIXABLE_PROPERTIES, &identifier[prefix.len()..])
}

/// Returns `true` if `value` is a vendor-prefixed CSS property value keyword supported by Autoprefixer.
pub fn is_autoprefixable_property_value(value: &str) -> bool {
    in_set_ignore_case(AUTOPREFIXABLE_PROPERTY_VALUES, value)
}

/// Maps a `flex-flow` / `grid-auto-flow` direction keyword to its `(start, end)` physical sides.
pub fn direction_flow_to_sides(flow: &str) -> Option<(&'static str, &'static str)> {
    match flow {
        "row" => Some(("left", "right")),
        "row-reverse" => Some(("right", "left")),
        "column" => Some(("top", "bottom")),
        "column-reverse" => Some(("bottom", "top")),
        _ => None,
    }
}

/// Returns `true` if `selector` is a valid HTML custom element tag name according to the
/// WHATWG HTML specification.
pub fn is_custom_element(selector: &str) -> bool {
    if selector.bytes().any(|b| b.is_ascii_uppercase())
        || !selector.contains('-')
        || !selector.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        || !selector.chars().all(is_pcen_char)
    {
        return false;
    }
    !in_set(HYPHEN_CONTAINING_TYPE_SELECTORS, selector)
}

fn is_pcen_char(c: char) -> bool {
    matches!(c, '-' | '.' | '_' | '0'..='9' | 'a'..='z')
        || matches!(
            c as u32,
            0x00B7
                | 0x00C0..=0x00D6
                | 0x00D8..=0x00F6
                | 0x00F8..=0x037D
                | 0x037F..=0x1FFF
                | 0x200C..=0x200D
                | 0x203F..=0x2040
                | 0x2070..=0x218F
                | 0x2C00..=0x2FEF
                | 0x3001..=0xD7FF
                | 0xF900..=0xFDCF
                | 0xFDF0..=0xFFFD
                | 0x10000..=0xEFFFF
        )
}

/// Returns `true` if `value` is a custom-ident token inside `counter-increment`
/// (i.e. not a reserved keyword or integer).
pub fn is_counter_increment_custom_ident_value(value: &str) -> bool {
    !in_set_ignore_case(COUNTER_INCREMENT_KEYWORDS, value) && !has_integer_prefix(value)
}

/// Returns `true` if `value` is a custom-ident token inside `counter-reset`
/// (i.e. not a reserved keyword or integer).
pub fn is_counter_reset_custom_ident_value(value: &str) -> bool {
    !in_set_ignore_case(COUNTER_RESET_KEYWORDS, value) && !has_integer_prefix(value)
}

fn has_integer_prefix(value: &str) -> bool {
    value
        .trim_start()
        .trim_start_matches(['+', '-'])
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_digit)
}

/// Returns `true` if `word` is a valid CSS `font-size` token (either a `font-size` keyword,
/// a percentage, or a `<length>` dimension).
pub fn is_valid_font_size(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    if in_set(FONT_SIZE_KEYWORDS, word) {
        return true;
    }
    let Some((_, unit)) = crate::utils::syntax::get_dimension(word) else {
        return false;
    };
    unit == "%" || in_set_ignore_case(LENGTH_UNITS, &unit)
}

/// Returns `true` if `name` (with or without leading `:`) is a context-providing
/// functional pseudo-class (`:is`, `:where`, `:not`, `:has`, `:matches`, `:-moz-any`,
/// `:-webkit-any`, `:nth-child`, `:nth-last-child`).
pub fn is_context_functional_pseudo_class(name: &str) -> bool {
    let norm = crate::utils::text::normalize_pseudo_name(name);
    in_set(LOGICAL_COMBINATIONS_PSEUDO_CLASSES, &norm)
        || in_set(AN_PLUS_B_OF_S_NOTATION_PSEUDO_CLASSES, &norm)
}

/// Returns `true` if `name` is a functional pseudo-class whose arguments are selectors
/// (`:is`, `:where`, `:not`, `:has`, `:matches`, `:-moz-any`, `:-webkit-any`, `:nth-child`,
/// `:nth-last-child`, `:host`, `:host-context`, `::slotted`).
pub fn is_selector_containing_pseudo_class(name: &str) -> bool {
    let norm = crate::utils::text::normalize_pseudo_name(name);
    is_context_functional_pseudo_class(&norm)
        || matches!(norm.as_str(), "host" | "host-context" | "slotted")
}

/// Returns `true` if `word` is a generic `font-family` keyword or CSS-wide keyword.
pub fn is_family_name_keyword(word: &str) -> bool {
    in_set_ignore_case(FONT_FAMILY_KEYWORDS, word)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLES: &[(&str, &[&str])] = &[
        ("LENGTH_UNITS", LENGTH_UNITS),
        ("RESOLUTION_UNITS", RESOLUTION_UNITS),
        ("FLOW_RELATIVE_UNITS", FLOW_RELATIVE_UNITS),
        ("UNITS", UNITS),
        ("DEPRECATED_AT_KEYWORDS", DEPRECATED_AT_KEYWORDS),
        (
            "NESTING_SUPPORTED_AT_KEYWORDS",
            NESTING_SUPPORTED_AT_KEYWORDS,
        ),
        (
            "DECLARATION_CONTAINING_AT_KEYWORDS",
            DECLARATION_CONTAINING_AT_KEYWORDS,
        ),
        ("PAGE_MARGIN_AT_KEYWORDS", PAGE_MARGIN_AT_KEYWORDS),
        ("FONT_FEATURE_VALUE_TYPES", FONT_FEATURE_VALUE_TYPES),
        ("AT_KEYWORDS", AT_KEYWORDS),
        ("CAMEL_CASE_FUNCTIONS", CAMEL_CASE_FUNCTIONS),
        (
            "HUE_AS_FIRST_COMPONENT_COLOR_FUNCTIONS",
            HUE_AS_FIRST_COMPONENT_COLOR_FUNCTIONS,
        ),
        (
            "HUE_AS_THIRD_COMPONENT_COLOR_FUNCTIONS",
            HUE_AS_THIRD_COMPONENT_COLOR_FUNCTIONS,
        ),
        ("HUE_COLOR_FUNCTIONS", HUE_COLOR_FUNCTIONS),
        (
            "LIGHTNESS_ZERO_TO_ONE_COLOR_FUNCTIONS",
            LIGHTNESS_ZERO_TO_ONE_COLOR_FUNCTIONS,
        ),
        (
            "LIGHTNESS_ZERO_TO_HUNDRED_COLOR_FUNCTIONS",
            LIGHTNESS_ZERO_TO_HUNDRED_COLOR_FUNCTIONS,
        ),
        ("LIGHTNESS_COLOR_FUNCTIONS", LIGHTNESS_COLOR_FUNCTIONS),
        (
            "WITH_ALPHA_ALIAS_COLOR_FUNCTIONS",
            WITH_ALPHA_ALIAS_COLOR_FUNCTIONS,
        ),
        (
            "WITHOUT_ALPHA_ALIAS_COLOR_FUNCTIONS",
            WITHOUT_ALPHA_ALIAS_COLOR_FUNCTIONS,
        ),
        (
            "LEGACY_NOTATION_COLOR_FUNCTIONS",
            LEGACY_NOTATION_COLOR_FUNCTIONS,
        ),
        ("COLOR_FUNCTIONS", COLOR_FUNCTIONS),
        (
            "SINGLE_ARGUMENT_MATH_FUNCTIONS",
            SINGLE_ARGUMENT_MATH_FUNCTIONS,
        ),
        ("MATH_FUNCTIONS", MATH_FUNCTIONS),
        ("DEPRECATED_MEDIA_TYPES_NAMES", DEPRECATED_MEDIA_TYPES_NAMES),
        ("PREFIXES", PREFIXES),
        (
            "DEPRECATED_MEDIA_FEATURE_NAMES",
            DEPRECATED_MEDIA_FEATURE_NAMES,
        ),
        (
            "RANGE_TYPE_MEDIA_FEATURE_NAMES",
            RANGE_TYPE_MEDIA_FEATURE_NAMES,
        ),
        (
            "RANGE_TYPE_MEDIA_FEATURE_NAMES_WITH_MIN_MAX_PREFIX",
            RANGE_TYPE_MEDIA_FEATURE_NAMES_WITH_MIN_MAX_PREFIX,
        ),
        (
            "DISCRETE_TYPE_MEDIA_FEATURE_NAMES",
            DISCRETE_TYPE_MEDIA_FEATURE_NAMES,
        ),
        ("MEDIA_FEATURE_NAMES", MEDIA_FEATURE_NAMES),
        ("BASIC_KEYWORDS", BASIC_KEYWORDS),
        ("CSS_WIDE_KEYWORDS", CSS_WIDE_KEYWORDS),
        ("DISPLAY_OUTSIDE_KEYWORDS", DISPLAY_OUTSIDE_KEYWORDS),
        ("DISPLAY_INSIDE_KEYWORDS", DISPLAY_INSIDE_KEYWORDS),
        ("DISPLAY_LEGACY_KEYWORDS", DISPLAY_LEGACY_KEYWORDS),
        ("DISPLAY_LIST_ITEM_KEYWORD", DISPLAY_LIST_ITEM_KEYWORD),
        ("SYSTEM_FONT_KEYWORDS", SYSTEM_FONT_KEYWORDS),
        ("FONT_FAMILY_KEYWORDS", FONT_FAMILY_KEYWORDS),
        ("APPLE_SYSTEM_FONTS", APPLE_SYSTEM_FONTS),
        ("MOZILLA_SYSTEM_FONTS", MOZILLA_SYSTEM_FONTS),
        ("WEBKIT_SYSTEM_FONTS", WEBKIT_SYSTEM_FONTS),
        ("PREFIXED_SYSTEM_FONTS", PREFIXED_SYSTEM_FONTS),
        (
            "FONT_WEIGHT_RELATIVE_KEYWORDS",
            FONT_WEIGHT_RELATIVE_KEYWORDS,
        ),
        (
            "FONT_WEIGHT_ABSOLUTE_KEYWORDS",
            FONT_WEIGHT_ABSOLUTE_KEYWORDS,
        ),
        (
            "FONT_WEIGHT_NON_NUMERIC_KEYWORDS",
            FONT_WEIGHT_NON_NUMERIC_KEYWORDS,
        ),
        ("FONT_WEIGHT_NUMERIC_KEYWORDS", FONT_WEIGHT_NUMERIC_KEYWORDS),
        (
            "FONT_WEIGHT_NUMERIC_KEYWORDS_WITH_NAMED_EQUIVALENT",
            FONT_WEIGHT_NUMERIC_KEYWORDS_WITH_NAMED_EQUIVALENT,
        ),
        ("FONT_WEIGHT_KEYWORDS", FONT_WEIGHT_KEYWORDS),
        ("FONT_STYLE_KEYWORDS", FONT_STYLE_KEYWORDS),
        ("FONT_VARIANT_CSS2_KEYWORDS", FONT_VARIANT_CSS2_KEYWORDS),
        ("FONT_STRETCH_KEYWORDS", FONT_STRETCH_KEYWORDS),
        ("FONT_SIZE_KEYWORDS", FONT_SIZE_KEYWORDS),
        ("LINE_HEIGHT_KEYWORDS", LINE_HEIGHT_KEYWORDS),
        ("FONT_SHORTHAND_KEYWORDS", FONT_SHORTHAND_KEYWORDS),
        ("ANIMATION_NAME_KEYWORDS", ANIMATION_NAME_KEYWORDS),
        (
            "ANIMATION_TIMING_FUNCTION_KEYWORDS",
            ANIMATION_TIMING_FUNCTION_KEYWORDS,
        ),
        (
            "ANIMATION_ITERATION_COUNT_KEYWORDS",
            ANIMATION_ITERATION_COUNT_KEYWORDS,
        ),
        ("ANIMATION_DIRECTION_KEYWORDS", ANIMATION_DIRECTION_KEYWORDS),
        ("ANIMATION_FILL_MODE_KEYWORDS", ANIMATION_FILL_MODE_KEYWORDS),
        (
            "ANIMATION_PLAY_STATE_KEYWORDS",
            ANIMATION_PLAY_STATE_KEYWORDS,
        ),
        ("ANIMATION_SHORTHAND_KEYWORDS", ANIMATION_SHORTHAND_KEYWORDS),
        ("GRID_ROW_KEYWORDS", GRID_ROW_KEYWORDS),
        ("GRID_COLUMN_KEYWORDS", GRID_COLUMN_KEYWORDS),
        ("GRID_AREA_KEYWORDS", GRID_AREA_KEYWORDS),
        ("COUNTER_INCREMENT_KEYWORDS", COUNTER_INCREMENT_KEYWORDS),
        ("COUNTER_RESET_KEYWORDS", COUNTER_RESET_KEYWORDS),
        ("LIST_STYLE_TYPE_KEYWORDS", LIST_STYLE_TYPE_KEYWORDS),
        ("LIST_STYLE_POSITION_KEYWORDS", LIST_STYLE_POSITION_KEYWORDS),
        ("LIST_STYLE_IMAGE_KEYWORDS", LIST_STYLE_IMAGE_KEYWORDS),
        (
            "LIST_STYLE_SHORTHAND_KEYWORDS",
            LIST_STYLE_SHORTHAND_KEYWORDS,
        ),
        ("CAMEL_CASE_KEYWORDS", CAMEL_CASE_KEYWORDS),
        ("KEYFRAME_SELECTOR_KEYWORDS", KEYFRAME_SELECTOR_KEYWORDS),
        (
            "PREFIXED_SYSTEM_COLOR_KEYWORDS",
            PREFIXED_SYSTEM_COLOR_KEYWORDS,
        ),
        (
            "DEPRECATED_SYSTEM_COLOR_KEYWORDS",
            DEPRECATED_SYSTEM_COLOR_KEYWORDS,
        ),
        ("SYSTEM_COLORS_KEYWORDS", SYSTEM_COLORS_KEYWORDS),
        ("NAMED_COLORS_KEYWORDS", NAMED_COLORS_KEYWORDS),
        (
            "ACCEPT_CUSTOM_IDENTS_PROPERTIES",
            ACCEPT_CUSTOM_IDENTS_PROPERTIES,
        ),
        (
            "ACCEPT_PROPERTY_NAMES_PROPERTIES",
            ACCEPT_PROPERTY_NAMES_PROPERTIES,
        ),
        ("SHORTHAND_PROPERTIES", SHORTHAND_PROPERTIES),
        (
            "SINGLE_VALUE_COLOR_PROPERTIES",
            SINGLE_VALUE_COLOR_PROPERTIES,
        ),
        ("MULTI_VALUE_COLOR_PROPERTIES", MULTI_VALUE_COLOR_PROPERTIES),
        ("COLOR_PROPERTIES", COLOR_PROPERTIES),
        ("LONGHAND_TIME_PROPERTIES", LONGHAND_TIME_PROPERTIES),
        ("SHORTHAND_TIME_PROPERTIES", SHORTHAND_TIME_PROPERTIES),
        ("PAGE_CONTEXT_PROPERTIES", PAGE_CONTEXT_PROPERTIES),
        ("MARGIN_CONTEXT_PROPERTIES", MARGIN_CONTEXT_PROPERTIES),
        ("PREVIOUSLY_KNOWN_PROPERTIES", PREVIOUSLY_KNOWN_PROPERTIES),
        (
            "PREVIOUSLY_KNOWN_PREFIXED_PROPERTIES",
            PREVIOUSLY_KNOWN_PREFIXED_PROPERTIES,
        ),
        ("FLOW_RELATIVE_PROPERTIES", FLOW_RELATIVE_PROPERTIES),
        (
            "DEPRECATED_HTML_TYPE_SELECTORS",
            DEPRECATED_HTML_TYPE_SELECTORS,
        ),
        ("STANDARD_HTML_TYPE_SELECTORS", STANDARD_HTML_TYPE_SELECTORS),
        (
            "EXPERIMENTAL_HTML_TYPE_SELECTORS",
            EXPERIMENTAL_HTML_TYPE_SELECTORS,
        ),
        ("HTML_TYPE_SELECTORS", HTML_TYPE_SELECTORS),
        (
            "DEPRECATED_SVG_TYPE_SELECTORS",
            DEPRECATED_SVG_TYPE_SELECTORS,
        ),
        ("STANDARD_SVG_TYPE_SELECTORS", STANDARD_SVG_TYPE_SELECTORS),
        (
            "EXPERIMENTAL_SVG_TYPE_SELECTORS",
            EXPERIMENTAL_SVG_TYPE_SELECTORS,
        ),
        ("SVG_TYPE_SELECTORS", SVG_TYPE_SELECTORS),
        (
            "MIXED_CASE_SVG_TYPE_SELECTORS",
            MIXED_CASE_SVG_TYPE_SELECTORS,
        ),
        ("MATHML_TYPE_SELECTORS", MATHML_TYPE_SELECTORS),
        (
            "HYPHEN_CONTAINING_TYPE_SELECTORS",
            HYPHEN_CONTAINING_TYPE_SELECTORS,
        ),
        (
            "LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS",
            LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS,
        ),
        ("SHADOW_TREE_PSEUDO_ELEMENTS", SHADOW_TREE_PSEUDO_ELEMENTS),
        (
            "ELEMENT_BACKED_PSEUDO_ELEMENTS",
            ELEMENT_BACKED_PSEUDO_ELEMENTS,
        ),
        ("SUB_PSEUDO_ELEMENTS", SUB_PSEUDO_ELEMENTS),
        (
            "ELEMENT_REPRESENTING_PSEUDO_ELEMENTS",
            ELEMENT_REPRESENTING_PSEUDO_ELEMENTS,
        ),
        (
            "WEBKIT_SCROLLBAR_PSEUDO_ELEMENTS",
            WEBKIT_SCROLLBAR_PSEUDO_ELEMENTS,
        ),
        (
            "VENDOR_SPECIFIC_PSEUDO_ELEMENTS",
            VENDOR_SPECIFIC_PSEUDO_ELEMENTS,
        ),
        ("DEPRECATED_PSEUDO_ELEMENTS", DEPRECATED_PSEUDO_ELEMENTS),
        ("PSEUDO_ELEMENTS", PSEUDO_ELEMENTS),
        (
            "PERCENTAGE_KEYFRAME_WITH_NAMED_EQUIVALENT",
            PERCENTAGE_KEYFRAME_WITH_NAMED_EQUIVALENT,
        ),
        (
            "AN_PLUS_B_NOTATION_PSEUDO_CLASSES",
            AN_PLUS_B_NOTATION_PSEUDO_CLASSES,
        ),
        (
            "AN_PLUS_B_OF_S_NOTATION_PSEUDO_CLASSES",
            AN_PLUS_B_OF_S_NOTATION_PSEUDO_CLASSES,
        ),
        (
            "TREE_STRUCTURAL_PSEUDO_CLASSES",
            TREE_STRUCTURAL_PSEUDO_CLASSES,
        ),
        ("AT_RULE_PAGE_PSEUDO_CLASSES", AT_RULE_PAGE_PSEUDO_CLASSES),
        ("LINGUISTIC_PSEUDO_CLASSES", LINGUISTIC_PSEUDO_CLASSES),
        ("DIR_IDENTIFIERS", DIR_IDENTIFIERS),
        ("FORGIVING_PSEUDO_CLASSES", FORGIVING_PSEUDO_CLASSES),
        (
            "LOGICAL_COMBINATIONS_PSEUDO_CLASSES",
            LOGICAL_COMBINATIONS_PSEUDO_CLASSES,
        ),
        ("DEPRECATED_PSEUDO_CLASSES", DEPRECATED_PSEUDO_CLASSES),
        (
            "VENDOR_SPECIFIC_PSEUDO_CLASSES",
            VENDOR_SPECIFIC_PSEUDO_CLASSES,
        ),
        (
            "WEBKIT_SCROLLBAR_PSEUDO_CLASSES",
            WEBKIT_SCROLLBAR_PSEUDO_CLASSES,
        ),
        (
            "RESOURCE_STATE_PSEUDO_CLASSES",
            RESOURCE_STATE_PSEUDO_CLASSES,
        ),
        ("PSEUDO_CLASSES", PSEUDO_CLASSES),
        ("DISABLEABLE_ELEMENTS", DISABLEABLE_ELEMENTS),
        ("CHECKABLE_ELEMENTS", CHECKABLE_ELEMENTS),
        ("REQUIRABLE_ELEMENTS", REQUIRABLE_ELEMENTS),
        ("VALIDATABLE_ELEMENTS", VALIDATABLE_ELEMENTS),
        ("RANGEABLE_ELEMENTS", RANGEABLE_ELEMENTS),
        (
            "PLACEHOLDER_SHOWABLE_ELEMENTS",
            PLACEHOLDER_SHOWABLE_ELEMENTS,
        ),
        ("DEFAULTABLE_ELEMENTS", DEFAULTABLE_ELEMENTS),
        ("AUTOFILLABLE_ELEMENTS", AUTOFILLABLE_ELEMENTS),
        ("LINK_ELEMENTS", LINK_ELEMENTS),
        ("COLOR_PROFILE_DESCRIPTORS", COLOR_PROFILE_DESCRIPTORS),
        ("COUNTER_STYLE_DESCRIPTORS", COUNTER_STYLE_DESCRIPTORS),
        ("FONT_FACE_DESCRIPTORS", FONT_FACE_DESCRIPTORS),
        (
            "FONT_FEATURE_VALUES_DESCRIPTORS",
            FONT_FEATURE_VALUES_DESCRIPTORS,
        ),
        (
            "FONT_PALETTE_VALUES_DESCRIPTORS",
            FONT_PALETTE_VALUES_DESCRIPTORS,
        ),
        ("PAGE_DESCRIPTORS", PAGE_DESCRIPTORS),
        ("POSITION_TRY_DESCRIPTORS", POSITION_TRY_DESCRIPTORS),
        ("PROPERTY_DESCRIPTORS", PROPERTY_DESCRIPTORS),
        ("VIEW_TRANSITION_DESCRIPTORS", VIEW_TRANSITION_DESCRIPTORS),
        ("VIEWPORT_DESCRIPTORS", VIEWPORT_DESCRIPTORS),
        ("AUTOPREFIXABLE_AT_RULES", AUTOPREFIXABLE_AT_RULES),
        ("AUTOPREFIXABLE_SELECTORS", AUTOPREFIXABLE_SELECTORS),
        ("AUTOPREFIXABLE_PROPERTIES", AUTOPREFIXABLE_PROPERTIES),
        (
            "AUTOPREFIXABLE_PROPERTY_VALUES",
            AUTOPREFIXABLE_PROPERTY_VALUES,
        ),
    ];

    const MIXED_CASE_TABLES: &[&str] = &[
        "UNITS",
        "CAMEL_CASE_FUNCTIONS",
        "CAMEL_CASE_KEYWORDS",
        "DEPRECATED_SVG_TYPE_SELECTORS",
        "STANDARD_SVG_TYPE_SELECTORS",
        "SVG_TYPE_SELECTORS",
        "MIXED_CASE_SVG_TYPE_SELECTORS",
    ];

    fn assert_bool(f: fn(&str) -> bool, cases: &[(&str, bool)]) {
        for &(input, expected) in cases {
            assert_eq!(f(input), expected, "input: {input:?}");
        }
    }

    fn assert_subset(name: &str, sub: &[&str], sup: &[&str]) {
        for s in sub {
            assert!(sup.contains(s), "{name}: {s:?} missing");
        }
    }

    fn assert_same_set(name: &str, a: &[&str], b: &[&str]) {
        assert_eq!(a.len(), b.len(), "{name}: length mismatch");
        assert_subset(name, a, b);
        assert_subset(name, b, a);
    }

    #[test]
    fn tables_have_no_duplicates() {
        for &(name, table) in TABLES {
            let mut seen = std::collections::HashSet::new();
            for entry in table {
                assert!(seen.insert(*entry), "{name}: duplicate {entry:?}");
            }
        }
    }

    #[test]
    fn tables_have_no_empty_or_padded_entries() {
        for &(name, table) in TABLES {
            assert!(!table.is_empty(), "{name}: empty table");
            for entry in table {
                assert!(!entry.is_empty(), "{name}: empty entry");
                assert_eq!(entry.trim(), *entry, "{name}: padded {entry:?}");
            }
        }
    }

    #[test]
    fn tables_are_lowercase_except_mixed_case_tables() {
        for &(name, table) in TABLES {
            let has_upper = table
                .iter()
                .any(|e| e.bytes().any(|b| b.is_ascii_uppercase()));
            assert_eq!(has_upper, MIXED_CASE_TABLES.contains(&name), "{name}");
        }
    }

    #[test]
    fn tables_sample_upstream_entries() {
        let cases: &[(&str, &[&str], &[&str])] = &[
            (
                "LENGTH_UNITS",
                LENGTH_UNITS,
                &["px", "rem", "cqmin", "dvh", "mozmm", "fr", "q"],
            ),
            (
                "RESOLUTION_UNITS",
                RESOLUTION_UNITS,
                &["dpi", "dpcm", "dppx", "x"],
            ),
            (
                "FLOW_RELATIVE_UNITS",
                FLOW_RELATIVE_UNITS,
                &["vi", "vb", "cqi", "cqb"],
            ),
            (
                "UNITS",
                UNITS,
                &["%", "s", "ms", "deg", "turn", "Hz", "kHz", "dppx"],
            ),
            (
                "DEPRECATED_AT_KEYWORDS",
                DEPRECATED_AT_KEYWORDS,
                &["document", "nest", "viewport"],
            ),
            (
                "NESTING_SUPPORTED_AT_KEYWORDS",
                NESTING_SUPPORTED_AT_KEYWORDS,
                &[
                    "container",
                    "layer",
                    "media",
                    "scope",
                    "starting-style",
                    "supports",
                ],
            ),
            (
                "PAGE_MARGIN_AT_KEYWORDS",
                PAGE_MARGIN_AT_KEYWORDS,
                &["top-left-corner", "bottom-center", "right-bottom"],
            ),
            (
                "FONT_FEATURE_VALUE_TYPES",
                FONT_FEATURE_VALUE_TYPES,
                &["annotation", "styleset", "swash"],
            ),
            (
                "AT_KEYWORDS",
                AT_KEYWORDS,
                &[
                    "apply",
                    "font-face",
                    "import",
                    "keyframes",
                    "namespace",
                    "property",
                    "position-try",
                    "view-transition",
                ],
            ),
            (
                "CAMEL_CASE_FUNCTIONS",
                CAMEL_CASE_FUNCTIONS,
                &["translateX", "scaleY", "rotateZ", "skewX"],
            ),
            (
                "COLOR_FUNCTIONS",
                COLOR_FUNCTIONS,
                &["rgb", "hsla", "oklch", "color-mix", "light-dark", "color"],
            ),
            (
                "MATH_FUNCTIONS",
                MATH_FUNCTIONS,
                &["calc", "calc-size", "clamp", "atan2", "round", "random"],
            ),
            (
                "SINGLE_ARGUMENT_MATH_FUNCTIONS",
                SINGLE_ARGUMENT_MATH_FUNCTIONS,
                &["abs", "sqrt", "sign"],
            ),
            (
                "DEPRECATED_MEDIA_TYPES_NAMES",
                DEPRECATED_MEDIA_TYPES_NAMES,
                &["aural", "tty", "tv"],
            ),
            ("PREFIXES", PREFIXES, &["-webkit-", "-moz-", "-ms-", "-o-"]),
            (
                "RANGE_TYPE_MEDIA_FEATURE_NAMES",
                RANGE_TYPE_MEDIA_FEATURE_NAMES,
                &["width", "aspect-ratio", "resolution", "device-width"],
            ),
            (
                "RANGE_TYPE_MEDIA_FEATURE_NAMES_WITH_MIN_MAX_PREFIX",
                RANGE_TYPE_MEDIA_FEATURE_NAMES_WITH_MIN_MAX_PREFIX,
                &["min-width", "max-resolution", "min-color-index"],
            ),
            (
                "DISCRETE_TYPE_MEDIA_FEATURE_NAMES",
                DISCRETE_TYPE_MEDIA_FEATURE_NAMES,
                &["hover", "prefers-color-scheme", "scripting", "update"],
            ),
            (
                "MEDIA_FEATURE_NAMES",
                MEDIA_FEATURE_NAMES,
                &["width", "min-width", "hover", "device-aspect-ratio"],
            ),
            (
                "BASIC_KEYWORDS",
                BASIC_KEYWORDS,
                &["initial", "inherit", "revert", "revert-layer", "unset"],
            ),
            (
                "SYSTEM_FONT_KEYWORDS",
                SYSTEM_FONT_KEYWORDS,
                &[
                    "caption",
                    "icon",
                    "menu",
                    "message-box",
                    "small-caption",
                    "status-bar",
                ],
            ),
            (
                "FONT_FAMILY_KEYWORDS",
                FONT_FAMILY_KEYWORDS,
                &[
                    "serif",
                    "sans-serif",
                    "monospace",
                    "system-ui",
                    "math",
                    "inherit",
                ],
            ),
            (
                "PREFIXED_SYSTEM_FONTS",
                PREFIXED_SYSTEM_FONTS,
                &["-apple-system", "-moz-button", "-webkit-body"],
            ),
            (
                "FONT_WEIGHT_KEYWORDS",
                FONT_WEIGHT_KEYWORDS,
                &["normal", "bold", "bolder", "100", "900"],
            ),
            (
                "FONT_SIZE_KEYWORDS",
                FONT_SIZE_KEYWORDS,
                &[
                    "xx-small",
                    "xxx-large",
                    "smaller",
                    "math",
                    "-webkit-xxx-large",
                ],
            ),
            (
                "FONT_SHORTHAND_KEYWORDS",
                FONT_SHORTHAND_KEYWORDS,
                &["italic", "small-caps", "condensed", "large", "normal"],
            ),
            (
                "ANIMATION_SHORTHAND_KEYWORDS",
                ANIMATION_SHORTHAND_KEYWORDS,
                &[
                    "ease-in-out",
                    "infinite",
                    "alternate",
                    "forwards",
                    "paused",
                    "step-start",
                ],
            ),
            ("GRID_ROW_KEYWORDS", GRID_ROW_KEYWORDS, &["auto", "span"]),
            (
                "COUNTER_INCREMENT_KEYWORDS",
                COUNTER_INCREMENT_KEYWORDS,
                &["none", "unset"],
            ),
            (
                "LIST_STYLE_TYPE_KEYWORDS",
                LIST_STYLE_TYPE_KEYWORDS,
                &["disc", "decimal", "lower-roman", "upper-alpha", "georgian"],
            ),
            (
                "LIST_STYLE_SHORTHAND_KEYWORDS",
                LIST_STYLE_SHORTHAND_KEYWORDS,
                &["inside", "outside", "none", "square"],
            ),
            (
                "CAMEL_CASE_KEYWORDS",
                CAMEL_CASE_KEYWORDS,
                &["currentColor", "optimizeSpeed", "geometricPrecision"],
            ),
            (
                "KEYFRAME_SELECTOR_KEYWORDS",
                KEYFRAME_SELECTOR_KEYWORDS,
                &["from", "to"],
            ),
            (
                "SYSTEM_COLORS_KEYWORDS",
                SYSTEM_COLORS_KEYWORDS,
                &[
                    "canvas",
                    "canvastext",
                    "buttonface",
                    "activeborder",
                    "-moz-buttondefault",
                ],
            ),
            (
                "NAMED_COLORS_KEYWORDS",
                NAMED_COLORS_KEYWORDS,
                &["aliceblue", "rebeccapurple", "yellowgreen"],
            ),
            (
                "ACCEPT_CUSTOM_IDENTS_PROPERTIES",
                ACCEPT_CUSTOM_IDENTS_PROPERTIES,
                &[
                    "animation-name",
                    "font-family",
                    "counter-increment",
                    "list-style-type",
                ],
            ),
            (
                "ACCEPT_PROPERTY_NAMES_PROPERTIES",
                ACCEPT_PROPERTY_NAMES_PROPERTIES,
                &["transition", "transition-property", "will-change"],
            ),
            (
                "SHORTHAND_PROPERTIES",
                SHORTHAND_PROPERTIES,
                &[
                    "margin",
                    "border",
                    "font",
                    "grid-template",
                    "scroll-padding-inline",
                ],
            ),
            (
                "SINGLE_VALUE_COLOR_PROPERTIES",
                SINGLE_VALUE_COLOR_PROPERTIES,
                &["color", "background-color", "outline-color", "caret-color"],
            ),
            (
                "COLOR_PROPERTIES",
                COLOR_PROPERTIES,
                &["color", "border-color", "scrollbar-color"],
            ),
            (
                "LONGHAND_TIME_PROPERTIES",
                LONGHAND_TIME_PROPERTIES,
                &["transition-duration", "animation-delay"],
            ),
            (
                "PAGE_CONTEXT_PROPERTIES",
                PAGE_CONTEXT_PROPERTIES,
                &["margin", "direction", "font-size"],
            ),
            (
                "MARGIN_CONTEXT_PROPERTIES",
                MARGIN_CONTEXT_PROPERTIES,
                &["content", "vertical-align", "padding"],
            ),
            (
                "PREVIOUSLY_KNOWN_PROPERTIES",
                PREVIOUSLY_KNOWN_PROPERTIES,
                &["accelerator", "app-region"],
            ),
            (
                "PREVIOUSLY_KNOWN_PREFIXED_PROPERTIES",
                PREVIOUSLY_KNOWN_PREFIXED_PROPERTIES,
                &["-webkit-wrap", "-epub-text-combine"],
            ),
            (
                "FLOW_RELATIVE_PROPERTIES",
                FLOW_RELATIVE_PROPERTIES,
                &[
                    "margin-inline-start",
                    "inline-size",
                    "border-start-start-radius",
                    "inset-block",
                ],
            ),
            (
                "HTML_TYPE_SELECTORS",
                HTML_TYPE_SELECTORS,
                &["div", "a", "acronym", "marquee", "template", "search"],
            ),
            (
                "SVG_TYPE_SELECTORS",
                SVG_TYPE_SELECTORS,
                &["svg", "clipPath", "feGaussianBlur", "altGlyph", "hatch"],
            ),
            (
                "MIXED_CASE_SVG_TYPE_SELECTORS",
                MIXED_CASE_SVG_TYPE_SELECTORS,
                &["clipPath", "linearGradient", "textPath"],
            ),
            (
                "MATHML_TYPE_SELECTORS",
                MATHML_TYPE_SELECTORS,
                &["math", "mrow", "abs", "annotation-xml"],
            ),
            (
                "HYPHEN_CONTAINING_TYPE_SELECTORS",
                HYPHEN_CONTAINING_TYPE_SELECTORS,
                &["annotation-xml", "font-face", "missing-glyph"],
            ),
            (
                "LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS",
                LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS,
                &["before", "after", "first-line", "first-letter"],
            ),
            (
                "PSEUDO_ELEMENTS",
                PSEUDO_ELEMENTS,
                &[
                    "before",
                    "placeholder",
                    "selection",
                    "part",
                    "slotted",
                    "-webkit-scrollbar",
                    "view-transition",
                ],
            ),
            (
                "WEBKIT_SCROLLBAR_PSEUDO_ELEMENTS",
                WEBKIT_SCROLLBAR_PSEUDO_ELEMENTS,
                &[
                    "-webkit-scrollbar",
                    "-webkit-scrollbar-thumb",
                    "-webkit-resizer",
                ],
            ),
            (
                "AN_PLUS_B_NOTATION_PSEUDO_CLASSES",
                AN_PLUS_B_NOTATION_PSEUDO_CLASSES,
                &[
                    "nth-column",
                    "nth-last-column",
                    "nth-last-of-type",
                    "nth-of-type",
                ],
            ),
            (
                "TREE_STRUCTURAL_PSEUDO_CLASSES",
                TREE_STRUCTURAL_PSEUDO_CLASSES,
                &["root", "empty", "first-child", "only-of-type", "nth-child"],
            ),
            (
                "AT_RULE_PAGE_PSEUDO_CLASSES",
                AT_RULE_PAGE_PSEUDO_CLASSES,
                &["first", "right", "left", "blank"],
            ),
            (
                "PSEUDO_CLASSES",
                PSEUDO_CLASSES,
                &[
                    "hover",
                    "focus-visible",
                    "has",
                    "is",
                    "where",
                    "not",
                    "nth-child",
                    "host",
                    "popover-open",
                ],
            ),
            (
                "DEPRECATED_PSEUDO_CLASSES",
                DEPRECATED_PSEUDO_CLASSES,
                &["matches"],
            ),
            (
                "WEBKIT_SCROLLBAR_PSEUDO_CLASSES",
                WEBKIT_SCROLLBAR_PSEUDO_CLASSES,
                &["horizontal", "vertical", "decrement", "window-inactive"],
            ),
            (
                "AUTOPREFIXABLE_AT_RULES",
                AUTOPREFIXABLE_AT_RULES,
                &["@-moz-keyframes", "@-ms-viewport", "@-moz-document"],
            ),
            (
                "AUTOPREFIXABLE_SELECTORS",
                AUTOPREFIXABLE_SELECTORS,
                &[":-moz-any-link", "::-webkit-backdrop", "::-moz-selection"],
            ),
            (
                "AUTOPREFIXABLE_PROPERTIES",
                AUTOPREFIXABLE_PROPERTIES,
                &["transition", "user-select", "appearance", "writing-mode"],
            ),
            (
                "AUTOPREFIXABLE_PROPERTY_VALUES",
                AUTOPREFIXABLE_PROPERTY_VALUES,
                &[
                    "-webkit-max-content",
                    "-moz-calc",
                    "-ms-flexbox",
                    "-moz-thai",
                ],
            ),
        ];
        for &(name, table, samples) in cases {
            assert_subset(name, samples, table);
        }
    }

    #[test]
    fn tables_compose_like_upstream_unions() {
        let cat = |parts: &[&[&'static str]]| -> Vec<&'static str> {
            let mut out: Vec<&str> = Vec::new();
            for part in parts {
                for s in *part {
                    if !out.contains(s) {
                        out.push(s);
                    }
                }
            }
            out
        };
        let mixed: Vec<&str> = SVG_TYPE_SELECTORS
            .iter()
            .copied()
            .filter(|s| *s != s.to_lowercase())
            .collect();
        let flow_units: Vec<&str> = UNITS
            .iter()
            .filter_map(|u| physical_to_flow_relative_unit(u))
            .flat_map(|(a, b)| [a, b])
            .collect();
        let cases: &[(&str, &[&str], Vec<&str>)] = &[
            (
                "CSS_WIDE_KEYWORDS",
                CSS_WIDE_KEYWORDS,
                cat(&[BASIC_KEYWORDS]),
            ),
            (
                "GRID_COLUMN_KEYWORDS",
                GRID_COLUMN_KEYWORDS,
                cat(&[GRID_ROW_KEYWORDS]),
            ),
            (
                "GRID_AREA_KEYWORDS",
                GRID_AREA_KEYWORDS,
                cat(&[GRID_ROW_KEYWORDS]),
            ),
            (
                "COUNTER_RESET_KEYWORDS",
                COUNTER_RESET_KEYWORDS,
                cat(&[COUNTER_INCREMENT_KEYWORDS]),
            ),
            (
                "HUE_COLOR_FUNCTIONS",
                HUE_COLOR_FUNCTIONS,
                cat(&[
                    HUE_AS_FIRST_COMPONENT_COLOR_FUNCTIONS,
                    HUE_AS_THIRD_COMPONENT_COLOR_FUNCTIONS,
                ]),
            ),
            (
                "LIGHTNESS_COLOR_FUNCTIONS",
                LIGHTNESS_COLOR_FUNCTIONS,
                cat(&[
                    LIGHTNESS_ZERO_TO_ONE_COLOR_FUNCTIONS,
                    LIGHTNESS_ZERO_TO_HUNDRED_COLOR_FUNCTIONS,
                ]),
            ),
            (
                "LEGACY_NOTATION_COLOR_FUNCTIONS",
                LEGACY_NOTATION_COLOR_FUNCTIONS,
                cat(&[
                    WITH_ALPHA_ALIAS_COLOR_FUNCTIONS,
                    WITHOUT_ALPHA_ALIAS_COLOR_FUNCTIONS,
                ]),
            ),
            (
                "MEDIA_FEATURE_NAMES",
                MEDIA_FEATURE_NAMES,
                cat(&[
                    DEPRECATED_MEDIA_FEATURE_NAMES,
                    RANGE_TYPE_MEDIA_FEATURE_NAMES,
                    RANGE_TYPE_MEDIA_FEATURE_NAMES_WITH_MIN_MAX_PREFIX,
                    DISCRETE_TYPE_MEDIA_FEATURE_NAMES,
                ]),
            ),
            (
                "PREFIXED_SYSTEM_FONTS",
                PREFIXED_SYSTEM_FONTS,
                cat(&[
                    APPLE_SYSTEM_FONTS,
                    MOZILLA_SYSTEM_FONTS,
                    WEBKIT_SYSTEM_FONTS,
                ]),
            ),
            (
                "FONT_WEIGHT_NON_NUMERIC_KEYWORDS",
                FONT_WEIGHT_NON_NUMERIC_KEYWORDS,
                cat(&[FONT_WEIGHT_RELATIVE_KEYWORDS, FONT_WEIGHT_ABSOLUTE_KEYWORDS]),
            ),
            (
                "FONT_WEIGHT_KEYWORDS",
                FONT_WEIGHT_KEYWORDS,
                cat(&[
                    BASIC_KEYWORDS,
                    FONT_WEIGHT_NON_NUMERIC_KEYWORDS,
                    FONT_WEIGHT_NUMERIC_KEYWORDS,
                ]),
            ),
            (
                "FONT_SHORTHAND_KEYWORDS",
                FONT_SHORTHAND_KEYWORDS,
                cat(&[
                    BASIC_KEYWORDS,
                    FONT_STYLE_KEYWORDS,
                    FONT_VARIANT_CSS2_KEYWORDS,
                    FONT_WEIGHT_KEYWORDS,
                    FONT_STRETCH_KEYWORDS,
                    FONT_SIZE_KEYWORDS,
                    LINE_HEIGHT_KEYWORDS,
                    FONT_FAMILY_KEYWORDS,
                ]),
            ),
            (
                "ANIMATION_SHORTHAND_KEYWORDS",
                ANIMATION_SHORTHAND_KEYWORDS,
                cat(&[
                    BASIC_KEYWORDS,
                    ANIMATION_NAME_KEYWORDS,
                    ANIMATION_TIMING_FUNCTION_KEYWORDS,
                    ANIMATION_ITERATION_COUNT_KEYWORDS,
                    ANIMATION_DIRECTION_KEYWORDS,
                    ANIMATION_FILL_MODE_KEYWORDS,
                    ANIMATION_PLAY_STATE_KEYWORDS,
                ]),
            ),
            (
                "LIST_STYLE_SHORTHAND_KEYWORDS",
                LIST_STYLE_SHORTHAND_KEYWORDS,
                cat(&[
                    BASIC_KEYWORDS,
                    LIST_STYLE_TYPE_KEYWORDS,
                    LIST_STYLE_POSITION_KEYWORDS,
                    LIST_STYLE_IMAGE_KEYWORDS,
                ]),
            ),
            (
                "COLOR_PROPERTIES",
                COLOR_PROPERTIES,
                cat(&[SINGLE_VALUE_COLOR_PROPERTIES, MULTI_VALUE_COLOR_PROPERTIES]),
            ),
            (
                "HTML_TYPE_SELECTORS",
                HTML_TYPE_SELECTORS,
                cat(&[
                    DEPRECATED_HTML_TYPE_SELECTORS,
                    STANDARD_HTML_TYPE_SELECTORS,
                    EXPERIMENTAL_HTML_TYPE_SELECTORS,
                ]),
            ),
            (
                "SVG_TYPE_SELECTORS",
                SVG_TYPE_SELECTORS,
                cat(&[
                    DEPRECATED_SVG_TYPE_SELECTORS,
                    STANDARD_SVG_TYPE_SELECTORS,
                    EXPERIMENTAL_SVG_TYPE_SELECTORS,
                ]),
            ),
            (
                "MIXED_CASE_SVG_TYPE_SELECTORS",
                MIXED_CASE_SVG_TYPE_SELECTORS,
                mixed,
            ),
            (
                "FLOW_RELATIVE_UNITS",
                FLOW_RELATIVE_UNITS,
                cat(&[&flow_units]),
            ),
        ];
        for (name, actual, expected) in cases {
            assert_same_set(name, actual, expected);
        }
        let subsets: &[(&str, &[&str], &[&str])] = &[
            (
                "BASIC_KEYWORDS in FONT_FAMILY_KEYWORDS",
                BASIC_KEYWORDS,
                FONT_FAMILY_KEYWORDS,
            ),
            (
                "BASIC_KEYWORDS in SYSTEM_FONT_KEYWORDS",
                BASIC_KEYWORDS,
                SYSTEM_FONT_KEYWORDS,
            ),
            (
                "BASIC_KEYWORDS in FONT_SIZE_KEYWORDS",
                BASIC_KEYWORDS,
                FONT_SIZE_KEYWORDS,
            ),
            (
                "DEPRECATED_AT_KEYWORDS in AT_KEYWORDS",
                DEPRECATED_AT_KEYWORDS,
                AT_KEYWORDS,
            ),
            (
                "NESTING_SUPPORTED_AT_KEYWORDS in AT_KEYWORDS",
                NESTING_SUPPORTED_AT_KEYWORDS,
                AT_KEYWORDS,
            ),
            (
                "PAGE_MARGIN_AT_KEYWORDS in AT_KEYWORDS",
                PAGE_MARGIN_AT_KEYWORDS,
                AT_KEYWORDS,
            ),
            (
                "FONT_FEATURE_VALUE_TYPES in AT_KEYWORDS",
                FONT_FEATURE_VALUE_TYPES,
                AT_KEYWORDS,
            ),
            ("LENGTH_UNITS in UNITS", LENGTH_UNITS, UNITS),
            ("RESOLUTION_UNITS in UNITS", RESOLUTION_UNITS, UNITS),
            (
                "SINGLE_ARGUMENT_MATH_FUNCTIONS in MATH_FUNCTIONS",
                SINGLE_ARGUMENT_MATH_FUNCTIONS,
                MATH_FUNCTIONS,
            ),
            (
                "DEPRECATED_SYSTEM_COLOR_KEYWORDS in SYSTEM_COLORS_KEYWORDS",
                DEPRECATED_SYSTEM_COLOR_KEYWORDS,
                SYSTEM_COLORS_KEYWORDS,
            ),
            (
                "PREFIXED_SYSTEM_COLOR_KEYWORDS in SYSTEM_COLORS_KEYWORDS",
                PREFIXED_SYSTEM_COLOR_KEYWORDS,
                SYSTEM_COLORS_KEYWORDS,
            ),
            (
                "AN_PLUS_B_OF_S in TREE_STRUCTURAL",
                AN_PLUS_B_OF_S_NOTATION_PSEUDO_CLASSES,
                TREE_STRUCTURAL_PSEUDO_CLASSES,
            ),
            (
                "AN_PLUS_B in PSEUDO_CLASSES",
                AN_PLUS_B_NOTATION_PSEUDO_CLASSES,
                PSEUDO_CLASSES,
            ),
            (
                "LOGICAL_COMBINATIONS in PSEUDO_CLASSES",
                LOGICAL_COMBINATIONS_PSEUDO_CLASSES,
                PSEUDO_CLASSES,
            ),
            (
                "FORGIVING in LOGICAL_COMBINATIONS",
                FORGIVING_PSEUDO_CLASSES,
                LOGICAL_COMBINATIONS_PSEUDO_CLASSES,
            ),
            (
                "LINGUISTIC in PSEUDO_CLASSES",
                LINGUISTIC_PSEUDO_CLASSES,
                PSEUDO_CLASSES,
            ),
            (
                "LEVEL_ONE_AND_TWO in PSEUDO_ELEMENTS",
                LEVEL_ONE_AND_TWO_PSEUDO_ELEMENTS,
                PSEUDO_ELEMENTS,
            ),
            (
                "ELEMENT_REPRESENTING in PSEUDO_ELEMENTS",
                ELEMENT_REPRESENTING_PSEUDO_ELEMENTS,
                PSEUDO_ELEMENTS,
            ),
            (
                "VENDOR_SPECIFIC in PSEUDO_ELEMENTS",
                VENDOR_SPECIFIC_PSEUDO_ELEMENTS,
                PSEUDO_ELEMENTS,
            ),
            (
                "SHADOW_TREE in ELEMENT_BACKED",
                SHADOW_TREE_PSEUDO_ELEMENTS,
                ELEMENT_BACKED_PSEUDO_ELEMENTS,
            ),
            (
                "ELEMENT_BACKED in ELEMENT_REPRESENTING",
                ELEMENT_BACKED_PSEUDO_ELEMENTS,
                ELEMENT_REPRESENTING_PSEUDO_ELEMENTS,
            ),
        ];
        for &(name, sub, sup) in subsets {
            assert_subset(name, sub, sup);
        }
    }

    #[test]
    fn in_set_is_case_sensitive() {
        let set = &["foo", "Bar"];
        let cases = [
            ("foo", true),
            ("Bar", true),
            ("FOO", false),
            ("bar", false),
            ("", false),
            ("baz", false),
        ];
        for (input, expected) in cases {
            assert_eq!(in_set(set, input), expected, "{input:?}");
        }
        assert!(!in_set(&[], "foo"));
    }

    #[test]
    fn in_set_ignore_case_ignores_ascii_case() {
        let set = &["foo", "Bar"];
        let cases = [
            ("foo", true),
            ("FOO", true),
            ("bar", true),
            ("bAR", true),
            ("", false),
            ("fo", false),
        ];
        for (input, expected) in cases {
            assert_eq!(in_set_ignore_case(set, input), expected, "{input:?}");
        }
        assert!(!in_set_ignore_case(&[], "foo"));
    }

    #[test]
    fn physical_to_flow_relative_unit_maps_physical_units() {
        let cases = [
            ("vw", Some(("vi", "vb"))),
            ("vh", Some(("vb", "vi"))),
            ("svw", Some(("svi", "svb"))),
            ("svh", Some(("svb", "svi"))),
            ("lvw", Some(("lvi", "lvb"))),
            ("lvh", Some(("lvb", "lvi"))),
            ("dvw", Some(("dvi", "dvb"))),
            ("dvh", Some(("dvb", "dvi"))),
            ("cqw", Some(("cqi", "cqb"))),
            ("cqh", Some(("cqb", "cqi"))),
            ("vi", None),
            ("px", None),
            ("vmin", None),
            ("VW", None),
            ("", None),
        ];
        for (input, expected) in cases {
            assert_eq!(physical_to_flow_relative_unit(input), expected, "{input:?}");
        }
    }

    #[test]
    fn is_math_function_matches_upstream() {
        assert_bool(
            is_math_function,
            &[
                ("calc", true),
                ("cALc", true),
                ("calc-size", true),
                ("var", false),
            ],
        );
        assert_bool(
            is_math_function,
            &[
                ("CLAMP", true),
                ("min", true),
                ("round", true),
                ("env", false),
                ("", false),
                ("calc(", false),
            ],
        );
    }

    #[test]
    fn media_feature_name_allowed_value_keywords_lookup() {
        let cases: &[(&str, Option<&[&str]>)] = &[
            ("hover", Some(&["none", "hover"])),
            ("any-pointer", Some(&["none", "coarse", "fine"])),
            ("prefers-color-scheme", Some(&["light", "dark"])),
            (
                "prefers-contrast",
                Some(&["no-preference", "less", "more", "custom"]),
            ),
            ("resolution", Some(&["infinite"])),
            ("scripting", Some(&["none", "initial-only", "enabled"])),
            (
                "display-mode",
                Some(&[
                    "fullscreen",
                    "standalone",
                    "minimal-ui",
                    "browser",
                    "picture-in-picture",
                ]),
            ),
            ("width", None),
            ("grid", None),
            ("HOVER", None),
            ("", None),
        ];
        for &(input, expected) in cases {
            assert_eq!(
                media_feature_name_allowed_value_keywords(input),
                expected,
                "{input:?}"
            );
        }
        for name in DISCRETE_TYPE_MEDIA_FEATURE_NAMES {
            if !matches!(*name, "grid" | "light-level") {
                assert!(
                    media_feature_name_allowed_value_keywords(name).is_some(),
                    "{name}"
                );
            }
        }
    }

    #[test]
    fn media_feature_name_allowed_value_types_lookup() {
        let cases: &[(&str, Option<&[&str]>)] = &[
            ("aspect-ratio", Some(&["ratio"])),
            ("device-aspect-ratio", Some(&["ratio"])),
            ("color", Some(&["integer"])),
            ("monochrome", Some(&["integer"])),
            ("width", Some(&["length"])),
            ("device-height", Some(&["length"])),
            ("resolution", Some(&["resolution"])),
            ("grid", Some(&["mq-boolean"])),
            ("min-width", None),
            ("hover", None),
            ("WIDTH", None),
            ("", None),
        ];
        for &(input, expected) in cases {
            assert_eq!(
                media_feature_name_allowed_value_types(input),
                expected,
                "{input:?}"
            );
        }
        for name in RANGE_TYPE_MEDIA_FEATURE_NAMES {
            assert!(
                media_feature_name_allowed_value_types(name).is_some(),
                "{name}"
            );
        }
    }

    #[test]
    fn is_css_wide_keyword_ignores_case() {
        assert_bool(
            is_css_wide_keyword,
            &[
                ("initial", true),
                ("inherit", true),
                ("revert", true),
                ("revert-layer", true),
                ("unset", true),
                ("INHERIT", true),
                ("Unset", true),
                ("auto", false),
                ("none", false),
                ("default", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn physical_to_flow_relative_value_keyword_lookup() {
        let cases = [
            ("float", "left", Some("inline-start")),
            ("float", "right", Some("inline-end")),
            ("clear", "left", Some("inline-start")),
            ("clear", "right", Some("inline-end")),
            ("clear", "both", None),
            ("offset-anchor", "top", Some("block-start")),
            ("offset-position", "bottom", Some("block-end")),
            ("offset-position", "left", Some("inline-start")),
            ("offset-anchor", "right", Some("inline-end")),
            ("resize", "horizontal", Some("inline")),
            ("resize", "vertical", Some("block")),
            ("resize", "both", None),
            ("text-align", "left", Some("start")),
            ("text-align-last", "right", Some("end")),
            ("text-align", "center", None),
            ("float", "top", None),
            ("margin", "left", None),
            ("FLOAT", "left", None),
            ("float", "LEFT", None),
        ];
        for (prop, kw, expected) in cases {
            assert_eq!(
                physical_to_flow_relative_value_keyword(prop, kw),
                expected,
                "{prop}: {kw}"
            );
        }
    }

    #[test]
    fn flow_relative_value_keywords_for_property_lookup() {
        let cases: &[(&str, Option<&[&str]>)] = &[
            ("clear", Some(&["inline-start", "inline-end"])),
            ("float", Some(&["inline-start", "inline-end"])),
            (
                "offset-anchor",
                Some(&["block-start", "block-end", "inline-start", "inline-end"]),
            ),
            (
                "offset-position",
                Some(&["block-start", "block-end", "inline-start", "inline-end"]),
            ),
            ("resize", Some(&["inline", "block"])),
            ("text-align", Some(&["start", "end"])),
            ("text-align-last", Some(&["start", "end"])),
            ("margin", None),
            ("Float", None),
            ("", None),
        ];
        for &(input, expected) in cases {
            assert_eq!(
                flow_relative_value_keywords_for_property(input),
                expected,
                "{input:?}"
            );
        }
        for (prop, kw) in [
            ("float", "left"),
            ("offset-anchor", "top"),
            ("resize", "vertical"),
            ("text-align", "right"),
        ] {
            let mapped = physical_to_flow_relative_value_keyword(prop, kw).unwrap();
            assert!(
                flow_relative_value_keywords_for_property(prop)
                    .unwrap()
                    .contains(&mapped)
            );
        }
    }

    #[test]
    fn shorthand_to_reset_to_initial_property_lookup() {
        let border = shorthand_to_reset_to_initial_property("border").unwrap();
        assert_eq!(border.len(), 6);
        assert!(border.contains(&"border-image") && border.contains(&"border-image-width"));
        let font = shorthand_to_reset_to_initial_property("font").unwrap();
        assert_eq!(font.len(), 13);
        assert!(font.contains(&"font-kerning") && font.contains(&"font-variation-settings"));
        for input in ["margin", "border-top", "BORDER", ""] {
            assert_eq!(
                shorthand_to_reset_to_initial_property(input),
                None,
                "{input:?}"
            );
        }
    }

    #[test]
    fn physical_shorthand_to_flow_relative_properties_lookup() {
        let cases: &[(&str, usize, &[&str])] = &[
            ("border", 24, &["border-block", "border-inline-end-color"]),
            (
                "border-color",
                6,
                &["border-block-color", "border-inline-end-color"],
            ),
            (
                "border-radius",
                4,
                &["border-start-start-radius", "border-end-end-radius"],
            ),
            ("border-style", 6, &["border-inline-style"]),
            ("border-width", 6, &["border-block-start-width"]),
            (
                "contain-intrinsic-size",
                2,
                &[
                    "contain-intrinsic-block-size",
                    "contain-intrinsic-inline-size",
                ],
            ),
            ("inset", 6, &["inset-block", "inset-inline-end"]),
            ("margin", 6, &["margin-block", "margin-inline-start"]),
            ("overflow", 2, &["overflow-block", "overflow-inline"]),
            ("overscroll-behavior", 2, &["overscroll-behavior-block"]),
            ("padding", 6, &["padding-inline"]),
            ("scroll-margin", 6, &["scroll-margin-block-end"]),
            ("scroll-padding", 6, &["scroll-padding-inline-start"]),
        ];
        for &(input, len, samples) in cases {
            let props = physical_shorthand_to_flow_relative_properties(input).unwrap();
            assert_eq!(props.len(), len, "{input}");
            assert_subset(input, samples, props);
            assert_subset(input, props, FLOW_RELATIVE_PROPERTIES);
        }
        for input in ["margin-left", "font", "MARGIN", ""] {
            assert_eq!(
                physical_shorthand_to_flow_relative_properties(input),
                None,
                "{input:?}"
            );
        }
    }

    #[test]
    fn longhand_sub_properties_of_shorthand_lookup() {
        let cases: &[(&str, &[&str])] = &[
            (
                "margin",
                &["margin-top", "margin-right", "margin-bottom", "margin-left"],
            ),
            ("flex", &["flex-grow", "flex-shrink", "flex-basis"]),
            ("flex-flow", &["flex-direction", "flex-wrap"]),
            ("place-items", &["align-items", "justify-items"]),
            (
                "scroll-padding-inline",
                &["scroll-padding-inline-start", "scroll-padding-inline-end"],
            ),
        ];
        for &(input, expected) in cases {
            assert_same_set(
                input,
                longhand_sub_properties_of_shorthand(input).unwrap(),
                expected,
            );
        }
        let animation = longhand_sub_properties_of_shorthand("animation").unwrap();
        assert_eq!(animation.len(), 8);
        assert!(animation.contains(&"animation-play-state"));
        for shorthand in SHORTHAND_PROPERTIES {
            assert!(
                longhand_sub_properties_of_shorthand(shorthand).is_some(),
                "{shorthand}"
            );
        }
        for input in ["margin-top", "color", "MARGIN", ""] {
            assert_eq!(
                longhand_sub_properties_of_shorthand(input),
                None,
                "{input:?}"
            );
        }
    }

    #[test]
    fn physical_to_flow_relative_property_lookup() {
        let cases = [
            ("margin-left", Some("margin-inline-start")),
            ("margin-bottom", Some("margin-block-end")),
            ("padding-right", Some("padding-inline-end")),
            ("border-top", Some("border-block-start")),
            ("border-left-width", Some("border-inline-start-width")),
            ("border-bottom-style", Some("border-block-end-style")),
            ("border-right-color", Some("border-inline-end-color")),
            ("left", Some("inset-inline-start")),
            ("top", Some("inset-block-start")),
            ("width", Some("inline-size")),
            ("max-height", Some("max-block-size")),
            ("scroll-margin-top", Some("scroll-margin-block-start")),
            ("scroll-padding-left", Some("scroll-padding-inline-start")),
            ("border-top-left-radius", Some("border-start-start-radius")),
            ("border-bottom-right-radius", Some("border-end-end-radius")),
            ("overflow-x", Some("overflow-inline")),
            ("overscroll-behavior-y", Some("overscroll-behavior-block")),
            (
                "contain-intrinsic-width",
                Some("contain-intrinsic-inline-size"),
            ),
            ("margin", None),
            ("margin-inline-start", None),
            ("Margin-Left", None),
            ("", None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                physical_to_flow_relative_property(input),
                expected,
                "{input:?}"
            );
        }
        for physical in [
            "margin-left",
            "padding-top",
            "border-right-style",
            "min-width",
            "overflow-y",
        ] {
            let flow = physical_to_flow_relative_property(physical).unwrap();
            assert!(FLOW_RELATIVE_PROPERTIES.contains(&flow), "{flow}");
        }
    }

    #[test]
    fn physical_shorthand_to_flow_relative_by_axis_lookup() {
        let cases = [
            (
                "border-color",
                Some(("border-block-color", "border-inline-color", false)),
            ),
            (
                "border-style",
                Some(("border-block-style", "border-inline-style", false)),
            ),
            (
                "border-width",
                Some(("border-block-width", "border-inline-width", false)),
            ),
            ("inset", Some(("inset-block", "inset-inline", false))),
            ("margin", Some(("margin-block", "margin-inline", false))),
            (
                "overflow",
                Some(("overflow-block", "overflow-inline", true)),
            ),
            (
                "overscroll-behavior",
                Some((
                    "overscroll-behavior-block",
                    "overscroll-behavior-inline",
                    true,
                )),
            ),
            ("padding", Some(("padding-block", "padding-inline", false))),
            (
                "scroll-margin",
                Some(("scroll-margin-block", "scroll-margin-inline", false)),
            ),
            (
                "scroll-padding",
                Some(("scroll-padding-block", "scroll-padding-inline", false)),
            ),
            ("border", None),
            ("border-radius", None),
            ("MARGIN", None),
            ("", None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                physical_shorthand_to_flow_relative_by_axis(input),
                expected,
                "{input:?}"
            );
        }
    }

    #[test]
    fn applicable_elements_by_pseudo_class_lookup() {
        let cases: &[(&str, Option<&[&str]>)] = &[
            ("any-link", Some(LINK_ELEMENTS)),
            ("link", Some(&["a", "area"])),
            ("visited", Some(LINK_ELEMENTS)),
            ("autofill", Some(&["input", "select", "textarea"])),
            ("checked", Some(&["input", "option"])),
            ("default", Some(&["button", "input", "option"])),
            ("disabled", Some(DISABLEABLE_ELEMENTS)),
            ("enabled", Some(DISABLEABLE_ELEMENTS)),
            ("in-range", Some(&["input"])),
            ("out-of-range", Some(RANGEABLE_ELEMENTS)),
            ("invalid", Some(VALIDATABLE_ELEMENTS)),
            ("user-invalid", Some(VALIDATABLE_ELEMENTS)),
            ("user-valid", Some(VALIDATABLE_ELEMENTS)),
            (
                "valid",
                Some(&["button", "fieldset", "form", "input", "select", "textarea"]),
            ),
            ("optional", Some(REQUIRABLE_ELEMENTS)),
            ("required", Some(&["input", "select", "textarea"])),
            ("placeholder-shown", Some(&["input", "textarea"])),
            ("hover", None),
            ("Checked", None),
            (":checked", None),
            ("", None),
        ];
        for &(input, expected) in cases {
            assert_eq!(
                applicable_elements_by_pseudo_class(input),
                expected,
                "{input:?}"
            );
        }
    }

    #[test]
    fn at_rule_descriptors_lookup() {
        let cases: &[(&str, Option<&[&str]>)] = &[
            ("color-profile", Some(COLOR_PROFILE_DESCRIPTORS)),
            ("counter-style", Some(COUNTER_STYLE_DESCRIPTORS)),
            ("font-face", Some(FONT_FACE_DESCRIPTORS)),
            ("FONT-FACE", Some(FONT_FACE_DESCRIPTORS)),
            ("font-feature-values", Some(FONT_FEATURE_VALUES_DESCRIPTORS)),
            ("font-palette-values", Some(FONT_PALETTE_VALUES_DESCRIPTORS)),
            ("page", Some(PAGE_DESCRIPTORS)),
            ("Page", Some(PAGE_DESCRIPTORS)),
            ("position-try", Some(POSITION_TRY_DESCRIPTORS)),
            ("property", Some(PROPERTY_DESCRIPTORS)),
            ("view-transition", Some(VIEW_TRANSITION_DESCRIPTORS)),
            ("viewport", Some(VIEWPORT_DESCRIPTORS)),
            ("-ms-viewport", Some(VIEWPORT_DESCRIPTORS)),
            ("-o-viewport", Some(VIEWPORT_DESCRIPTORS)),
            ("media", None),
            ("@font-face", None),
            ("-webkit-viewport", None),
            ("", None),
        ];
        for &(input, expected) in cases {
            assert_eq!(at_rule_descriptors(input), expected, "{input:?}");
        }
        let samples: &[(&str, &[&str])] = &[
            (
                "font-face",
                &["src", "font-display", "unicode-range", "size-adjust"],
            ),
            (
                "counter-style",
                &["system", "symbols", "additive-symbols", "speak-as"],
            ),
            ("property", &["syntax", "inherits", "initial-value"]),
            ("page", &["size", "marks", "bleed"]),
            ("position-try", &["top", "position-area", "margin-inline"]),
        ];
        for &(at_rule, expected) in samples {
            assert_subset(at_rule, expected, at_rule_descriptors(at_rule).unwrap());
        }
    }

    #[test]
    fn is_autoprefixable_at_rule_name_matches_upstream() {
        assert_bool(
            is_autoprefixable_at_rule_name,
            &[
                ("-moz-keyframes", true),
                ("-ms-viewport", true),
                ("-moz-document", true),
                ("keyframes", false),
                ("viewport", false),
            ],
        );
        assert_bool(
            is_autoprefixable_at_rule_name,
            &[
                ("-WEBKIT-Keyframes", true),
                ("-o-viewport", true),
                ("-webkit-document", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn is_autoprefixable_selector_matches_upstream() {
        assert_bool(
            is_autoprefixable_selector,
            &[
                (":-moz-any-link", true),
                ("::-webkit-backdrop", true),
                (":-ms-input-placeholder", true),
                ("::-webkit-input-placeholder", true),
                ("::placeholder", false),
                (":any-link", false),
            ],
        );
        assert_bool(
            is_autoprefixable_selector,
            &[
                (":-MOZ-Full-Screen", true),
                ("-moz-any-link", false),
                ("::-moz-any-link", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn is_autoprefixable_media_feature_name_prefixed() {
        assert_bool(
            is_autoprefixable_media_feature_name,
            &[
                ("-webkit-min-device-pixel-ratio", true),
                ("-webkit-device-pixel-ratio", true),
                ("-moz-max-device-pixel-ratio", true),
                ("-o-min-device-pixel-ratio", true),
                ("-WEBKIT-MIN-DEVICE-PIXEL-RATIO", true),
                ("-webkit-min-width", false),
                ("width", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn is_autoprefixable_property_matches_upstream() {
        assert_bool(
            is_autoprefixable_property,
            &[
                ("-webkit-transition", true),
                ("transition", false),
                ("-webkit-touch-callout", false),
                ("-ms-interpolation-mode", false),
            ],
        );
        assert_bool(
            is_autoprefixable_property,
            &[
                ("-moz-user-select", true),
                ("-ms-flex", true),
                ("-WEBKIT-Transform", true),
                ("--transition", false),
                ("-webkit-", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn is_autoprefixable_property_value_matches_upstream() {
        assert_bool(
            is_autoprefixable_property_value,
            &[("-webkit-max-content", true), ("max-content", false)],
        );
        assert_bool(
            is_autoprefixable_property_value,
            &[
                ("-MOZ-Calc", true),
                ("-ms-grid", true),
                ("-moz-thai", true),
                ("-webkit-box", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn direction_flow_to_sides_lookup() {
        let cases = [
            ("row", Some(("left", "right"))),
            ("row-reverse", Some(("right", "left"))),
            ("column", Some(("top", "bottom"))),
            ("column-reverse", Some(("bottom", "top"))),
            ("wrap", None),
            ("ROW", None),
            ("", None),
        ];
        for (input, expected) in cases {
            assert_eq!(direction_flow_to_sides(input), expected, "{input:?}");
        }
    }

    #[test]
    fn is_custom_element_matches_upstream() {
        assert_bool(
            is_custom_element,
            &[
                ("x-foo", true),
                ("X-foo", false),
                ("x-Foo", false),
                ("X-FOO", false),
                (".foo", false),
                (".foo-bar", false),
                ("#foo", false),
                ("#foo-bar", false),
                (":foo", false),
                (":foo-bar", false),
                ("::foo", false),
                ("::foo-bar", false),
                ("div", false),
                ("dIv", false),
                ("DiV", false),
                ("DIV", false),
                ("foo", false),
                ("acronym", false),
                ("font-face", false),
                ("clipPath", false),
                ("abs", false),
                ("annotation-xml", false),
                ("applet", false),
            ],
        );
        assert_bool(
            is_custom_element,
            &[
                ("my-element", true),
                ("a-", true),
                ("missing-glyph", false),
                ("1-foo", false),
                ("-foo", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn is_custom_element_non_ascii() {
        assert_bool(is_custom_element, &[("math-α", true), ("emotion-😍", true)]);
    }

    #[test]
    fn is_counter_increment_custom_ident_value_matches_upstream() {
        assert_bool(
            is_counter_increment_custom_ident_value,
            &[
                ("counter", true),
                ("cOuNtEr", true),
                ("COUNTER", true),
                ("counter-name", true),
                ("counter1", true),
                ("counter2", true),
                ("none", false),
                ("inherit", false),
                ("initial", false),
                ("unset", false),
                ("-1", false),
                ("1", false),
            ],
        );
        assert_bool(
            is_counter_increment_custom_ident_value,
            &[("NONE", false), ("revert-layer", false), ("+2", false)],
        );
    }

    #[test]
    fn is_counter_reset_custom_ident_value_matches_upstream() {
        assert_bool(
            is_counter_reset_custom_ident_value,
            &[
                ("counter", true),
                ("cOuNtEr", true),
                ("COUNTER", true),
                ("counter-name", true),
                ("counter1", true),
                ("counter2", true),
                ("none", false),
                ("inherit", false),
                ("initial", false),
                ("unset", false),
                ("-1", false),
                ("1", false),
            ],
        );
        assert_bool(
            is_counter_reset_custom_ident_value,
            &[("Inherit", false), ("revert", false), ("0", false)],
        );
    }

    #[test]
    fn counter_custom_ident_numeric_prefix() {
        for f in [
            is_counter_increment_custom_ident_value,
            is_counter_reset_custom_ident_value,
        ] {
            assert_bool(f, &[("1foo", false), ("1.5", false)]);
        }
    }

    #[test]
    fn is_valid_font_size_matches_upstream() {
        assert_bool(
            is_valid_font_size,
            &[
                ("10px", true),
                ("20%", true),
                ("20", false),
                ("small", true),
                ("smaller", true),
                ("smallest", false),
                ("math", true),
                ("", false),
            ],
        );
        assert_bool(
            is_valid_font_size,
            &[
                ("1.5em", true),
                ("2REM", true),
                ("10cqi", true),
                ("10deg", false),
                ("10s", false),
                ("inherit", true),
                ("px", false),
            ],
        );
    }

    #[test]
    fn is_valid_font_size_keyword_case() {
        assert_bool(is_valid_font_size, &[("SMALL", false), ("Medium", false)]);
    }

    #[test]
    fn is_context_functional_pseudo_class_matches_upstream() {
        assert_bool(
            is_context_functional_pseudo_class,
            &[
                (":hover", false),
                (":has", true),
                (":is", true),
                (":matches", true),
                (":not", true),
                (":where", true),
                (":nth-child", true),
                (":nth-last-child", true),
            ],
        );
        assert_bool(
            is_context_functional_pseudo_class,
            &[
                ("is", true),
                (":NOT", true),
                ("::where", true),
                (":nth-of-type", false),
                (":host", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn is_selector_containing_pseudo_class_lookup() {
        assert_bool(
            is_selector_containing_pseudo_class,
            &[
                (":is", true),
                (":not", true),
                (":has", true),
                (":nth-child", true),
                (":host", true),
                (":host-context", true),
                ("::slotted", true),
                (":HOST", true),
                ("slotted", true),
                (":hover", false),
                ("::before", false),
                (":nth-of-type", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn is_family_name_keyword_matches_upstream() {
        assert_bool(
            is_family_name_keyword,
            &[
                ("serif", true),
                ("SANS-SERIF", true),
                ("monospace", true),
                ("\"serif\"", false),
                ("Arial", false),
            ],
        );
        assert_bool(
            is_family_name_keyword,
            &[
                ("system-ui", true),
                ("ui-monospace", true),
                ("inherit", true),
                ("Cursive", true),
                ("helvetica", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn is_autoprefixable_media_feature_name_device_pixel_ratio() {
        assert_bool(
            is_autoprefixable_media_feature_name,
            &[
                ("min-device-pixel-ratio", true),
                ("-ms-device-pixel-ratio", true),
            ],
        );
    }

    #[test]
    fn is_autoprefixable_at_rule_name_at_sign() {
        assert_bool(
            is_autoprefixable_at_rule_name,
            &[("@-moz-keyframes", false)],
        );
    }
}
