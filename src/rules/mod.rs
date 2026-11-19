pub mod at_rule_no_unknown;
pub mod block_no_empty;
pub mod color_hex_length;
pub mod color_no_invalid_hex;
pub mod comment_no_empty;
pub mod custom_property_no_missing_var_function;
pub mod declaration_block_no_duplicate_custom_properties;
pub mod declaration_block_no_duplicate_properties;
pub mod declaration_block_no_shorthand_property_overrides;
pub mod declaration_no_important;
pub mod font_family_no_duplicate_names;
pub mod font_family_no_missing_generic_family_keyword;
pub mod function_calc_no_unspaced_operator;
pub mod function_linear_gradient_no_nonstandard_direction;
pub mod function_no_unknown;
pub mod keyframe_block_no_duplicate_selectors;
pub mod keyframe_declaration_no_important;
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
pub mod property_no_unknown;
pub mod selector_anb_no_unmatchable;
pub mod selector_max_id;
pub mod selector_pseudo_class_no_unknown;
pub mod selector_pseudo_element_no_unknown;
pub mod selector_type_no_unknown;
pub mod string_no_newline;
pub mod unit_no_unknown;

use crate::{
    config::{Config, RuleSetting},
    context::LintContext,
    diagnostic::Severity,
};
use oxc_css_parser::{ast::*, token::Comment};

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

    for (name, opt_setting) in &config.rules {
        let Some(setting) = opt_setting else {
            continue;
        };
        if let Some(rule) = instantiate_rule(name, setting) {
            active.push(ConfiguredRule {
                rule,
                severity: setting.severity,
            });
        }
    }

    active
}

fn instantiate_rule(name: &str, setting: &RuleSetting) -> Option<Box<dyn Rule>> {
    match name {
        "at-rule-no-unknown" => Some(Box::new(at_rule_no_unknown::AtRuleNoUnknown)),
        "block-no-empty" => Some(Box::new(block_no_empty::BlockNoEmpty)),
        "comment-no-empty" => Some(Box::new(comment_no_empty::CommentNoEmpty)),
        "custom-property-no-missing-var-function" => Some(Box::new(
            custom_property_no_missing_var_function::CustomPropertyNoMissingVarFunction,
        )),
        "no-empty-source" => Some(Box::new(no_empty_source::NoEmptySource)),
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
        "function-no-unknown" => Some(Box::new(function_no_unknown::FunctionNoUnknown)),
        "keyframe-block-no-duplicate-selectors" => Some(Box::new(
            keyframe_block_no_duplicate_selectors::KeyframeBlockNoDuplicateSelectors,
        )),
        "keyframe-declaration-no-important" => Some(Box::new(
            keyframe_declaration_no_important::KeyframeDeclarationNoImportant,
        )),
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
        "no-duplicate-selectors" => Some(Box::new(no_duplicate_selectors::NoDuplicateSelectors)),
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
        "property-no-unknown" => Some(Box::new(property_no_unknown::PropertyNoUnknown)),
        "selector-anb-no-unmatchable" => Some(Box::new(
            selector_anb_no_unmatchable::SelectorAnbNoUnmatchable,
        )),
        "selector-pseudo-class-no-unknown" => Some(Box::new(
            selector_pseudo_class_no_unknown::SelectorPseudoClassNoUnknown,
        )),
        "selector-pseudo-element-no-unknown" => Some(Box::new(
            selector_pseudo_element_no_unknown::SelectorPseudoElementNoUnknown,
        )),
        "selector-type-no-unknown" => {
            Some(Box::new(selector_type_no_unknown::SelectorTypeNoUnknown))
        }
        "string-no-newline" => Some(Box::new(string_no_newline::StringNoNewline)),
        "unit-no-unknown" => Some(Box::new(unit_no_unknown::UnitNoUnknown)),
        "color-no-invalid-hex" => Some(Box::new(color_no_invalid_hex::ColorNoInvalidHex)),
        "color-hex-length" => {
            let mode = match setting.primary.as_str() {
                Some("long") => color_hex_length::HexLengthMode::Long,
                _ => color_hex_length::HexLengthMode::Short,
            };
            Some(Box::new(color_hex_length::ColorHexLength { mode }))
        }
        "selector-max-id" => {
            let max = setting.primary.as_u64().unwrap_or(0) as usize;
            Some(Box::new(selector_max_id::SelectorMaxId { max }))
        }
        _ => None,
    }
}
