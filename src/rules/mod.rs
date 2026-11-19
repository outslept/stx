pub mod alpha_value_notation;
pub mod at_rule_no_unknown;
pub mod block_no_empty;
pub mod color_function_alias_notation;
pub mod color_function_notation;
pub mod color_hex_length;
pub mod color_no_invalid_hex;
pub mod comment_no_empty;
pub mod custom_property_no_missing_var_function;
pub mod declaration_block_no_duplicate_custom_properties;
pub mod declaration_block_no_duplicate_properties;
pub mod declaration_block_no_shorthand_property_overrides;
pub mod declaration_no_important;
pub mod font_family_name_quotes;
pub mod font_family_no_duplicate_names;
pub mod font_family_no_missing_generic_family_keyword;
pub mod function_calc_no_unspaced_operator;
pub mod function_linear_gradient_no_nonstandard_direction;
pub mod function_name_case;
pub mod function_no_unknown;
pub mod function_url_quotes;
pub mod hue_degree_notation;
pub mod keyframe_block_no_duplicate_selectors;
pub mod keyframe_declaration_no_important;
pub mod length_zero_no_unit;
pub mod lightness_notation;
pub mod media_feature_name_no_unknown;
pub mod named_grid_areas_no_invalid;
pub mod no_descending_specificity;
pub mod no_duplicate_at_import_rules;
pub mod no_duplicate_selectors;
pub mod no_empty_source;
pub mod no_invalid_double_slash_comments;
pub mod no_invalid_position_at_import_rule;
pub mod no_invalid_position_declaration;
pub mod no_irregular_whitespace;
pub mod number_max_precision;
pub mod property_no_unknown;
pub mod selector_anb_no_unmatchable;
pub mod selector_max_id;
pub mod selector_pseudo_class_no_unknown;
pub mod selector_pseudo_element_no_unknown;
pub mod selector_type_no_unknown;
pub mod shorthand_property_no_redundant_values;
pub mod string_no_newline;
pub mod unit_no_unknown;
pub mod value_keyword_case;
pub mod value_no_vendor_prefix;
pub mod value_utils;

use crate::{config::Config, context::LintContext, diagnostic::Severity};
use oxc_css_parser::{
    ast::{
        AtRule, ComplexSelector, ComponentValue, CompoundSelector, Declaration, KeyframeBlock,
        QualifiedRule, SelectorList, SimpleBlock, SimpleSelector, Stylesheet,
    },
    token::Comment,
};

pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn check_stylesheet<'a>(&self, _ast: &'a Stylesheet<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_comment<'a>(&self, _comment: &'a Comment<'a>, _ctx: &mut LintContext<'a>) {}
    fn check_qualified_rule<'a>(&self, _rule: &'a QualifiedRule<'a>, _ctx: &mut LintContext<'a>) {}
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

pub struct ConfiguredRule {
    pub rule: Box<dyn Rule>,
    pub severity: Severity,
}

