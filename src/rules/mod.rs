//! Rule reference and runtime registration.
//!
//! Each rule type documents its settings and examples. Search for the rule's
//! kebab-case configuration key to go directly to its type. A rule is only active
//! when selected by the configuration or a preset.

pub mod alpha_value_notation;
pub mod annotation_no_unknown;
pub mod at_charset_rule_no_invalid;
pub mod at_rule_allowed_list;
pub mod at_rule_descriptor_no_unknown;
pub mod at_rule_descriptor_value_no_unknown;
pub mod at_rule_disallowed_list;
pub mod at_rule_empty_line_before;
pub mod at_rule_no_deprecated;
pub mod at_rule_no_unknown;
pub mod at_rule_no_vendor_prefix;
pub mod at_rule_prelude_no_invalid;
pub mod at_rule_property_required_list;
pub mod block_no_empty;
pub mod block_no_redundant_nested_style_rules;
pub mod color_function_alias_notation;
pub mod color_function_notation;
pub mod color_hex_alpha;
pub mod color_hex_length;
pub mod color_named;
pub mod color_no_hex;
pub mod color_no_invalid_hex;
pub mod comment_empty_line_before;
pub mod comment_no_empty;
pub mod comment_pattern;
pub mod comment_whitespace_inside;
pub mod comment_word_disallowed_list;
pub mod container_name_pattern;
pub mod custom_media_pattern;
pub mod custom_property_empty_line_before;
pub mod custom_property_no_missing_var_function;
pub mod custom_property_pattern;
pub mod declaration_block_no_duplicate_custom_properties;
pub mod declaration_block_no_duplicate_properties;
pub mod declaration_block_no_redundant_longhand_properties;
pub mod declaration_block_no_shorthand_property_overrides;
pub mod declaration_block_single_line_max_declarations;
pub mod declaration_empty_line_before;
pub mod declaration_no_important;
pub mod declaration_property_custom_property_allowed_list;
pub mod declaration_property_max_values;
pub mod declaration_property_unit_allowed_list;
pub mod declaration_property_unit_disallowed_list;
pub mod declaration_property_value_allowed_list;
pub mod declaration_property_value_disallowed_list;
pub mod declaration_property_value_keyword_no_deprecated;
pub mod display_notation;
pub mod font_family_name_quotes;
pub mod font_family_no_duplicate_names;
pub mod font_family_no_missing_generic_family_keyword;
pub mod font_weight_notation;
pub mod function_allowed_list;
pub mod function_calc_no_unspaced_operator;
pub mod function_disallowed_list;
pub mod function_linear_gradient_no_nonstandard_direction;
pub mod function_name_case;
pub mod function_no_unknown;
pub mod function_url_no_scheme_relative;
pub mod function_url_quotes;
pub mod function_url_scheme_allowed_list;
pub mod function_url_scheme_disallowed_list;
pub mod hue_degree_notation;
pub mod import_notation;
pub mod keyframe_block_no_duplicate_selectors;
pub mod keyframe_declaration_no_important;
pub mod keyframe_selector_notation;
pub mod keyframes_name_pattern;
pub mod layer_name_pattern;
pub mod length_zero_no_unit;
pub mod lightness_notation;
pub mod media_feature_name_allowed_list;
pub mod media_feature_name_disallowed_list;
pub mod media_feature_name_no_unknown;
pub mod media_feature_name_no_vendor_prefix;
pub mod media_feature_name_unit_allowed_list;
pub mod media_feature_name_value_allowed_list;
pub mod media_feature_range_notation;
pub mod media_query_no_invalid;
pub mod media_type_no_deprecated;
pub mod named_grid_areas_no_invalid;
pub mod no_descending_specificity;
pub mod no_duplicate_at_import_rules;
pub mod no_duplicate_selectors;
pub mod no_empty_source;
pub mod no_invalid_double_slash_comments;
pub mod no_invalid_position_at_import_rule;
pub mod no_invalid_position_declaration;
pub mod no_irregular_whitespace;
pub mod no_unknown_animations;
pub mod no_unknown_custom_media;
pub mod no_unknown_custom_properties;
pub mod number_max_precision;
pub mod property_allowed_list;
pub mod property_disallowed_list;
pub mod property_no_deprecated;
pub mod property_no_unknown;
pub mod property_no_vendor_prefix;
pub mod rule_empty_line_before;
pub mod rule_nesting_at_rule_required_list;
pub mod rule_selector_property_disallowed_list;
pub mod selector_anb_no_unmatchable;
pub mod selector_attribute_name_disallowed_list;
pub mod selector_attribute_operator_allowed_list;
pub mod selector_attribute_operator_disallowed_list;
pub mod selector_attribute_quotes;
pub mod selector_class_pattern;
pub mod selector_combinator_allowed_list;
pub mod selector_combinator_disallowed_list;
pub mod selector_disallowed_list;
pub mod selector_id_pattern;
pub mod selector_max_attribute;
pub mod selector_max_class;
pub mod selector_max_combinators;
pub mod selector_max_compound_selectors;
pub mod selector_max_id;
pub mod selector_max_pseudo_class;
pub mod selector_max_specificity;
pub mod selector_max_type;
pub mod selector_max_universal;
pub mod selector_nested_pattern;
pub mod selector_no_vendor_prefix;
pub mod selector_not_notation;
pub mod selector_pseudo_class_allowed_list;
pub mod selector_pseudo_class_disallowed_list;
pub mod selector_pseudo_class_no_unknown;
pub mod selector_pseudo_element_allowed_list;
pub mod selector_pseudo_element_colon_notation;
pub mod selector_pseudo_element_disallowed_list;
pub mod selector_pseudo_element_no_unknown;
pub mod selector_type_case;
pub mod selector_type_no_unknown;
pub mod shorthand_property_no_redundant_values;
pub mod string_no_newline;
pub mod syntax_string_no_invalid;
pub mod time_min_milliseconds;
pub mod unit_allowed_list;
pub mod unit_disallowed_list;
pub mod unit_layout_mappings;
pub mod unit_no_unknown;
pub mod value_keyword_case;
pub mod value_keyword_layout_mappings;
pub mod value_no_vendor_prefix;

