use soul_ast::{
    abstract_syntax_tree::{
        conditionals::{ElseKind, IfCaseKind, Match},
        expression::{Expression, ExpressionKind},
        expression_groups::ExpressionGroup,
    },
    sementic_models::scope::ScopeValueKind,
};

use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_expression(&mut self, expression: &mut Expression) {
        match &mut expression.node {
            ExpressionKind::If(r#if) => {
                self.collect_expression(&mut r#if.condition);
                self.collect_block(&mut r#if.block);
                for branch in &mut r#if.else_branchs {
                    match &mut branch.node {
                        ElseKind::Else(el) => self.collect_block(&mut el.node),
                        ElseKind::ElseIf(el) => {
                            self.collect_expression(&mut el.node.condition);
                            self.collect_block(&mut el.node.block);
                            debug_assert!(el.node.else_branchs.is_empty());
                        }
                    }
                }
            }
            ExpressionKind::For(r#for) => {
                self.collect_expression(&mut r#for.collection);
                self.collect_block(&mut r#for.block);
            }
            ExpressionKind::While(r#while) => {
                if let Some(condition) = &mut r#while.condition {
                    self.collect_expression(condition);
                }
                self.collect_block(&mut r#while.block);
            }
            ExpressionKind::Match(r#match) => {
                self.collect_match(r#match);
            }
            ExpressionKind::Index(index) => {
                self.collect_expression(&mut index.index);
                self.collect_expression(&mut index.collection);
            }
            ExpressionKind::Unary(unary) => {
                self.collect_expression(&mut unary.expression);
            }
            ExpressionKind::Block(block) => self.collect_block(block),
            ExpressionKind::Binary(binary) => {
                self.collect_expression(&mut binary.left);
                self.collect_expression(&mut binary.right);
            }
            ExpressionKind::Deref(spanned) => self.collect_expression(spanned),
            ExpressionKind::Ternary(ternary) => {
                self.collect_expression(&mut ternary.if_branch);
                self.collect_expression(&mut ternary.condition);
                self.collect_expression(&mut ternary.else_branch);
            }
            ExpressionKind::ReturnLike(return_like) => {
                if let Some(value) = &mut return_like.value {
                    self.collect_expression(value);
                }
            }
            ExpressionKind::FieldAccess(field_access) => {
                self.collect_expression(&mut field_access.object)
            }
            ExpressionKind::FunctionCall(function_call) => {
                for arg in &mut function_call.arguments.values {
                    self.collect_expression(arg);
                }
                if let Some(callee) = &mut function_call.callee {
                    self.collect_expression(callee);
                }
            }
            ExpressionKind::Ref {
                is_mutable: _,
                expression,
            } => self.collect_expression(expression),
            ExpressionKind::ExpressionGroup(expression_group) => match expression_group {
                ExpressionGroup::Tuple(tuple) => {
                    for item in &mut tuple.values {
                        self.collect_expression(item)
                    }
                }
                ExpressionGroup::Array(array) => {
                    for item in &mut array.values {
                        self.collect_expression(item)
                    }
                }
                ExpressionGroup::NamedTuple(named_tuple) => {
                    for (_name, item) in &mut named_tuple.values {
                        self.collect_expression(item)
                    }
                }
                ExpressionGroup::ArrayFiller(array_filler) => {
                    self.collect_expression(&mut array_filler.amount);
                    self.collect_expression(&mut array_filler.fill_expr);

                    if let Some(index) = &mut array_filler.index {
                        self.declare_value(ScopeValueKind::Variable(index));
                    }
                }
            },
            ExpressionKind::StructConstructor(struct_constructor) => {
                for (_name, item) in &mut struct_constructor.arguments.values {
                    self.collect_expression(item)
                }
            }

            ExpressionKind::ExternalExpression(_) => todo!("impl external expressoins"),
            ExpressionKind::Lambda(_) => todo!("impl lambda"),
            ExpressionKind::Empty
            | ExpressionKind::Default
            | ExpressionKind::Literal(_)
            | ExpressionKind::Variable { .. }
            | ExpressionKind::StaticFieldAccess(_) => (),
        }
    }

    fn collect_match(&mut self, r#match: &mut Match) {
        self.collect_expression(&mut r#match.condition);
        for case in &mut r#match.cases {
            self.push_scope(&mut case.scope_id);
            match &mut case.if_kind {
                IfCaseKind::WildCard(variable) => {
                    if let Some(variable) = variable {
                        self.declare_value(ScopeValueKind::Variable(variable));
                    }
                }
                IfCaseKind::Expression(expression) => self.collect_expression(expression),
                IfCaseKind::Variant { name: _, params } => {
                    for variable in params {
                        self.declare_value(ScopeValueKind::Variable(variable));
                    }
                }
                IfCaseKind::NamedVariant { name: _, params } => {
                    for (_ident, variable) in params {
                        self.declare_value(ScopeValueKind::Variable(variable));
                    }
                }
                IfCaseKind::Bind {
                    variable,
                    condition,
                } => {
                    let _ = self.declare_value(ScopeValueKind::Variable(variable));
                    self.collect_expression(condition);
                }
            }
            self.pop_scope();
        }
    }
}
