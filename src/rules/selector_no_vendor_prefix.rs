use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{
        OptionMatcher, SelectorItemKind, is_autoprefixable_selector, options_matches,
        parse_option_matchers, scan_selector_items,
    },
};
use oxc_css_parser::{Span, ast::SelectorList};
use serde_json::{Map, Value};

crate::define_stx_lint! {
    /// Disallows vendor prefixes on pseudo-selectors known to Autoprefixer.
    ///
    /// Set to `true`. `ignoreSelectors` matches the selector text, including its colons,
    /// by string or `/regex/` pattern. Auto-fix strips the prefix from reported names;
    /// other vendor-specific pseudos are left alone.
    ///
    /// Example: `::-moz-placeholder {}` is fixed to `::placeholder {}`.
    "selector-no-vendor-prefix", restriction, fix = fix;
    pub struct SelectorNoVendorPrefix {
        ignore_selectors: Vec<OptionMatcher>,
    }
}

impl SelectorNoVendorPrefix {
    pub fn from_secondary(secondary: Option<&Map<String, Value>>) -> Self {
        Self {
            ignore_selectors: parse_option_matchers(secondary, "ignoreSelectors"),
        }
    }
}

fn strip_prefix(s: &str) -> Option<&str> {
    let lower = s.to_ascii_lowercase();
    for p in ["-webkit-", "-moz-", "-ms-", "-o-"] {
        if lower.starts_with(p) {
            return Some(&s[p.len()..]);
        }
    }
    None
}

impl Rule for SelectorNoVendorPrefix {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        let items = scan_selector_items(ctx.source, list.span);
        for item in items {
            let colon_end = match item.kind {
                SelectorItemKind::PseudoClass { colon_span } => colon_span.end,
                SelectorItemKind::PseudoElement { colon_span, .. } => colon_span.end,
                _ => continue,
            };

            let raw = ctx.slice(item.span);
            if is_autoprefixable_selector(raw)
                && !options_matches(&self.ignore_selectors, raw)
                && let Some(unprefixed) = strip_prefix(item.name)
            {
                let name_span = Span {
                    start: colon_end,
                    end: item.span.end,
                };
                ctx.report_with_fix(
                    self.name(),
                    item.span,
                    format!("Vendor-prefixed selector \"{}\"", raw),
                    Some(Fix::single(name_span, unprefixed.to_string())),
                );
            }
        }
    }
}