pub mod declaration_property_value_no_unknown;
pub mod max_nesting_depth;
pub mod media_feature_name_value_no_unknown;
pub mod nesting_selector_no_missing_scoping_root;
pub mod property_layout_mappings;
pub mod relative_selector_nesting_notation;
pub mod selector_no_deprecated;
pub mod selector_no_invalid;
pub mod selector_no_qualifying_type;
pub mod selector_no_unmatchable;
pub mod value_no_invalid;
pub mod value_type_custom_property_required_list;

use crate::{config::Config, context::LintContext, diagnostic::Severity, utils::PatternMatcher};
use oxc_css_parser::{
    ast::{
        AtRule, ComplexSelector, ComponentValue, CompoundSelector, Declaration, KeyframeBlock,
        QualifiedRule, SassVariableDeclaration, SelectorList, SimpleBlock, SimpleSelector,
        Stylesheet, UnknownQualifiedRule,
    },
    token::Comment,
};
use std::sync::Arc;

pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_comment<'a>(&self, _comment: &'a Comment<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_qualified_rule<'a>(&self, _rule: &'a QualifiedRule<'a>, _ctx: &mut LintContext<'a>) {}
    /// A `$variable: value` declaration, which postcss-scss parses as a declaration.
    fn check_sass_variable_declaration<'a>(
        &self,
        _decl: &'a SassVariableDeclaration<'a>,
        _ctx: &mut LintContext<'a>,
    ) {
    }
    /// A style rule whose selector the parser could not read.
    fn check_unknown_qualified_rule<'a>(
        &self,
        _rule: &'a UnknownQualifiedRule<'a>,
        _ctx: &mut LintContext<'a>,
    ) {
    }
    fn check_at_rule<'a>(&self, _at_rule: &'a AtRule<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_keyframe_block<'a>(&self, _block: &'a KeyframeBlock<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_simple_block<'a>(&self, _block: &'a SimpleBlock<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_declaration<'a>(&self, _decl: &'a Declaration<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_selector_list<'a>(&self, _list: &'a SelectorList<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_complex_selector<'a>(
        &self,
        _sel: &'a ComplexSelector<'a>,
        _ctx: &mut LintContext<'a>,
    ) {
    }
    fn check_compound_selector<'a>(
        &self,
        _sel: &'a CompoundSelector<'a>,
        _ctx: &mut LintContext<'a>,
    ) {
    }
    fn check_simple_selector<'a>(&self, _sel: &'a SimpleSelector<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_component_value<'a>(&self, _val: &'a ComponentValue<'a>, _ctx: &mut LintContext<'a>) {}
}

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

pub struct ConfiguredRule {
    pub rule: Box<dyn Rule>,
    pub severity: Severity,
    pub custom_message: Option<Arc<str>>,
}

