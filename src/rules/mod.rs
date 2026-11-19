pub mod block_no_empty;
pub mod color_no_invalid_hex;
pub mod comment_no_empty;
pub mod declaration_no_important;
pub mod keyframe_declaration_no_important;
pub mod no_empty_source;
pub mod selector_max_id;

use crate::context::LintContext;
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

pub fn default_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(block_no_empty::BlockNoEmpty),
        Box::new(comment_no_empty::CommentNoEmpty),
        Box::new(no_empty_source::NoEmptySource),
        Box::new(declaration_no_important::DeclarationNoImportant),
        Box::new(keyframe_declaration_no_important::KeyframeDeclarationNoImportant),
        Box::new(color_no_invalid_hex::ColorNoInvalidHex),
        Box::new(selector_max_id::SelectorMaxId { max: 0 }),
    ]
}
