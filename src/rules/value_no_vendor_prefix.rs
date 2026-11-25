use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{ValueTokenKind, declaration_value_span, tokenize_value},
};
use oxc_css_parser::{
    Span,
    ast::{Declaration, InterpolableIdent},
};
use regex::Regex;
use serde_json::{Map, Value};

enum IgnorePattern {
    Exact(String),
    Regex(Regex),
}

impl IgnorePattern {
    fn parse(s: &str) -> Option<Self> {
        if let Some(rest) = s.strip_prefix('/') {
            if let Some(pattern) = rest.strip_suffix('/') {
                return Regex::new(pattern).ok().map(Self::Regex);
            }
            if let Some(pattern) = rest.strip_suffix("/i") {
                return Regex::new(&format!("(?i){pattern}")).ok().map(Self::Regex);
            }
        }
        Some(Self::Exact(s.to_string()))
    }

    fn is_match(&self, candidate: &str) -> bool {
        match self {
            Self::Exact(s) => s == candidate,
            Self::Regex(re) => re.is_match(candidate),
        }
    }
}

crate::define_stx_lint! {
    /// Disallows vendor prefixes on a known set of value keywords and functions.
    ///
    /// Set to `true`. `ignoreValues` accepts a string or a list of strings and `/regex/`
    /// patterns; use function names without parentheses. Auto-fix removes the prefix
    /// from reported keywords and function names, not from property names.
    ///
    /// Example: `.foo { display: -webkit-flex; }` is fixed to `.foo { display: flex; }`.
    "value-no-vendor-prefix", restriction, fix = fix;
    #[derive(Default)]
    pub struct ValueNoVendorPrefix {
        ignore_values: Vec<IgnorePattern>,
    }
}

const PREFIXED_VALUES: &[&str] = &[
    "-webkit-flex",
    "-ms-flex",
    "-ms-flexbox",
    "-webkit-inline-flex",
    "-ms-inline-flex",
    "-ms-inline-flexbox",
    "-webkit-grid",
    "-ms-grid",
    "-webkit-inline-grid",
    "-ms-inline-grid",
    "-webkit-grab",
    "-moz-grab",
    "-webkit-grabbing",
    "-moz-grabbing",
    "-webkit-zoom-in",
    "-moz-zoom-in",
    "-webkit-zoom-out",
    "-moz-zoom-out",
    "-webkit-sticky",
    "-webkit-fit-content",
    "-moz-fit-content",
    "-webkit-max-content",
    "-moz-max-content",
    "-webkit-min-content",
    "-moz-min-content",
    "-webkit-fill-available",
    "-moz-available",
    "-webkit-isolate",
    "-moz-isolate",
    "-ms-isolate",
    "-webkit-isolate-override",
    "-moz-isolate-override",
    "-webkit-plaintext",
    "-moz-plaintext",
    "-o-crisp-edges",
    "-moz-crisp-edges",
    "-webkit-crisp-edges",
    "-o-pre-wrap",
    "-moz-pre-wrap",
    "-moz-all",
    "-moz-none",
    "-webkit-none",
    "-ms-none",
    "-moz-text",
    "-webkit-text",
    "-ms-text",
    "-moz-contain",
    "-webkit-contain",
    "-moz-hangul",
    "-moz-hangul-consonant",
    "-moz-arabic-indic",
    "-moz-bengali",
    "-moz-cjk-earthly-branch",
    "-moz-cjk-heavenly-stem",
    "-moz-devanagari",
    "-moz-gujarati",
    "-moz-gurmukhi",
    "-moz-kannada",
    "-moz-khmer",
    "-moz-lao",
    "-moz-malayalam",
    "-moz-myanmar",
    "-moz-oriya",
    "-moz-persian",
    "-moz-tamil",
    "-moz-telugu",
    "-moz-thai",
    "-moz-trad-chinese-formal",
    "-moz-trad-chinese-informal",
    "-moz-simp-chinese-formal",
    "-moz-simp-chinese-informal",
    "-moz-japanese-formal",
    "-moz-japanese-informal",
    "-moz-urdu",
    "-xv-digits",
    "-xv-literal-punctuation",
    "-xv-no-punctuation",
];

