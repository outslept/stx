use crate::{context::LintContext, diagnostic::Severity};
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

pub struct ConfiguredRule {
    pub rule: Box<dyn Rule>,
    pub severity: Severity,
    pub custom_message: Option<Arc<str>>,
}
