use parser_models::ast::{Expression, ExpressionGroup, ExpressionKind};
use soul_utils::error::{SoulError, SoulErrorKind};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_expression(&mut self, expression: &mut Expression) {
        match &mut expression.node {
            
            ExpressionKind::Index(index) => {
                self.resolve_expression(&mut index.collection);
                self.resolve_expression(&mut index.index);
            }
            ExpressionKind::FunctionCall(function_call) => {
                function_call.id = self.lookup_function(&function_call.name);
                
                if function_call.id.is_none() {
                    self.log_error(SoulError::new(
                        format!(
                            "function '{}' is undefined in scope",
                            function_call.name.as_str(),
                        ),
                        SoulErrorKind::NotFoundInScope,
                        Some(expression.span),
                    ));
                };
                
                for arg in &mut function_call.arguments {
                    self.resolve_expression(arg);
                }
            }
            ExpressionKind::FieldAccess(field_access) => {
                self.resolve_expression(&mut field_access.parent);
            }
            ExpressionKind::Variable { id:_, ident, resolved } => {
                self.resolve_variable(ident, resolved, expression.span);
            }
            ExpressionKind::Unary(unary) => {
                self.resolve_expression(&mut unary.expression);
            }
            ExpressionKind::Binary(binary) => {
                self.resolve_expression(&mut binary.left);
                self.resolve_expression(&mut binary.right);
            }
            ExpressionKind::StructConstructor(struct_constructor) => {
                for (_name, value) in &mut struct_constructor.named_tuple.values {
                    self.resolve_expression(value);
                }
            }
            ExpressionKind::If(r#if) => {
                self.resolve_expression(&mut r#if.condition);
                self.resolve_block(&mut r#if.block);
            }
            ExpressionKind::While(r#while) => {
                if let Some(value) = &mut r#while.condition {
                    self.resolve_expression(value);
                }
                self.resolve_block(&mut r#while.block);
            }
            ExpressionKind::Deref{ id:_, inner } => {
                self.resolve_expression(inner);
            }
            ExpressionKind::Ref { expression, .. } => {
                self.resolve_expression(expression);
            }
            ExpressionKind::Block(block) => {
                self.resolve_block(block);
            }
            ExpressionKind::ReturnLike(return_like) => {
                if let Some(value) = &mut return_like.value {
                    self.resolve_expression(value);
                }
            }
            ExpressionKind::ExpressionGroup { id:_, group } => {
                self.resolve_expression_group(group);
            }
            ExpressionKind::TypeNamespace(_) => todo!("impl typeNamespace"),

            ExpressionKind::Default(_) 
            | ExpressionKind::Literal(_) 
            | ExpressionKind::ExternalExpression(_) => (),
        }
    }

    fn resolve_expression_group(&mut self, expression_group: &mut ExpressionGroup) {
        match expression_group {
            ExpressionGroup::Tuple(tuple) => {
                for value in tuple {
                    self.resolve_expression(value);
                }
            }
            ExpressionGroup::Array(array) => {
                for value in &mut array.values {
                    self.resolve_expression(value);
                }
            }
            ExpressionGroup::NamedTuple(named_tuple) => {
                for (_name, value) in &mut named_tuple.values {
                    self.resolve_expression(value);
                }
            }
        }
    }   
}