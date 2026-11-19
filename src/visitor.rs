use crate::{
    context::{AstParent, LintContext},
    rules::ConfiguredRule,
};
use oxc_css_parser::ast::*;

pub struct SinglePassVisitor<'r, 'a> {
    ctx: &'r mut LintContext<'a>,
    rules: &'r [ConfiguredRule],
}

impl<'r, 'a> SinglePassVisitor<'r, 'a> {
    pub fn new(ctx: &'r mut LintContext<'a>, rules: &'r [ConfiguredRule]) -> Self {
        Self { ctx, rules }
    }

    pub fn visit_stylesheet(&mut self, stylesheet: &'a Stylesheet<'a>) {
        for r in self.rules {
            self.ctx.current_severity = r.severity;
            r.rule.check_stylesheet(stylesheet, self.ctx);
        }

        for comment in self.ctx.comments {
            for r in self.rules {
                self.ctx.current_severity = r.severity;
                r.rule.check_comment(comment, self.ctx);
            }
        }

        self.visit_statements(&stylesheet.statements);
    }

    fn visit_statements(&mut self, statements: &'a [Statement<'a>]) {
        for stmt in statements {
            match stmt {
                Statement::QualifiedRule(qrule) => {
                    for r in self.rules {
                        self.ctx.current_severity = r.severity;
                        r.rule.check_qualified_rule(qrule, self.ctx);
                    }
                    self.ctx.ancestry.push(AstParent::QualifiedRule(qrule));
                    self.visit_selector_list(&qrule.selector);
                    self.visit_simple_block(&qrule.block);
                    self.ctx.ancestry.pop();
                }
                Statement::AtRule(at_rule) => {
                    for r in self.rules {
                        self.ctx.current_severity = r.severity;
                        r.rule.check_at_rule(at_rule, self.ctx);
                    }
                    self.ctx.ancestry.push(AstParent::AtRule(at_rule));
                    if let Some(block) = &at_rule.block {
                        self.visit_simple_block(block);
                    }
                    self.ctx.ancestry.pop();
                }
                Statement::KeyframeBlock(kf_block) => {
                    for r in self.rules {
                        self.ctx.current_severity = r.severity;
                        r.rule.check_keyframe_block(kf_block, self.ctx);
                    }
                    self.ctx.ancestry.push(AstParent::KeyframeBlock(kf_block));
                    self.visit_simple_block(&kf_block.block);
                    self.ctx.ancestry.pop();
                }
                Statement::Declaration(decl) => {
                    self.visit_declaration(decl);
                }
                Statement::UnknownQualifiedRule(u_rule) => {
                    self.visit_simple_block(&u_rule.block);
                }
                Statement::SassIfAtRule(sass_if) => {
                    self.visit_component_value(&sass_if.if_clause.condition);
                    self.visit_simple_block(&sass_if.if_clause.block);
                    for clause in &sass_if.else_if_clauses {
                        self.visit_component_value(&clause.condition);
                        self.visit_simple_block(&clause.block);
                    }
                    if let Some(else_block) = &sass_if.else_clause {
                        self.visit_simple_block(else_block);
                    }
                }
                Statement::SassVariableDeclaration(sass_var) => {
                    self.visit_component_value(&sass_var.value);
                }
                Statement::LessConditionalQualifiedRule(less_rule) => {
                    self.visit_selector_list(&less_rule.selector);
                    self.visit_simple_block(&less_rule.block);
                }
                Statement::LessMixinDefinition(less_mixin) => {
                    self.visit_simple_block(&less_mixin.block);
                }
                Statement::LessVariableDeclaration(less_var) => {
                    self.visit_component_value(&less_var.value);
                }
                Statement::PostcssSimpleVarDeclaration(p_var) => {
                    for val in &p_var.value {
                        self.visit_component_value(val);
                    }
                }
                _ => {}
            }
        }
    }

    fn visit_simple_block(&mut self, block: &'a SimpleBlock<'a>) {
        for r in self.rules {
            self.ctx.current_severity = r.severity;
            r.rule.check_simple_block(block, self.ctx);
        }
        self.ctx.ancestry.push(AstParent::SimpleBlock(block));
        self.visit_statements(&block.statements);
        self.ctx.ancestry.pop();
    }

    fn visit_declaration(&mut self, decl: &'a Declaration<'a>) {
        for r in self.rules {
            self.ctx.current_severity = r.severity;
            r.rule.check_declaration(decl, self.ctx);
        }
        self.ctx.ancestry.push(AstParent::Declaration(decl));
        for val in &decl.value {
            self.visit_component_value(val);
        }
        self.ctx.ancestry.pop();
    }

    fn visit_selector_list(&mut self, list: &'a SelectorList<'a>) {
        for r in self.rules {
            self.ctx.current_severity = r.severity;
            r.rule.check_selector_list(list, self.ctx);
        }
        for complex in &list.selectors {
            self.visit_complex_selector(complex);
        }
    }

    fn visit_complex_selector(&mut self, complex: &'a ComplexSelector<'a>) {
        for r in self.rules {
            self.ctx.current_severity = r.severity;
            r.rule.check_complex_selector(complex, self.ctx);
        }
        for child in &complex.children {
            if let ComplexSelectorChild::CompoundSelector(compound) = child {
                self.visit_compound_selector(compound);
            }
        }
    }

    fn visit_compound_selector(&mut self, compound: &'a CompoundSelector<'a>) {
        for r in self.rules {
            self.ctx.current_severity = r.severity;
            r.rule.check_compound_selector(compound, self.ctx);
        }
        for simple in &compound.children {
            for r in self.rules {
                self.ctx.current_severity = r.severity;
                r.rule.check_simple_selector(simple, self.ctx);
            }
            if let SimpleSelector::PseudoClass(pseudo) = simple {
                if let Some(arg) = &pseudo.arg {
                    match &arg.kind {
                        PseudoClassSelectorArgKind::SelectorList(list) => {
                            self.visit_selector_list(list);
                        }
                        PseudoClassSelectorArgKind::CompoundSelectorList(list) => {
                            for c in &list.selectors {
                                self.visit_compound_selector(c);
                            }
                        }
                        PseudoClassSelectorArgKind::RelativeSelectorList(list) => {
                            for rel in &list.selectors {
                                self.visit_complex_selector(&rel.complex_selector);
                            }
                        }
                        PseudoClassSelectorArgKind::Nth(nth) => {
                            if let Some(matcher) = &nth.matcher {
                                if let Some(list) = &matcher.selector {
                                    self.visit_selector_list(list);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn visit_component_value(&mut self, value: &'a ComponentValue<'a>) {
        for r in self.rules {
            self.ctx.current_severity = r.severity;
            r.rule.check_component_value(value, self.ctx);
        }
        match value {
            ComponentValue::Function(func) => {
                for arg in &func.args {
                    self.visit_component_value(arg);
                }
            }
            ComponentValue::Calc(calc) => {
                self.visit_component_value(&calc.left);
                self.visit_component_value(&calc.right);
            }
            ComponentValue::CalcParenthesized(parens) => {
                self.visit_component_value(&parens.expr);
            }
            ComponentValue::BracketBlock(block) => {
                for val in &block.value {
                    self.visit_component_value(val);
                }
            }
            ComponentValue::SassList(list) => {
                for el in &list.elements {
                    self.visit_component_value(el);
                }
            }
            ComponentValue::SassBinaryExpression(expr) => {
                self.visit_component_value(&expr.left);
                self.visit_component_value(&expr.right);
            }
            ComponentValue::SassParenthesizedExpression(parens) => {
                self.visit_component_value(&parens.expr);
            }
            ComponentValue::SassUnaryExpression(unary) => {
                self.visit_component_value(&unary.expr);
            }
            ComponentValue::SassKeywordArgument(kw) => {
                self.visit_component_value(&kw.value);
            }
            _ => {}
        }
    }
}