pub fn build_rules(config: &Config) -> Vec<ConfiguredRule> {
    let mut active = Vec::new();
    for (name, maybe_setting) in &config.rules {
        let Some(setting) = maybe_setting else {
            continue;
        };
        let rule: Option<Box<dyn Rule>> = match name.as_str() {
            "alpha-value-notation" => Some(Box::new(
                alpha_value_notation::AlphaValueNotation::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                ),
            )),
            "at-rule-empty-line-before" => {
                at_rule_empty_line_before::AtRuleEmptyLineBefore::from_setting(setting)
                    .map(|r| Box::new(r) as Box<dyn Rule>)
            },
            "at-rule-no-deprecated" => Some(Box::new(
                          at_rule_no_deprecated::AtRuleNoDeprecated::new(setting.secondary.as_ref()),
                      )),
            "at-rule-allowed-list" => Some(Box::new(
                at_rule_allowed_list::AtRuleAllowedList::from_primary(&setting.primary),
            )),
            "block-no-redundant-nested-style-rules" => Some(Box::new(
                          block_no_redundant_nested_style_rules::BlockNoRedundantNestedStyleRules,
                      )),
                      "color-hex-alpha" => {
                          color_hex_alpha::ColorHexAlpha::from_primary(&setting.primary)
                              .map(|r| Box::new(r) as Box<dyn Rule>)
                      },
            "at-rule-descriptor-value-no-unknown" => {
                at_rule_descriptor_value_no_unknown::AtRuleDescriptorValueNoUnknown::from_setting(
                    setting,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "at-rule-disallowed-list" => {
                at_rule_disallowed_list::AtRuleDisallowedList::from_setting(setting)
                    .map(|r| Box::new(r) as Box<dyn Rule>)
            },
            "comment-word-disallowed-list" => {
                           comment_word_disallowed_list::CommentWordDisallowedList::from_config(
                               &setting.primary,
                               setting.severity,
                           )
                           .map(|r| Box::new(r) as Box<dyn Rule>)
                       }
            "at-rule-property-required-list" => Some(Box::new(
                at_rule_property_required_list::AtRulePropertyRequiredList::new(
                    &setting.primary,
                    setting.secondary.as_ref(),
                ),
            )),
            "at-rule-descriptor-no-unknown" => {
                at_rule_descriptor_no_unknown::AtRuleDescriptorNoUnknown::from_setting(setting)
                    .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "at-charset-rule-no-invalid" => Some(Box::new(
                at_charset_rule_no_invalid::AtCharsetRuleNoInvalid,
            )),
            "annotation-no-unknown" => Some(Box::new(
                annotation_no_unknown::AnnotationNoUnknown::from_secondary(
                    setting.secondary.as_ref(),
                ),
            )),
            "at-rule-prelude-no-invalid" => Some(Box::new(
                at_rule_prelude_no_invalid::AtRulePreludeNoInvalid::new(
                    setting.secondary.as_ref(),
                ),
            )),
            "at-rule-no-unknown" => Some(Box::new(at_rule_no_unknown::AtRuleNoUnknown::new(
                setting.secondary.as_ref(),
            ))),
            "at-rule-no-vendor-prefix" => Some(Box::new(
                at_rule_no_vendor_prefix::AtRuleNoVendorPrefix::new(setting.secondary.as_ref()),
            )),
            "block-no-empty" => Some(Box::new(block_no_empty::BlockNoEmpty::from_secondary(
                setting.secondary.as_ref(),
            ))),
            "comment-pattern" => {
                          comment_pattern::CommentPattern::from_config(&setting.primary, setting.severity)
                              .map(|r| Box::new(r) as Box<dyn Rule>)
                      },
            "color-function-alias-notation" => {
                color_function_alias_notation::ColorFunctionAliasNotation::from_primary(
                    &setting.primary,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "color-function-notation" => {
                color_function_notation::ColorFunctionNotation::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            },
            "color-no-hex" => color_no_hex::ColorNoHex::from_config(
                           &setting.primary,
                           setting.secondary.as_ref(),
                           setting.severity,
                       )
                       .map(|r| Box::new(r) as Box<dyn Rule>),
            "color-hex-length" => {
                let mode = if setting.primary.as_str() == Some("long") {
                    color_hex_length::HexLengthMode::Long
                } else {
                    color_hex_length::HexLengthMode::Short
                };
                Some(Box::new(color_hex_length::ColorHexLength { mode }))
            }
            "color-no-invalid-hex" => {
                        color_no_invalid_hex::ColorNoInvalidHex::from_config(&setting.primary, setting.severity)
                            .map(|r| Box::new(r) as Box<dyn Rule>)
                    }
            "comment-empty-line-before" => Some(Box::new(
                comment_empty_line_before::CommentEmptyLineBefore::from_setting(setting),
            )),
            "comment-no-empty" => {
                           comment_no_empty::CommentNoEmpty::from_config(&setting.primary, setting.severity)
                               .map(|r| Box::new(r) as Box<dyn Rule>)
                       }
                       "comment-whitespace-inside" => {
                                     comment_whitespace_inside::CommentWhitespaceInside::from_config(
                                         &setting.primary,
                                         setting.severity,
                                     )
                                     .map(|r| Box::new(r) as Box<dyn Rule>)
                                 }
                                 "container-name-pattern" => {
                                     container_name_pattern::ContainerNamePattern::from_config(
                                         &setting.primary,
                                         setting.secondary.as_ref(),
                                     )
                                     .map(|r| Box::new(r) as Box<dyn Rule>)
                                 }
            "media-query-no-invalid" => Some(Box::new(
                media_query_no_invalid::MediaQueryNoInvalid::from_secondary(
                    setting.secondary.as_ref(),
                ),
            )),
            "color-named" => color_named::ColorNamed::from_config(
                            &setting.primary,
                            setting.secondary.as_ref(),
                            setting.severity,
                        )
                        .map(|r| Box::new(r) as Box<dyn Rule>),
            "custom-media-pattern" => {
                           custom_media_pattern::CustomMediaPattern::from_config(
                               &setting.primary,
                               setting.secondary.as_ref(),
                           )
                           .map(|r| Box::new(r) as Box<dyn Rule>)
                       },
                       "custom-property-empty-line-before" => {
                                       custom_property_empty_line_before::CustomPropertyEmptyLineBefore::from_config(
                                           &setting.primary,
                                           setting.secondary.as_ref(),
                                       )
                                       .map(|r| Box::new(r) as Box<dyn Rule>)
                                   }
            "custom-property-no-missing-var-function" => Some(Box::new(
                custom_property_no_missing_var_function::CustomPropertyNoMissingVarFunction,
            )),
            "custom-property-pattern" => {
                            custom_property_pattern::CustomPropertyPattern::from_config(
                                &setting.primary,
                                setting.secondary.as_ref(),
                            )
                            .map(|r| Box::new(r) as Box<dyn Rule>)
                        }
                        "declaration-block-no-duplicate-custom-properties" => {
                                      declaration_block_no_duplicate_custom_properties::DeclarationBlockNoDuplicateCustomProperties::from_config(
                                          &setting.primary,
                                          setting.secondary.as_ref(),
                                      )
                                      .map(|r| Box::new(r) as Box<dyn Rule>)
                                  }
                                  "declaration-block-no-duplicate-properties" => {
                                                 declaration_block_no_duplicate_properties::DeclarationBlockNoDuplicateProperties::from_config(
                                                     &setting.primary,
                                                     setting.secondary.as_ref(),
                                                 )
                                                 .map(|r| Box::new(r) as Box<dyn Rule>)
                                             }
            "declaration-block-no-redundant-longhand-properties" => Some(Box::new(
                declaration_block_no_redundant_longhand_properties::DeclarationBlockNoRedundantLonghandProperties::from_secondary(
                    setting.secondary.as_ref(),
                ),
            )),
            "declaration-block-no-shorthand-property-overrides" => Some(Box::new(
                declaration_block_no_shorthand_property_overrides::DeclarationBlockNoShorthandPropertyOverrides,
            )),
            "declaration-block-single-line-max-declarations" => {
                let max = setting.primary.as_u64().unwrap_or(1) as usize;
                Some(Box::new(
                    declaration_block_single_line_max_declarations::DeclarationBlockSingleLineMaxDeclarations { max },
                ))
            }
            "declaration-empty-line-before" => Some(Box::new(
                declaration_empty_line_before::DeclarationEmptyLineBefore::from_setting(setting),
            )),
            "declaration-no-important" => {
                Some(Box::new(declaration_no_important::DeclarationNoImportant))
            }
            "declaration-property-custom-property-allowed-list" => {
                declaration_property_custom_property_allowed_list::DeclarationPropertyCustomPropertyAllowedList::from_config(
                    &setting.primary,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "declaration-property-max-values" => {
                declaration_property_max_values::DeclarationPropertyMaxValues::from_config(
                    &setting.primary,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "declaration-property-unit-allowed-list" => {
                declaration_property_unit_allowed_list::DeclarationPropertyUnitAllowedList::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "declaration-property-unit-disallowed-list" => {
                declaration_property_unit_disallowed_list::DeclarationPropertyUnitDisallowedList::from_config(
                    &setting.primary,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "declaration-property-value-allowed-list" => {
                declaration_property_value_allowed_list::DeclarationPropertyValueAllowedList::from_config(
                    &setting.primary,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "declaration-property-value-disallowed-list" => {
                declaration_property_value_disallowed_list::DeclarationPropertyValueDisallowedList::from_config(
                    &setting.primary,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "declaration-property-value-keyword-no-deprecated" => {
                Some(Box::new(
                    declaration_property_value_keyword_no_deprecated::DeclarationPropertyValueKeywordNoDeprecated::new(
                        setting.secondary.as_ref(),
                    ),
                ))
            }
            "display-notation" => display_notation::DisplayNotation::from_primary(&setting.primary)
                .map(|r| Box::new(r) as Box<dyn Rule>),
            "font-weight-notation" => {
                font_weight_notation::FontWeightNotation::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "function-allowed-list" => {
                Some(Box::new(function_allowed_list::FunctionAllowedList::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                )))
            }
            "function-disallowed-list" => {
                Some(Box::new(function_disallowed_list::FunctionDisallowedList::from_config(
                    &setting.primary,
                )))
            }
            "function-url-no-scheme-relative" => Some(Box::new(
                function_url_no_scheme_relative::FunctionUrlNoSchemeRelative,
            )),
            "function-url-scheme-allowed-list" => {
                Some(Box::new(function_url_scheme_allowed_list::FunctionUrlSchemeAllowedList::from_config(
                    &setting.primary,
                )))
            }
            "function-url-scheme-disallowed-list" => {
                Some(Box::new(function_url_scheme_disallowed_list::FunctionUrlSchemeDisallowedList::from_config(
                    &setting.primary,
                )))
            }
            "font-family-name-quotes" => {
                let mode = match setting.primary.as_str() {
                    Some("always-unless-keyword") => {
                        font_family_name_quotes::FontFamilyQuoteMode::AlwaysUnlessKeyword
                    }
                    Some("always-where-required") => {
                        font_family_name_quotes::FontFamilyQuoteMode::AlwaysWhereRequired
                    }
                    _ => font_family_name_quotes::FontFamilyQuoteMode::AlwaysWhereRecommended,
                };
                Some(Box::new(font_family_name_quotes::FontFamilyNameQuotes {
                    mode,
                }))
            }
            "font-family-no-duplicate-names" => Some(Box::new(
                font_family_no_duplicate_names::FontFamilyNoDuplicateNames::from_secondary(
                    setting.secondary.as_ref(),
                ),
            )),
            "font-family-no-missing-generic-family-keyword" => Some(Box::new(
                font_family_no_missing_generic_family_keyword::FontFamilyNoMissingGenericFamilyKeyword::from_config(
                    setting.secondary.as_ref(),
                ),
            )),
            "function-calc-no-unspaced-operator" => Some(Box::new(
                function_calc_no_unspaced_operator::FunctionCalcNoUnspacedOperator,
            )),
            "function-linear-gradient-no-nonstandard-direction" => Some(Box::new(
                function_linear_gradient_no_nonstandard_direction::FunctionLinearGradientNoNonstandardDirection,
            )),
            "function-name-case" => {
                let mode = if setting.primary.as_str() == Some("upper") {
                    function_name_case::CaseMode::Upper
                } else {
                    function_name_case::CaseMode::Lower
                };
                Some(Box::new(function_name_case::FunctionNameCase::new(
                    mode,
                    setting.secondary.as_ref(),
                )))
            }
            "function-no-unknown" => Some(Box::new(function_no_unknown::FunctionNoUnknown::from_config(
                setting.secondary.as_ref(),
            ))),
            "function-url-quotes" => Some(Box::new(
                function_url_quotes::FunctionUrlQuotes::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                ),
            )),
            "hue-degree-notation" => {
                let mode = if setting.primary.as_str() == Some("number") {
                    hue_degree_notation::HueDegreeMode::Number
                } else {
                    hue_degree_notation::HueDegreeMode::Angle
                };
                Some(Box::new(hue_degree_notation::HueDegreeNotation { mode }))
            }
            "import-notation" => {
                let mode = if setting.primary.as_str() == Some("string") {
                    import_notation::ImportNotationMode::String
                } else {
                    import_notation::ImportNotationMode::Url
                };
                Some(Box::new(import_notation::ImportNotation { mode }))
            }
            "keyframe-block-no-duplicate-selectors" => Some(Box::new(
                keyframe_block_no_duplicate_selectors::KeyframeBlockNoDuplicateSelectors,
            )),
            "keyframe-declaration-no-important" => Some(Box::new(
                keyframe_declaration_no_important::KeyframeDeclarationNoImportant,
            )),
            "keyframe-selector-notation" => {
                let mode = match setting.primary.as_str() {
                    Some("percentage") => {
                        keyframe_selector_notation::KeyframeSelectorMode::Percentage
                    }
                    Some("keyword") => keyframe_selector_notation::KeyframeSelectorMode::Keyword,
                    _ => keyframe_selector_notation::KeyframeSelectorMode::PercentageUnlessWithinKeywordOnlyBlock,
                };
                Some(Box::new(
                    keyframe_selector_notation::KeyframeSelectorNotation { mode },
                ))
            }
            "keyframes-name-pattern" => Some(Box::new(
                keyframes_name_pattern::KeyframesNamePattern {
                    matcher: PatternMatcher::new(setting.primary.as_str()),
                },
            )),
            "layer-name-pattern" => Some(Box::new(layer_name_pattern::LayerNamePattern {
                matcher: PatternMatcher::new(setting.primary.as_str()),
            })),
            "length-zero-no-unit" => Some(Box::new(
                length_zero_no_unit::LengthZeroNoUnit::from_config(setting.secondary.as_ref()),
            )),
            "lightness-notation" => {
                let mode = if setting.primary.as_str() == Some("number") {
                    lightness_notation::LightnessMode::Number
                } else {
                    lightness_notation::LightnessMode::Percentage
                };
                Some(Box::new(lightness_notation::LightnessNotation { mode }))
            }
            "media-feature-name-unit-allowed-list" => {
                media_feature_name_unit_allowed_list::MediaFeatureNameUnitAllowedList::from_config(
                    &setting.primary,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "media-feature-name-value-allowed-list" => {
                media_feature_name_value_allowed_list::MediaFeatureNameValueAllowedList::from_config(
                    &setting.primary,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "media-feature-name-no-unknown" => Some(Box::new(
                media_feature_name_no_unknown::MediaFeatureNameNoUnknown::new(
                    setting.secondary.as_ref(),
                ),
            )),
            "media-feature-name-allowed-list" => Some(Box::new(
                media_feature_name_allowed_list::MediaFeatureNameAllowedList::from_config(
                    &setting.primary,
                ),
            )),
            "media-feature-name-disallowed-list" => Some(Box::new(
                media_feature_name_disallowed_list::MediaFeatureNameDisallowedList::from_config(
                    &setting.primary,
                ),
            )),
            "media-type-no-deprecated" => Some(Box::new(
                media_type_no_deprecated::MediaTypeNoDeprecated::new(setting.secondary.as_ref()),
            )),
            "media-feature-name-no-vendor-prefix" => Some(Box::new(
                media_feature_name_no_vendor_prefix::MediaFeatureNameNoVendorPrefix::from_secondary(
                    setting.secondary.as_ref(),
                ),
            )),
            "media-feature-range-notation" => {
                let mode = if setting.primary.as_str() == Some("prefix") {
                    media_feature_range_notation::MediaRangeMode::Prefix
                } else {
                    media_feature_range_notation::MediaRangeMode::Context
                };
                Some(Box::new(
                    media_feature_range_notation::MediaFeatureRangeNotation {
                        mode,
                        except_exact_value: crate::utils::secondary_str_list(
                            setting.secondary.as_ref(),
                            "except",
                        )
                        .iter()
                        .any(|s| s == "exact-value"),
                    },
                ))
            }
            "named-grid-areas-no-invalid" => Some(Box::new(
                named_grid_areas_no_invalid::NamedGridAreasNoInvalid,
            )),
            "no-descending-specificity" => Some(Box::new(
                no_descending_specificity::NoDescendingSpecificity::from_config(
                    setting.secondary.as_ref(),
                ),
            )),
            "no-duplicate-at-import-rules" => Some(Box::new(
                no_duplicate_at_import_rules::NoDuplicateAtImportRules,
            )),
            "no-duplicate-selectors" => Some(Box::new(
                no_duplicate_selectors::NoDuplicateSelectors::from_config(
                    setting.secondary.as_ref(),
                ),
            )),
            "no-unknown-custom-properties" => Some(Box::new(
                no_unknown_custom_properties::NoUnknownCustomProperties,
            )),
            "no-empty-source" => Some(Box::new(no_empty_source::NoEmptySource)),
            "no-invalid-double-slash-comments" => Some(Box::new(
                no_invalid_double_slash_comments::NoInvalidDoubleSlashComments,
            )),
            "no-invalid-position-at-import-rule" => Some(Box::new(
                no_invalid_position_at_import_rule::NoInvalidPositionAtImportRule::from_config(
                    setting.secondary.as_ref(),
                ),
            )),
            "no-invalid-position-declaration" => Some(Box::new(
                no_invalid_position_declaration::NoInvalidPositionDeclaration::from_config(
                    setting.secondary.as_ref(),
                ),
            )),
            "no-irregular-whitespace" => {
                Some(Box::new(no_irregular_whitespace::NoIrregularWhitespace))
            }
            "number-max-precision" => Some(Box::new(
                number_max_precision::NumberMaxPrecision::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                ),
            )),
            "property-allowed-list" => Some(Box::new(
                property_allowed_list::PropertyAllowedList::from_config(&setting.primary),
            )),
            "property-disallowed-list" => Some(Box::new(
                property_disallowed_list::PropertyDisallowedList::from_config(&setting.primary),
            )),
            "property-no-deprecated" => Some(Box::new(
                property_no_deprecated::PropertyNoDeprecated::new(setting.secondary.as_ref()),
            )),
            "property-no-unknown" => Some(Box::new(
                property_no_unknown::PropertyNoUnknown::from_config(setting.secondary.as_ref()),
            )),
            "property-no-vendor-prefix" => {
                Some(Box::new(property_no_vendor_prefix::PropertyNoVendorPrefix::from_secondary(
                    setting.secondary.as_ref(),
                )))
            }
            "no-unknown-animations" => Some(Box::new(
                no_unknown_animations::NoUnknownAnimations,
            )),
            "rule-nesting-at-rule-required-list" => Some(Box::new(
                rule_nesting_at_rule_required_list::RuleNestingAtRuleRequiredList::from_config(
                    &setting.primary,
                ),
            )),
            "rule-empty-line-before" => Some(Box::new(
                rule_empty_line_before::RuleEmptyLineBefore::from_setting(setting),
            )),
            "selector-anb-no-unmatchable" => Some(Box::new(
                selector_anb_no_unmatchable::SelectorAnbNoUnmatchable,
            )),
            "selector-attribute-name-disallowed-list" => Some(Box::new(
                selector_attribute_name_disallowed_list::SelectorAttributeNameDisallowedList::from_config(
                    &setting.primary,
                ),
            )),
            "selector-attribute-operator-allowed-list" => Some(Box::new(
                selector_attribute_operator_allowed_list::SelectorAttributeOperatorAllowedList::from_config(
                    &setting.primary,
                ),
            )),
            "selector-attribute-operator-disallowed-list" => Some(Box::new(
                selector_attribute_operator_disallowed_list::SelectorAttributeOperatorDisallowedList::from_config(
                    &setting.primary,
                ),
            )),
            "selector-combinator-allowed-list" => Some(Box::new(
                selector_combinator_allowed_list::SelectorCombinatorAllowedList::from_config(
                    &setting.primary,
                ),
            )),
            "selector-combinator-disallowed-list" => Some(Box::new(
                selector_combinator_disallowed_list::SelectorCombinatorDisallowedList::from_config(
                    &setting.primary,
                ),
            )),
            "selector-attribute-quotes" => {
                let mode = if setting.primary.as_str() == Some("never") {
                    function_url_quotes::UrlQuoteMode::Never
                } else {
                    function_url_quotes::UrlQuoteMode::Always
                };
                Some(Box::new(
                    selector_attribute_quotes::SelectorAttributeQuotes { mode },
                ))
            }
            "selector-class-pattern" => Some(Box::new(
                selector_class_pattern::SelectorClassPattern {
                    matcher: PatternMatcher::new(setting.primary.as_str()),
                },
            )),
            "selector-id-pattern" => Some(Box::new(selector_id_pattern::SelectorIdPattern {
                matcher: PatternMatcher::new(setting.primary.as_str()),
            })),
            "selector-max-attribute" => {
                selector_max_attribute::SelectorMaxAttribute::from_config(&setting.primary, setting.secondary.as_ref())
                    .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "selector-max-class" => selector_max_class::R::from_config(&setting.primary)
                .map(|r| Box::new(r) as Box<dyn Rule>),
            "selector-max-combinators" => selector_max_combinators::R::from_config(&setting.primary)
                .map(|r| Box::new(r) as Box<dyn Rule>),
            "selector-max-compound-selectors" => selector_max_compound_selectors::R::from_config(
                &setting.primary,
                setting.secondary.as_ref(),
            )
            .map(|r| Box::new(r) as Box<dyn Rule>),
            "selector-max-pseudo-class" => selector_max_pseudo_class::R::from_config(&setting.primary)
                .map(|r| Box::new(r) as Box<dyn Rule>),
            "selector-max-specificity" => selector_max_specificity::SelectorMaxSpecificity::from_config(&setting.primary)
                .map(|r| Box::new(r) as Box<dyn Rule>),
            "selector-max-type" => selector_max_type::R::from_config(
                &setting.primary,
                setting.secondary.as_ref(),
            )
            .map(|r| Box::new(r) as Box<dyn Rule>),
            "selector-max-universal" => selector_max_universal::R::from_config(
                &setting.primary,
                setting.secondary.as_ref(),
            )
            .map(|r| Box::new(r) as Box<dyn Rule>),
            "selector-max-id" => Some(Box::new(selector_max_id::SelectorMaxId::from_config(
                &setting.primary,
                setting.secondary.as_ref(),
            ))),
            "selector-disallowed-list" => {
                Some(Box::new(selector_disallowed_list::SelectorDisallowedList::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                )))
            }
            "selector-nested-pattern" => {
                Some(Box::new(selector_nested_pattern::SelectorNestedPattern::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                )))
            }
            "rule-selector-property-disallowed-list" => {
                rule_selector_property_disallowed_list::RuleSelectorPropertyDisallowedList::from_config(
                    &setting.primary,
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "no-unknown-custom-media" => Some(Box::new(
                no_unknown_custom_media::NoUnknownCustomMedia,
            )),
            "syntax-string-no-invalid" => Some(Box::new(
                syntax_string_no_invalid::SyntaxStringNoInvalid,
            )),
            "selector-no-vendor-prefix" => {
                Some(Box::new(selector_no_vendor_prefix::SelectorNoVendorPrefix::from_secondary(
                    setting.secondary.as_ref(),
                )))
            }
            "selector-not-notation" => {
                let mode = if setting.primary.as_str() == Some("simple") {
                    selector_not_notation::SelectorNotMode::Simple
                } else {
                    selector_not_notation::SelectorNotMode::Complex
                };
                Some(Box::new(selector_not_notation::SelectorNotNotation {
                    mode,
                }))
            }
            "selector-pseudo-class-no-unknown" => Some(Box::new(
                selector_pseudo_class_no_unknown::SelectorPseudoClassNoUnknown::from_secondary(
                    setting.secondary.as_ref(),
                ),
            )),
            "selector-pseudo-class-allowed-list" => Some(Box::new(
                selector_pseudo_class_allowed_list::SelectorPseudoClassAllowedList::from_config(
                    &setting.primary,
                ),
            )),
            "selector-pseudo-class-disallowed-list" => Some(Box::new(
                selector_pseudo_class_disallowed_list::SelectorPseudoClassDisallowedList::from_config(
                    &setting.primary,
                ),
            )),
            "selector-pseudo-element-allowed-list" => Some(Box::new(
                selector_pseudo_element_allowed_list::SelectorPseudoElementAllowedList::from_config(
                    &setting.primary,
                ),
            )),
            "selector-pseudo-element-disallowed-list" => Some(Box::new(
                selector_pseudo_element_disallowed_list::SelectorPseudoElementDisallowedList::from_config(
                    &setting.primary,
                ),
            )),
            "selector-pseudo-element-colon-notation" => {
                let mode = if setting.primary.as_str() == Some("single") {
                    selector_pseudo_element_colon_notation::ColonNotationMode::Single
                } else {
                    selector_pseudo_element_colon_notation::ColonNotationMode::Double
                };
                Some(Box::new(
                    selector_pseudo_element_colon_notation::SelectorPseudoElementColonNotation {
                        mode,
                    },
                ))
            }
            "selector-pseudo-element-no-unknown" => Some(Box::new(
                selector_pseudo_element_no_unknown::SelectorPseudoElementNoUnknown::from_secondary(
                    setting.secondary.as_ref(),
                ),
            )),
            "selector-type-case" => {
                let mode = if setting.primary.as_str() == Some("upper") {
                    function_name_case::CaseMode::Upper
                } else {
                    function_name_case::CaseMode::Lower
                };
                Some(Box::new(selector_type_case::SelectorTypeCase {
                    mode,
                    ignore_types: crate::utils::parse_option_matchers(setting.secondary.as_ref(), "ignoreTypes"),
                }))
            }
            "selector-type-no-unknown" => Some(Box::new(
                selector_type_no_unknown::SelectorTypeNoUnknown::from_secondary(setting.secondary.as_ref()),
            )),
            "shorthand-property-no-redundant-values" => Some(Box::new(
                shorthand_property_no_redundant_values::ShorthandPropertyNoRedundantValues::from_secondary(
                    setting.secondary.as_ref(),
                ),
            )),
            "string-no-newline" => Some(Box::new(string_no_newline::StringNoNewline::from_config(
                setting.secondary.as_ref(),
            ))),
            "time-min-milliseconds" => {
                time_min_milliseconds::TimeMinMilliseconds::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "unit-layout-mappings" => {
                unit_layout_mappings::UnitLayoutMappings::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "value-keyword-layout-mappings" => {
                value_keyword_layout_mappings::ValueKeywordLayoutMappings::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                )
                .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "unit-allowed-list" => Some(Box::new(
                unit_allowed_list::UnitAllowedList::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                ),
            )),
            "unit-disallowed-list" => Some(Box::new(
                unit_disallowed_list::UnitDisallowedList::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                ),
            )),
            "unit-no-unknown" => Some(Box::new(
                unit_no_unknown::UnitNoUnknown::from_config(setting.secondary.as_ref()),
            )),
            "value-keyword-case" => {
                let mode = if setting.primary.as_str() == Some("upper") {
                    function_name_case::CaseMode::Upper
                } else {
                    function_name_case::CaseMode::Lower
                };
                Some(Box::new(value_keyword_case::ValueKeywordCase::new(
                    mode,
                    setting.secondary.as_ref(),
                )))
            }
            "max-nesting-depth" => {
                max_nesting_depth::MaxNestingDepth::from_config(&setting.primary, setting.secondary.as_ref())
                    .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "nesting-selector-no-missing-scoping-root" => Some(Box::new(
                nesting_selector_no_missing_scoping_root::NestingSelectorNoMissingScopingRoot::new(setting.secondary.as_ref()),
            )),
            "relative-selector-nesting-notation" => relative_selector_nesting_notation::RelativeSelectorNestingNotation::from_config(&setting.primary)
                .map(|r| Box::new(r) as Box<dyn Rule>),
            "selector-no-deprecated" => Some(Box::new(
                selector_no_deprecated::SelectorNoDeprecated::new(setting.secondary.as_ref()),
            )),
            "selector-no-qualifying-type" => Some(Box::new(
                selector_no_qualifying_type::SelectorNoQualifyingType::new(setting.secondary.as_ref()),
            )),
            "media-feature-name-value-no-unknown" => Some(Box::new(
                media_feature_name_value_no_unknown::MediaFeatureNameValueNoUnknown::new(
                    setting.secondary.as_ref(),
                ),
            )),
            "property-layout-mappings" => {
                property_layout_mappings::PropertyLayoutMappings::from_config(&setting.primary, setting.secondary.as_ref())
                    .map(|r| Box::new(r) as Box<dyn Rule>)
            }
            "selector-no-unmatchable" => Some(Box::new(selector_no_unmatchable::SelectorNoUnmatchable)),
            "selector-no-invalid" => Some(Box::new(selector_no_invalid::SelectorNoInvalid)),
            "value-no-invalid" => Some(Box::new(value_no_invalid::ValueNoInvalid)),
            "value-type-custom-property-required-list" => Some(Box::new(
                value_type_custom_property_required_list::ValueTypeCustomPropertyRequiredList::from_config(
                    &setting.primary,
                    setting.secondary.as_ref(),
                ),
            )),
            "declaration-property-value-no-unknown" => Some(Box::new(
                declaration_property_value_no_unknown::DeclarationPropertyValueNoUnknown::from_config(
                    setting.secondary.as_ref(),
                ),
            )),
            "value-no-vendor-prefix" => Some(Box::new(
                value_no_vendor_prefix::ValueNoVendorPrefix::from_secondary(
                    setting.secondary.as_ref(),
                ),
            )),
            _ => None,
        };
        if let Some(rule) = rule {
            active.push(ConfiguredRule {
                rule,
                severity: setting.severity,
                custom_message: setting
                    .secondary
                    .as_ref()
                    .and_then(|secondary| secondary.get("message"))
                    .and_then(serde_json::Value::as_str)
                    .map(Arc::from),
            });
        }
    }
    active.sort_unstable_by_key(|configured| configured.rule.name());
    active
}

pub fn is_known_rule(name: &str, setting: Option<&crate::config::RuleSetting>) -> bool {
    let is_registered = |setting| {
        let mut config = Config::default();
        config.rules.insert(name.to_string(), Some(setting));
        build_rules(&config)
            .iter()
            .any(|configured| configured.rule.name() == name)
    };
    if setting.is_some_and(|setting| is_registered(setting.clone())) {
        return true;
    }
    let probes = [
        serde_json::Value::Bool(true),
        serde_json::Value::Number(0.into()),
        serde_json::Value::String("always".into()),
        serde_json::Value::String("lower".into()),
        serde_json::Value::String("short".into()),
        serde_json::Value::String("with-alpha".into()),
        serde_json::Value::String("without-alpha".into()),
        serde_json::Value::String("^x$".into()),
        serde_json::Value::Array(Vec::new()),
        serde_json::Value::Object(serde_json::Map::new()),
    ];
    probes.into_iter().any(|primary| {
        is_registered(crate::config::RuleSetting {
            primary,
            secondary: None,
            severity: Severity::Error,
        })
    })
}