pub fn build_rules(config: &Config) -> Vec<ConfiguredRule> {
    let mut active = Vec::new();

    for (name, maybe_setting) in &config.rules {
        let Some(setting) = maybe_setting else {
            continue;
        };

        let rule: Option<Box<dyn Rule>> = match name.as_str() {
            "alpha-value-notation" => {
                let mode = if setting.primary.as_str() == Some("number") {
                    alpha_value_notation::AlphaNotationMode::Number
                } else {
                    alpha_value_notation::AlphaNotationMode::Percentage
                };
                Some(Box::new(alpha_value_notation::AlphaValueNotation { mode }))
            }
            "at-rule-no-unknown" => Some(Box::new(at_rule_no_unknown::AtRuleNoUnknown)),
            "block-no-empty" => Some(Box::new(block_no_empty::BlockNoEmpty)),
            "color-function-alias-notation" => {
                let mode = if setting.primary.as_str() == Some("with-alpha") {
                    color_function_alias_notation::ColorFunctionAliasMode::WithAlpha
                } else {
                    color_function_alias_notation::ColorFunctionAliasMode::WithoutAlpha
                };
                Some(Box::new(
                    color_function_alias_notation::ColorFunctionAliasNotation { mode },
                ))
            }
            "color-function-notation" => {
                let mode = if setting.primary.as_str() == Some("legacy") {
                    color_function_notation::ColorFunctionNotationMode::Legacy
                } else {
                    color_function_notation::ColorFunctionNotationMode::Modern
                };
                Some(Box::new(color_function_notation::ColorFunctionNotation {
                    mode,
                }))
            }
            "color-hex-length" => {
                let mode = if setting.primary.as_str() == Some("long") {
                    color_hex_length::HexLengthMode::Long
                } else {
                    color_hex_length::HexLengthMode::Short
                };
                Some(Box::new(color_hex_length::ColorHexLength { mode }))
            }
            "color-no-invalid-hex" => Some(Box::new(color_no_invalid_hex::ColorNoInvalidHex)),
            "comment-no-empty" => Some(Box::new(comment_no_empty::CommentNoEmpty)),
            "custom-property-no-missing-var-function" => Some(Box::new(
                custom_property_no_missing_var_function::CustomPropertyNoMissingVarFunction,
            )),
            "declaration-block-no-duplicate-custom-properties" => Some(Box::new(
                declaration_block_no_duplicate_custom_properties::DeclarationBlockNoDuplicateCustomProperties,
            )),
            "declaration-block-no-duplicate-properties" => Some(Box::new(
                declaration_block_no_duplicate_properties::DeclarationBlockNoDuplicateProperties,
            )),
            "declaration-block-no-shorthand-property-overrides" => Some(Box::new(
                declaration_block_no_shorthand_property_overrides::DeclarationBlockNoShorthandPropertyOverrides,
            )),
            "declaration-no-important" => {
                Some(Box::new(declaration_no_important::DeclarationNoImportant))
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
                font_family_no_duplicate_names::FontFamilyNoDuplicateNames,
            )),
            "font-family-no-missing-generic-family-keyword" => Some(Box::new(
                font_family_no_missing_generic_family_keyword::FontFamilyNoMissingGenericFamilyKeyword,
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
                Some(Box::new(function_name_case::FunctionNameCase { mode }))
            }
            "function-no-unknown" => Some(Box::new(function_no_unknown::FunctionNoUnknown)),
            "function-url-quotes" => {
                let mode = if setting.primary.as_str() == Some("never") {
                    function_url_quotes::UrlQuoteMode::Never
                } else {
                    function_url_quotes::UrlQuoteMode::Always
                };
                Some(Box::new(function_url_quotes::FunctionUrlQuotes { mode }))
            }
            "hue-degree-notation" => {
                let mode = if setting.primary.as_str() == Some("number") {
                    hue_degree_notation::HueDegreeMode::Number
                } else {
                    hue_degree_notation::HueDegreeMode::Angle
                };
                Some(Box::new(hue_degree_notation::HueDegreeNotation { mode }))
            }
            "keyframe-block-no-duplicate-selectors" => Some(Box::new(
                keyframe_block_no_duplicate_selectors::KeyframeBlockNoDuplicateSelectors,
            )),
            "keyframe-declaration-no-important" => Some(Box::new(
                keyframe_declaration_no_important::KeyframeDeclarationNoImportant,
            )),
            "length-zero-no-unit" => Some(Box::new(length_zero_no_unit::LengthZeroNoUnit)),
            "lightness-notation" => {
                let mode = if setting.primary.as_str() == Some("number") {
                    lightness_notation::LightnessMode::Number
                } else {
                    lightness_notation::LightnessMode::Percentage
                };
                Some(Box::new(lightness_notation::LightnessNotation { mode }))
            }
            "media-feature-name-no-unknown" => Some(Box::new(
                media_feature_name_no_unknown::MediaFeatureNameNoUnknown,
            )),
            "named-grid-areas-no-invalid" => Some(Box::new(
                named_grid_areas_no_invalid::NamedGridAreasNoInvalid,
            )),
            "no-descending-specificity" => {
                Some(Box::new(no_descending_specificity::NoDescendingSpecificity))
            }
            "no-duplicate-at-import-rules" => Some(Box::new(
                no_duplicate_at_import_rules::NoDuplicateAtImportRules,
            )),
            "no-duplicate-selectors" => {
                Some(Box::new(no_duplicate_selectors::NoDuplicateSelectors))
            }
            "no-empty-source" => Some(Box::new(no_empty_source::NoEmptySource)),
            "no-invalid-double-slash-comments" => Some(Box::new(
                no_invalid_double_slash_comments::NoInvalidDoubleSlashComments,
            )),
            "no-invalid-position-at-import-rule" => Some(Box::new(
                no_invalid_position_at_import_rule::NoInvalidPositionAtImportRule,
            )),
            "no-invalid-position-declaration" => Some(Box::new(
                no_invalid_position_declaration::NoInvalidPositionDeclaration,
            )),
            "no-irregular-whitespace" => {
                Some(Box::new(no_irregular_whitespace::NoIrregularWhitespace))
            }
            "number-max-precision" => {
                let max = setting.primary.as_u64().unwrap_or(4) as usize;
                Some(Box::new(number_max_precision::NumberMaxPrecision { max }))
            }
            "property-no-unknown" => Some(Box::new(property_no_unknown::PropertyNoUnknown)),
            "selector-anb-no-unmatchable" => Some(Box::new(
                selector_anb_no_unmatchable::SelectorAnbNoUnmatchable,
            )),
            "selector-max-id" => {
                let max = setting.primary.as_u64().unwrap_or(0) as usize;
                Some(Box::new(selector_max_id::SelectorMaxId { max }))
            }
            "selector-pseudo-class-no-unknown" => Some(Box::new(
                selector_pseudo_class_no_unknown::SelectorPseudoClassNoUnknown,
            )),
            "selector-pseudo-element-no-unknown" => Some(Box::new(
                selector_pseudo_element_no_unknown::SelectorPseudoElementNoUnknown,
            )),
            "selector-type-no-unknown" => {
                Some(Box::new(selector_type_no_unknown::SelectorTypeNoUnknown))
            }
            "shorthand-property-no-redundant-values" => Some(Box::new(
                shorthand_property_no_redundant_values::ShorthandPropertyNoRedundantValues,
            )),
            "string-no-newline" => Some(Box::new(string_no_newline::StringNoNewline)),
            "unit-no-unknown" => Some(Box::new(unit_no_unknown::UnitNoUnknown)),
            "value-keyword-case" => {
                let mode = if setting.primary.as_str() == Some("upper") {
                    function_name_case::CaseMode::Upper
                } else {
                    function_name_case::CaseMode::Lower
                };
                let camel_case_svg_keywords = setting
                    .secondary
                    .as_ref()
                    .and_then(|m| m.get("camelCaseSvgKeywords"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Some(Box::new(value_keyword_case::ValueKeywordCase {
                    mode,
                    camel_case_svg_keywords,
                }))
            }
            "value-no-vendor-prefix" => {
                Some(Box::new(value_no_vendor_prefix::ValueNoVendorPrefix))
            }
            _ => None,
        };

        if let Some(rule) = rule {
            active.push(ConfiguredRule {
                rule,
                severity: setting.severity,
            });
        }
    }

    active
}
