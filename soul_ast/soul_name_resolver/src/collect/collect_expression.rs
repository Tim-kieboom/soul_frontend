use parser_models::ast::{ElseKind, Expression, ExpressionKind, If};
use soul_utils::error::{SoulError, SoulErrorKind};
use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_expression(&mut self, expression: &mut Expression) {

        match &mut expression.node {
            ExpressionKind::Null(node_id) => {
                *node_id = Some(self.alloc_id());
            }
            ExpressionKind::As(type_cast) => {
                type_cast.id = Some(self.alloc_id());
                self.collect_expression(&mut type_cast.left);
                self.collect_type(&mut type_cast.type_cast);
            }
            ExpressionKind::If(r#if) => {
                r#if.id = Some(self.alloc_id());
                self.collect_if(r#if);
            }
            ExpressionKind::While(r#while) => {
                r#while.id = Some(self.alloc_id());
                if let Some(condition) = &mut r#while.condition {
                    self.collect_expression(condition);
                }
                self.collect_block(&mut r#while.block);
            }
            ExpressionKind::Index(index) => {
                index.id = Some(self.alloc_id());
                self.collect_expression(&mut index.collection);
                self.collect_expression(&mut index.index);
            }
            ExpressionKind::Unary(unary) => {
                unary.id = Some(self.alloc_id());
                self.collect_expression(&mut unary.expression);
            }
            ExpressionKind::Block(block) => {
                block.node_id = Some(self.alloc_id());
                self.collect_block(block);
            }
            ExpressionKind::Binary(binary) => {
                binary.id = Some(self.alloc_id());
                self.collect_expression(&mut binary.left);
                self.collect_expression(&mut binary.right);
            }
            ExpressionKind::Deref { inner, id } => {
                *id = Some(self.alloc_id());
                self.collect_expression(inner);
            }
            ExpressionKind::ReturnLike(return_like) => {
                return_like.id = Some(self.alloc_id());
                if let Some(value) = &mut return_like.value {
                    self.collect_expression(value);
                } 
            }
            ExpressionKind::FunctionCall(function_call) => {
                function_call.id = Some(self.alloc_id());
                for arg in &mut function_call.arguments {
                    self.collect_expression(arg);
                }
                if let Some(callee) = &mut function_call.callee {
                    self.collect_expression(callee);
                }
            }
            ExpressionKind::Ref { expression, id, .. } => {
                *id = Some(self.alloc_id());
                self.collect_expression(expression);
            }
            ExpressionKind::ExternalExpression(_) => todo!("impl external expressions"),
            ExpressionKind::Default(id) => *id = Some(self.alloc_id()),
            ExpressionKind::Literal((id, _)) => *id = Some(self.alloc_id()),
            ExpressionKind::Variable { id, ident, .. } => {
                
                let variable_id = match self.check_variable(ident) {
                    Some(id) => id,
                    _ => {
                        self.log_error(SoulError::new(
                            format!("variable '{}' not found in scope", ident.as_str()),
                            SoulErrorKind::NotFoundInScope,
                            Some(ident.get_span()),
                        ));
                        self.alloc_id()
                    }
                };

                *id = Some(variable_id);
            }
            ExpressionKind::Array(array) => {
                array.id = Some(self.alloc_id());
                for value in &mut array.values {
                    self.collect_expression(value);
                }
            },
        }
    }

    fn collect_if(&mut self, r#if: &mut If) {
        self.collect_expression(&mut r#if.condition);
        self.collect_block(&mut r#if.block);

        let mut current = r#if.else_branchs.as_mut();
            while let Some(branch) = current {
            match &mut branch.node {
                ElseKind::Else(el) => {
                    self.collect_block(&mut el.node);
                    current = None;
                }
                ElseKind::ElseIf(el) => {
                    self.collect_expression(&mut el.node.condition);
                    self.collect_block(&mut el.node.block);
                    debug_assert!(el.node.else_branchs.is_none());
                    current = el.node.else_branchs.as_mut();
                }
            }
        }
    }   
}