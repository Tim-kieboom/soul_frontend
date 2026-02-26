use ast::{Expression, ExpressionKind};
use soul_utils::error::{SoulError, SoulErrorKind};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_expression(&mut self, expression: &mut Expression) {
        let span = expression.span;
        match &mut expression.node {
            ExpressionKind::As(type_cast) => {
                self.resolve_expression(&mut type_cast.left);
            }
            ExpressionKind::Index(index) => {
                self.resolve_expression(&mut index.collection);
                self.resolve_expression(&mut index.index);
            }
            ExpressionKind::FunctionCall(function_call) => {
                function_call.resolved = self.lookup_function(&function_call.name);

                if function_call.resolved.is_none() {
                    self.log_error(SoulError::new(
                        format!(
                            "function '{}' is undefined in scope",
                            function_call.name.as_str(),
                        ),
                        SoulErrorKind::NotFoundInScope,
                        Some(span),
                    ));
                };

                for arg in &mut function_call.arguments {
                    self.resolve_expression(arg);
                }
            }
            ExpressionKind::Variable {
                id: _,
                ident,
                resolved,
            } => {
                self.resolve_variable(ident, resolved, span);
            }
            ExpressionKind::Unary(unary) => {
                self.resolve_expression(&mut unary.expression);
            }
            ExpressionKind::Binary(binary) => {
                self.resolve_expression(&mut binary.left);
                self.resolve_expression(&mut binary.right);
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
            ExpressionKind::Deref { id: _, inner } => {
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

            ExpressionKind::Array(array) => {
                for value in &mut array.values {
                    self.resolve_expression(value);
                }
            }

            ExpressionKind::Null(_)
            | ExpressionKind::Default(_)
            | ExpressionKind::Literal(_)
            | ExpressionKind::ExternalExpression(_) => (),
        }
    }
}
