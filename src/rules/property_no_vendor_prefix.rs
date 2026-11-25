use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        AUTOPREFIXABLE_PROPERTIES, BASIC_KEYWORDS, OptionMatcher, decl_value_raw,
        in_set_ignore_case, options_matches, parse_option_matchers,
    },
};
use oxc_css_parser::ast::{Declaration, InterpolableIdent};
use serde_json::{Map, Value};

crate::define_stx_lint! {
    /// Disallow vendor prefixes on properties known to Autoprefixer.
    ///
    /// Reference: <https://stylelint.io/user-guide/rules/property-no-vendor-prefix>
    ///
    /// Enable with `true`. `ignoreProperties` matches the full prefixed name
    /// using exact strings or slash-delimited regexes. Custom properties are
    /// skipped. `-webkit-background-size` is reported only when each
    /// comma-separated part has two values or a single `auto` or CSS-wide keyword.
    /// Autofix removes the prefix from reported properties.
    ///
    /// ```css
    /// a { -o-columns: 2; } /* Fixes to columns */
    /// ```
    "property-no-vendor-prefix", restriction, fix = fix;
    pub struct PropertyNoVendorPrefix {
        ignore_properties: Vec<OptionMatcher>,
    }
}

impl PropertyNoVendorPrefix {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_properties: parse_option_matchers(secondary, "ignoreProperties"),
        }
    }
}

/// Upstream `vendor.unprefixed`: strips a leading `-\w+-`.
fn unprefixed(prop: &str) -> Option<&str> {
    let rest = prop.strip_prefix('-')?;
    let word = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .count();
    (word > 0).then_some(())?;
    rest[word..].strip_prefix('-')
}

/// See whatwg/compat#28: only some `-webkit-background-size` values are safe to unprefix.
fn is_safe_webkit_background_size(value: &str) -> bool {
    value.split(',').all(|background| {
        let values: Vec<&str> = background.split_whitespace().collect();
        match values.as_slice() {
            [_, _] => true,
            [first] => *first == "auto" || BASIC_KEYWORDS.contains(first),
            _ => false,
        }
    })
}

impl Rule for PropertyNoVendorPrefix {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_declaration<'a>(&self, decl: &'a Declaration<'a>, ctx: &mut LintContext<'a>) {
        let InterpolableIdent::Literal(ident) = &decl.name else {
            return;
        };
        let prop: &str = ident.raw;
        if options_matches(&self.ignore_properties, prop) || prop.starts_with("--") {
            return;
        }
        let Some(base) = unprefixed(prop) else {
            return;
        };
        if !in_set_ignore_case(AUTOPREFIXABLE_PROPERTIES, base) {
            return;
        }
        if prop == "-webkit-background-size"
            && !decl_value_raw(ctx.source, decl)
                .is_some_and(|(value, _)| is_safe_webkit_background_size(value))
        {
            return;
        }
        ctx.report_with_fix(
            self.name(),
            ident.span,
            format!("Vendor-prefixed property \"{prop}\""),
            Some(Fix::single(ident.span, base.to_string())),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::tester::{Tester, reject_message};
    use serde_json::json;

    #[test]
    fn matches_autoprefixer_properties() {
        let pass = [
            "a { -webkit-background-size: 1px; }",
            "a { -webkit-background-size: 1px   2px ,   1px; }",
        ];
        let fail = [
            reject_message(
                "a { -o-columns: 2; }",
                "a { columns: 2; }",
                "Vendor-prefixed property \"-o-columns\"",
            ),
            reject_message(
                "a { -webkit-background-size: 1px 2px; }",
                "a { background-size: 1px 2px; }",
                "Vendor-prefixed property \"-webkit-background-size\"",
            ),
        ];
        Tester::new("property-no-vendor-prefix", json!(true), pass, fail)
            .with_fix(true)
            .test();
        Tester::new(
            "property-no-vendor-prefix",
            json!([true, {"ignoreProperties": ["-moz-columns", "/^-webkit-animation-/i"]}]),
            [
                "a { -moz-columns: 2; }",
                "a { -webkit-ANIMATION-DeLaY: 0.5s; }",
            ],
            [],
        )
        .test();
    }
}
