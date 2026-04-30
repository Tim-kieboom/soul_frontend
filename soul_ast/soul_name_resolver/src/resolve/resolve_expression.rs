use ast::{Expression, ExpressionKind, ElseKind, FieldAccess};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_expression(&mut self, expression: &mut Expression) {
        let span = expression.span;
        match &mut expression.node {
            ExpressionKind::Sizeof(_) => (),
            ExpressionKind::ArrayContructor(ctor) => {
                self.resolve_expression(&mut ctor.amount);
                self.resolve_expression(&mut ctor.element);
            }
            ExpressionKind::FieldAccess(field_access) => {
                self.resolve_field_access(field_access);
            }
            ExpressionKind::StructConstructor(ctor) => {
                for (_, value) in &mut ctor.values {
                    self.resolve_expression(value);
                }
            }
            ExpressionKind::As(type_cast) => {
                self.resolve_expression(&mut type_cast.left);
            }
            ExpressionKind::Index(index) => {
                self.resolve_expression(&mut index.collection);
                self.resolve_expression(&mut index.index);
            }
            ExpressionKind::FunctionCall(function_call) => {
                self.resolve_function_call(function_call, span);
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

                let mut current = r#if.else_branchs.as_mut();
                while let Some(branch) = current {
                    match &mut branch.node {
                        ElseKind::Else(el) => {
                            self.resolve_block(&mut el.node);
                            current = None;
                        }
                        ElseKind::ElseIf(el) => {
                            self.resolve_expression(&mut el.node.condition);
                            self.resolve_block(&mut el.node.block);
                            current = el.node.else_branchs.as_mut();
                        }
                    }
                }
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

    fn resolve_field_access(&mut self, field_access: &mut FieldAccess) {
        if self.resolve_module_variable(field_access) {
            return;
        }

        self.resolve_expression(&mut field_access.object);
    }

    fn resolve_module_variable(&mut self, field_access: &mut FieldAccess) -> bool {
        let object_ident = match &field_access.object.node {
            ExpressionKind::Variable { ident, .. } => Some(ident.to_string()),
            _ => None,
        };

        let Some(module_name) = object_ident else {
            return false;
        };

        if self.lookup_module(&module_name).is_none() {
            return false;
        }

        let variable = self.lookup_module_variable(
            &module_name,
            field_access.field.as_str(),
            field_access.field.span,
        );

        if let Some(node_id) = variable {
            field_access.id = Some(node_id);
            if let ExpressionKind::Variable { resolved, .. } = &mut field_access.object.node {
                *resolved = Some(node_id);
            }
            return true;
        }

        false
    }
}
