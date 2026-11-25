use crate::{
    context::LintContext,
    fix::Fix,
    rules::Rule,
    utils::{SelectorItemKind, scan_selector_items},
};
use oxc_css_parser::ast::SelectorList;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColonNotationMode {
    Single,
    Double,
}

crate::define_stx_lint! {
    /// Enforces single- or double-colon notation for pseudo-elements.
    ///
    /// Set to `"double"` or `"single"`. Single-colon mode applies only to `before`,
    /// `after`, `first-line`, and `first-letter`; newer pseudo-elements keep `::`.
    /// Auto-fix changes the colon count without changing the name.
    ///
    /// Example: `"double"` changes `a:before {}` to `a::before {}`.
    "selector-pseudo-element-colon-notation", style, fix = fix;
    pub struct SelectorPseudoElementColonNotation {
        pub mode: ColonNotationMode,
    }
}

impl Default for SelectorPseudoElementColonNotation {
    fn default() -> Self {
        Self {
            mode: ColonNotationMode::Double,
        }
    }
}

const LEGACY_PSEUDO_ELEMENTS: &[&str] = &["before", "after", "first-letter", "first-line"];

impl Rule for SelectorPseudoElementColonNotation {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_selector_list<'a>(&self, list: &'a SelectorList<'a>, ctx: &mut LintContext<'a>) {
        let items = scan_selector_items(ctx.source, list.span);
        for item in items {
            let SelectorItemKind::PseudoElement {
                colon_span,
                single_colon,
            } = item.kind
            else {
                continue;
            };

            match self.mode {
                ColonNotationMode::Double => {
                    if single_colon {
                        ctx.report_with_fix(
                            self.name(),
                            colon_span,
                            "Expected double colon pseudo-element notation".to_string(),
                            Some(Fix::single(colon_span, "::".to_string())),
                        );
                    }
                }
                ColonNotationMode::Single => {
                    if !single_colon
                        && LEGACY_PSEUDO_ELEMENTS
                            .iter()
                            .any(|pe| pe.eq_ignore_ascii_case(item.name))
                    {
                        ctx.report_with_fix(
                            self.name(),
                            colon_span,
                            "Expected single colon pseudo-element notation".to_string(),
                            Some(Fix::single(colon_span, ":".to_string())),
                        );
                    }
                }
            }
        }
    }
}
