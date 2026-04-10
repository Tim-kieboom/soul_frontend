use ast::{Expression, ExpressionKind, FunctionKind, SoulType, TypeKind};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdAlloc},
    soul_names::PrimitiveTypes,
};

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
            ExpressionKind::FieldAccess(_) => (),
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
                let owner = parse_owner_type(self, function_call.callee.as_ref());
                let is_owner_qualifier = owner.is_some();
                if is_owner_qualifier {
                    // Type-qualified static call: `TypeName.fn(...)` or `int.fn(...)`.
                    function_call.callee = None;
                } else if let Some(callee) = &mut function_call.callee {
                    self.resolve_expression(callee);
                }

                function_call.resolved = self.store.find_function_by_name_and_owner_kind(
                    function_call.name.as_str(),
                    owner.as_ref().map(|el| &el.kind),
                );
                if function_call.resolved.is_none() && owner.is_some() {
                    // Fallback: keep older behavior if declaration uses a different owner encoding.
                    function_call.resolved = self.lookup_function(&function_call.name);
                }

                if function_call.resolved.is_none() {
                    self.log_error(SoulError::new(
                        format!(
                            "function '{}' is undefined in scope",
                            function_call.name.as_str(),
                        ),
                        SoulErrorKind::NotFoundInScope,
                        Some(span),
                    ));

                    function_call.resolved = Some(FunctionId::error());
                };

                if let Some(function_id) = function_call.resolved {
                    if let Some(signature) = self.store.get_function(function_id) {
                        let needs_callee = !matches!(signature.function_kind, FunctionKind::Static);

                        let has_callee = function_call.callee.is_some();
                        if has_callee && !needs_callee {
                            self.log_error(SoulError::new(
                                format!(
                                    "function '{}' is static and can not be called on an instance",
                                    function_call.name.as_str(),
                                ),
                                SoulErrorKind::InvalidContext,
                                Some(span),
                            ));
                        } else if !has_callee && needs_callee {
                            self.log_error(SoulError::new(
                                format!(
                                    "method '{}' requires a receiver (this/@this/&this)",
                                    function_call.name.as_str(),
                                ),
                                SoulErrorKind::InvalidContext,
                                Some(span),
                            ));
                        }
                    }
                }

                for arg in &mut function_call.arguments {
                    self.resolve_expression(&mut arg.value);
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

fn parse_owner_type(
    resolver: &mut NameResolver<'_>,
    callee: Option<&Box<Expression>>,
) -> Option<SoulType> {
    let Some(callee) = callee else {
        return None;
    };

    match &callee.node {
        ExpressionKind::Variable { ident, .. } => {
            if let Some(primitive) = PrimitiveTypes::from_str(ident.as_str()) {
                return Some(SoulType::new(None, TypeKind::Primitive(primitive), ident.span));
            }

            if resolver.info.scopes.lookup_type(ident).is_some() {
                return Some(SoulType::new(
                    None,
                    TypeKind::Stub(ast::Stub {
                        name: ident.as_str().to_string(),
                        generics: vec![],
                    }),
                    ident.span,
                ));
            }

            None
        }
        _ => None,
    }
}
