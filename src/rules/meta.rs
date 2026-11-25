/// High-level category of a lint rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCategory {
    /// Rules that catch invalid, unknown, or broken CSS syntax (`*-no-unknown`, `*-no-invalid`).
    Correctness,
    /// Rules that catch likely mistakes, overrides, or duplicates (`no-duplicate-*`).
    Suspicious,
    /// Rules that enforce stylistic conventions and notations (`*-notation`, `*-pattern`, `*-case`).
    Style,
    /// Rules that restrict specific CSS features (`*-no-vendor-prefix`, `*-allowed-list`, `*-disallowed-list`).
    Restriction,
}

/// Auto-fixing capability of a lint rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFixMeta {
    /// Rule does not provide auto-fixes.
    None,
    /// Auto-fix is planned but not yet implemented.
    FixPending,
    /// Rule provides safe auto-fixes.
    Fixable,
    /// Rule provides auto-fixes under certain conditions or options.
    Conditional,
}

/// Static metadata and documentation associated with a lint rule.
pub trait RuleMeta {
    /// Kebab-case key used in a configuration's `rules` object.
    const NAME: &'static str;
    /// Kind of problem the rule reports.
    const CATEGORY: RuleCategory;
    /// Whether the rule can emit edits for the autofix engine.
    const FIX: RuleFixMeta = RuleFixMeta::None;
    /// Whether the rule is enabled by the recommended preset.
    const IS_RECOMMENDED: bool = false;
    /// Optional one-line summary, separate from the full reference.
    const SHORT_DESCRIPTION: &'static str = "";
    /// Returns the rule's reference in Markdown when documentation is provided.
    fn documentation() -> Option<&'static str> {
        None
    }
}

/// Defines a rule type and its [`rules::RuleMeta`](crate::rules::RuleMeta) implementation.
///
/// The leading doc comments are published on the type and returned by
/// [`RuleMeta::documentation`]. The generated `NAME` can be returned from
/// [`Rule::name`]; the macro does not implement the rule's lint callbacks.
///
/// # Examples
///
/// ```
/// use stx::{define_stx_lint, rules::{RuleCategory, RuleFixMeta, RuleMeta}};
///
/// define_stx_lint! {
///     /// Flag empty widget blocks.
///     "widget-no-empty", correctness, fix = conditional, recommended = true;
///     pub struct WidgetNoEmpty;
/// }
///
/// assert_eq!(WidgetNoEmpty::NAME, "widget-no-empty");
/// assert_eq!(WidgetNoEmpty::CATEGORY, RuleCategory::Correctness);
/// assert_eq!(WidgetNoEmpty::FIX, RuleFixMeta::Conditional);
/// assert!(WidgetNoEmpty::documentation().unwrap().contains("empty widget blocks"));
/// ```
#[macro_export]
macro_rules! define_stx_lint {
    (
        $(#[doc = $doc:literal])+
        $rule_name:literal, $category:ident
        $(, fix = $fix:ident)?
        $(, recommended = $recommended:literal)?
        $(, short_description = $short_desc:literal)?
        $(,)?;
        $(#[$attr:meta])* $vis:vis struct $struct_name:ident $($body:tt)*
    ) => {
        $(#[doc = $doc])+
        #[doc(alias = $rule_name)]
        $(#[$attr])*
        $vis struct $struct_name $($body)*
        $crate::declare_stx_lint! {
            $(#[doc = $doc])+
            $struct_name, $rule_name, $category
            $(, fix = $fix)?
            $(, recommended = $recommended)?
            $(, short_description = $short_desc)?
        }
    };
}

/// Implements rule metadata for an existing type. Use [`define_stx_lint!`] for new rule types.
#[macro_export]
macro_rules! declare_stx_lint {
    (
        $(#[doc = $doc:literal])*
        $struct_name:ident,
        $rule_name:literal,
        $category:ident
        $(, fix = $fix:ident)?
        $(, recommended = $recommended:literal)?
        $(, short_description = $short_desc:literal)?
        $(,)?
    ) => {
        impl $crate::rules::RuleMeta for $struct_name {
            const NAME: &'static str = $rule_name;
            const CATEGORY: $crate::rules::RuleCategory = $crate::declare_stx_lint!(@category $category);
            $(const FIX: $crate::rules::RuleFixMeta = $crate::declare_stx_lint!(@fix $fix);)?
            $(const IS_RECOMMENDED: bool = $recommended;)?
            $(const SHORT_DESCRIPTION: &'static str = $short_desc;)?
            fn documentation() -> Option<&'static str> {
                Some(concat!($($doc, "\n"),*))
            }
        }
        impl $struct_name {
            pub const NAME: &'static str = $rule_name;
        }
    };
    (@category correctness) => { $crate::rules::RuleCategory::Correctness };
    (@category suspicious) => { $crate::rules::RuleCategory::Suspicious };
    (@category style) => { $crate::rules::RuleCategory::Style };
    (@category restriction) => { $crate::rules::RuleCategory::Restriction };
    (@fix none) => { $crate::rules::RuleFixMeta::None };
    (@fix pending) => { $crate::rules::RuleFixMeta::FixPending };
    (@fix fix) => { $crate::rules::RuleFixMeta::Fixable };
    (@fix conditional) => { $crate::rules::RuleFixMeta::Conditional };
}