const PREFIXED_FUNCTIONS: &[&str] = &[
    "-webkit-linear-gradient",
    "-moz-linear-gradient",
    "-o-linear-gradient",
    "-ms-linear-gradient",
    "-webkit-radial-gradient",
    "-moz-radial-gradient",
    "-o-radial-gradient",
    "-ms-radial-gradient",
    "-webkit-repeating-linear-gradient",
    "-moz-repeating-linear-gradient",
    "-o-repeating-linear-gradient",
    "-ms-repeating-linear-gradient",
    "-webkit-repeating-radial-gradient",
    "-moz-repeating-radial-gradient",
    "-o-repeating-radial-gradient",
    "-ms-repeating-radial-gradient",
    "-webkit-conic-gradient",
    "-moz-conic-gradient",
    "-o-conic-gradient",
    "-ms-conic-gradient",
    "-webkit-repeating-conic-gradient",
    "-moz-repeating-conic-gradient",
    "-o-repeating-conic-gradient",
    "-ms-repeating-conic-gradient",
    "-webkit-calc",
    "-moz-calc",
    "-webkit-image-set",
    "-moz-image-set",
    "-o-image-set",
    "-ms-image-set",
    "-webkit-cross-fade",
    "-webkit-element",
    "-moz-element",
    "-webkit-filter",
];

fn strip_vendor_prefix(s: &str) -> Option<&str> {
    let lower = s.to_ascii_lowercase();
    for prefix in [
        "-webkit-", "-moz-", "-ms-", "-o-", "-xv-", "-mso-", "-atsc-", "-khtml-",
    ] {
        if lower.starts_with(prefix) {
            return Some(&s[prefix.len()..]);
        }
    }
    None
}

impl ValueNoVendorPrefix {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        let mut ignore_values = Vec::new();
        if let Some(sec) = secondary
            && let Some(val) = sec.get("ignoreValues")
        {
            match val {
                Value::String(s) => {
                    if let Some(pat) = IgnorePattern::parse(s) {
                        ignore_values.push(pat);
                    }
                }
                Value::Array(arr) => {
                    for item in arr {
                        if let Value::String(s) = item
                            && let Some(pat) = IgnorePattern::parse(s)
                        {
                            ignore_values.push(pat);
                        }
                    }
                }
                _ => {}
            }
        }
        Self { ignore_values }
    }

    fn is_ignored(&self, val: &str) -> bool {
        self.ignore_values.iter().any(|p| p.is_match(val))
    }
}

