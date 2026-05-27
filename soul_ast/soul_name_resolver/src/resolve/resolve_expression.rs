use ast::{AnyArray, Argument, ElseKind, Expression, ExpressionKind, FieldAccess, FunctionCall};
use soul_utils::Ident;
use soul_utils::error::{SoulError, SoulErrorKind};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_expression(&mut self, expression: &mut Expression) {
        let span = expression.span;
        match &mut expression.node {
            ExpressionKind::Match(match_expression) => {
                self.resolve_expression(&mut match_expression.scrutinee);
                for arm in &mut match_expression.arms {
                    self.resolve_block(&mut arm.body);
                    self.resolve_match_pattern(&arm.pattern);
                }
            }
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

                let mut ref_call = FunctionCall {
                    generics: vec![],
                    id: None,
                    arguments: vec![Argument {
                        name: None,
                        value: (*index.index).clone(),
                    }],
                    resolved: None,
                    name: Ident::new("IndexRef".to_string(), span),
                    callee: Some(Box::new((*index.collection).clone())),
                    external_ref: None,
                    intrinsic: None,
                    intrinsic_value: None,
                };
                self.resolve_function_call(&mut ref_call, span);
                index.index_ref = ref_call.resolved;

                let mut mut_call = FunctionCall {
                    generics: vec![],
                    id: None,
                    arguments: vec![Argument {
                        name: None,
                        value: (*index.index).clone(),
                    }],
                    resolved: None,
                    name: Ident::new("IndexMut".to_string(), span),
                    callee: Some(Box::new((*index.collection).clone())),
                    external_ref: None,
                    intrinsic: None,
                    intrinsic_value: None,
                };
                self.resolve_function_call(&mut mut_call, span);
                index.index_mut = mut_call.resolved;
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
                self.current.in_if_condition = true;
                self.resolve_expression(&mut r#if.condition);
                self.current.in_if_condition = false;
                self.resolve_block(&mut r#if.block);

                let mut current = r#if.else_branchs.as_mut();
                while let Some(branch) = current {
                    match &mut branch.node {
                        ElseKind::Else(el) => {
                            self.resolve_block(&mut el.node);
                            current = None;
                        }
                        ElseKind::ElseIf(el) => {
                            self.current.in_if_condition = true;
                            self.resolve_expression(&mut el.node.condition);
                            self.current.in_if_condition = false;
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

            ExpressionKind::New(expr) => {
                self.resolve_expression(expr);
            }
            ExpressionKind::NewArray(any_array) => match any_array {
                AnyArray::ArrayLiteral(arr) => {
                    for value in &mut arr.values {
                        self.resolve_expression(value);
                    }
                }
                AnyArray::ArrayConstructor(arr) => {
                    self.resolve_expression(&mut arr.amount);
                    self.resolve_expression(&mut arr.element);
                }
            },
            ExpressionKind::Null(_)
            | ExpressionKind::Default(_)
            | ExpressionKind::Literal(_)
            | ExpressionKind::ExternalExpression(_) => (),
            ExpressionKind::TypeOf {
                expr,
                binding,
                type_name: _,
                binding_id: _,
                variant_name: _,
            } => {
                self.resolve_expression(expr);
                if binding.is_some() && !self.current.in_if_condition {
                    self.log_error(SoulError::new(
                        "typeof with binding can only be used as an if condition".to_string(),
                        SoulErrorKind::InvalidContext,
                        Some(span),
                    ));
                }
            }
            ExpressionKind::MatchMethod(mm) => {
                self.resolve_expression(&mut mm.expr);
                for arm in &mut mm.arms {
                    self.resolve_block(&mut arm.body);
                }
            }
        }
    }

    fn resolve_field_access(&mut self, field_access: &mut FieldAccess) {
        if self.resolve_module_variable(field_access) {
            return;
        }

        if self.resolve_enum_variant(field_access) {
            return;
        }

        self.resolve_expression(&mut field_access.object);
    }

    fn resolve_enum_variant(&mut self, field_access: &mut FieldAccess) -> bool {
        let object_ident = match &field_access.object.node {
            ExpressionKind::Variable { ident, .. } => ident,
            _ => return false,
        };

        let Some((enum_def, _)) = self.store.find_enum_by_name(object_ident.as_str()) else {
            return false;
        };

        if !enum_def
            .variants
            .iter()
            .any(|v| v.as_str() == field_access.field.as_str())
        {
            self.log_error(SoulError::new(
                format!(
                    "variant '{}' not found in enum '{}'",
                    field_access.field.as_str(),
                    object_ident.as_str()
                ),
                SoulErrorKind::NotFoundInScope,
                Some(field_access.field.span),
            ));
            return true;
        }

        field_access.id = Some(enum_def.id.unwrap());
        field_access.is_enum_variant = true;

        true
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

    fn resolve_match_pattern(&mut self, pattern: &ast::MatchPattern) {
        if let ast::MatchPattern::Array(elements) = pattern {
            for elem in elements {
                self.resolve_match_pattern(elem);
            }
        }
    }
}