impl Rule for ValueNoVendorPrefix {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        if ident.name.starts_with('$') || ident.name.starts_with('@') {
            return;
        }
        let imp_start = decl.important.as_ref().map(|i| i.span.start);
        let Some(val_span) =
            declaration_value_span(ctx.source, ident.span.end, decl.span.end, imp_start)
        else {
            return;
        };
        let tokens = tokenize_value(ctx.source, val_span);
        for tok in tokens {
            match tok.kind {
                ValueTokenKind::Ident => {
                    if PREFIXED_VALUES
                        .iter()
                        .any(|v| v.eq_ignore_ascii_case(tok.raw))
                    {
                        if self.is_ignored(tok.raw) {
                            continue;
                        }
                        if let Some(unprefixed) = strip_vendor_prefix(tok.raw) {
                            ctx.report_with_fix(
                                self.name(),
                                tok.span,
                                format!("Vendor-prefixed value \"{}\"", tok.raw),
                                Some(Fix::single(tok.span, unprefixed.to_string())),
                            );
                        }
                    }
                }
                ValueTokenKind::Function => {
                    let fn_name = &tok.raw[..tok.raw.len() - 1];
                    if PREFIXED_FUNCTIONS
                        .iter()
                        .any(|f| f.eq_ignore_ascii_case(fn_name))
                    {
                        if self.is_ignored(fn_name) {
                            continue;
                        }
                        if let Some(unprefixed) = strip_vendor_prefix(fn_name) {
                            let fn_span = Span {
                                start: tok.span.start,
                                end: tok.span.end - 1,
                            };
                            ctx.report_with_fix(
                                self.name(),
                                fn_span,
                                format!("Vendor-prefixed value \"{}\"", fn_name),
                                Some(Fix::single(fn_span, unprefixed.to_string())),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::tester::{Tester, reject};

    const NAME: &str = super::ValueNoVendorPrefix::NAME;

    fn rejected(value: &str) -> String {
        format!("Vendor-prefixed value \"{value}\"")
    }

    #[test]
    fn default_config() {
        let pass = [
            "a { display: flex; }",
            ".foo { display: -webkit-box; }",
            ".foo { display: -moz-box; }",
            "a { background: linear-gradient(to top, #000, #fff); }",
            "a { max-width: max-content; }",
            "a { -webkit-transform: translate(0, 0); }",
            "a { -wEbKiT-tRaNsFoRm: translate(0, 0); }",
            "a { -WEBKIT-TRANSFORM: translate(0, 0); }",
            "a { white-space: -pre-wrap; }",
            "a { background-color: -apple-wireless-playback-target-active; }",
            "a { display: -wap-marquee; }",
            "a { list-style-type: -moz-ethiopic-halehame; }",
            "a { display: -webkit-box /*-webkit-flex*/; }",
            "a { content: \"-webkit-flex\"; }",
        ];
        let fail = [
            reject(
                ".foo { display: -webkit-flex; }",
                ".foo { display: flex; }",
                rejected("-webkit-flex"),
                (1, 17, 1, 29),
            )
            .with_edit_info(0, (16, 24, "")),
            reject(
                ".foo { display: -wEbKiT-fLeX; }",
                ".foo { display: fLeX; }",
                rejected("-wEbKiT-fLeX"),
                (1, 17, 1, 29),
            )
            .with_edit_info(0, (16, 24, "")),
            reject(
                ".foo { display: -WEBKIT-FLEX; }",
                ".foo { display: FLEX; }",
                rejected("-WEBKIT-FLEX"),
                (1, 17, 1, 29),
            )
            .with_edit_info(0, (16, 24, "")),
            reject(
                ".foo { dIsPlAy: -webkit-flex; }",
                ".foo { dIsPlAy: flex; }",
                rejected("-webkit-flex"),
                (1, 17, 1, 29),
            )
            .with_edit_info(0, (16, 24, "")),
            reject(
                ".foo { DISPLAY: -webkit-flex; }",
                ".foo { DISPLAY: flex; }",
                rejected("-webkit-flex"),
                (1, 17, 1, 29),
            )
            .with_edit_info(0, (16, 24, "")),
            reject(
                ".foo { color: pink; display: -webkit-flex; }",
                ".foo { color: pink; display: flex; }",
                rejected("-webkit-flex"),
                (1, 30, 1, 42),
            )
            .with_edit_info(0, (29, 37, "")),
            reject(
                ".foo { image-rendering: -o-crisp-edges; }",
                ".foo { image-rendering: crisp-edges; }",
                rejected("-o-crisp-edges"),
                (1, 25, 1, 39),
            )
            .with_edit_info(0, (24, 27, "")),
            reject(
                ".foo { background: -webkit-linear-gradient(bottom, #000, #fff); }",
                ".foo { background: linear-gradient(bottom, #000, #fff); }",
                rejected("-webkit-linear-gradient"),
                (1, 20, 1, 43),
            )
            .with_edit_info(0, (19, 27, "")),
            reject(
                ".foo { max-width: -moz-max-content; }",
                ".foo { max-width: max-content; }",
                rejected("-moz-max-content"),
                (1, 19, 1, 35),
            )
            .with_edit_info(0, (18, 23, "")),
            reject(
                ".foo { speak: -xv-digits; }",
                ".foo { speak: digits; }",
                rejected("-xv-digits"),
                (1, 15, 1, 25),
            )
            .with_edit_info(0, (14, 18, "")),
            reject(
                ".foo { white-space: -o-pre-wrap; }",
                ".foo { white-space: pre-wrap; }",
                rejected("-o-pre-wrap"),
                (1, 21, 1, 32),
            )
            .with_edit_info(0, (20, 23, "")),
            reject(
                ".foo { -webkit-user-select: -moz-all; }",
                ".foo { -webkit-user-select: all; }",
                rejected("-moz-all"),
                (1, 29, 1, 37),
            )
            .with_edit_info(0, (28, 33, "")),
            reject(
                "a { display: inline /* comment */ -webkit-flex; }",
                "a { display: inline /* comment */ flex; }",
                rejected("-webkit-flex"),
                (1, 35, 1, 47),
            )
            .with_edit_info(0, (34, 42, "")),
        ];
        Tester::new(NAME, json!([true]), pass, fail)
            .with_fix(true)
            .with_compute_edit_info(true)
            .test();
    }

    #[test]
    fn scss_syntax() {
        let pass = [
            "a { $foo: -webkit-plaintext; }",
            "a { margin: $variable-webkit-variable; }",
            "a { margin: #{$variable-webkit-variable}; }",
            "a { #{$foo}: -webkit-plaintext; }",
        ];
        Tester::new(NAME, json!([true]), pass, [])
            .with_custom_syntax("postcss-scss")
            .with_fix(true)
            .test();
    }

    #[test]
    fn less_syntax() {
        let pass = [
            "@new-color: -webkit-function(@color, 10%);",
            "a { margin: @variable-webkit-variable; }",
        ];
        Tester::new(NAME, json!([true]), pass, [])
            .with_custom_syntax("postcss-less")
            .with_fix(true)
            .test();
    }

    #[test]
    fn ignore_values_regex_single() {
        let pass = ["a { list-style-type: -moz-hangul; }"];
        Tester::new(
            NAME,
            json!([true, { "ignoreValues": "/^-moz-hangul$/" }]),
            pass,
            [],
        )
        .test();
    }

    #[test]
    fn ignore_values_mixed_list() {
        let pass = [
            "a { cursor: -webkit-grab; }",
            "a { list-style-type: -moz-hangul; }",
            "a { background: -webkit-linear-gradient(bottom, #000, #fff); }",
            "a { -moz-user-select: -moz-all; }",
        ];
        let fail = [
            reject(
                ".foo { display: -webkit-flex; }",
                ".foo { display: flex; }",
                rejected("-webkit-flex"),
                (1, 17, 1, 29),
            )
            .with_edit_info(0, (16, 24, "")),
            reject(
                ".foo { display: -wEbKiT-fLeX; }",
                ".foo { display: fLeX; }",
                rejected("-wEbKiT-fLeX"),
                (1, 17, 1, 29),
            )
            .with_edit_info(0, (16, 24, "")),
            reject(
                ".foo { display: -WEBKIT-FLEX; }",
                ".foo { display: FLEX; }",
                rejected("-WEBKIT-FLEX"),
                (1, 17, 1, 29),
            )
            .with_edit_info(0, (16, 24, "")),
            reject(
                ".foo { list-style-type: -moz-hangul-consonant; }",
                ".foo { list-style-type: hangul-consonant; }",
                rejected("-moz-hangul-consonant"),
                (1, 25, 1, 46),
            )
            .with_edit_info(0, (24, 29, "")),
        ];
        Tester::new(
            NAME,
            json!([
                true,
                { "ignoreValues": ["-webkit-grab", "-moz-hangul", "/^-webkit-linear-/", "/^-moz-all$/"] },
            ]),
            pass,
            fail,
        )
        .with_fix(true)
        .with_compute_edit_info(true)
        .test();
    }
}
