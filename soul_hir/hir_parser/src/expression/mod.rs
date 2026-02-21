use ast::{VarTypeKind, scope::NodeId};
use hir::{FunctionId, HirType, HirTypeKind, IdAlloc, LocalId, Place, PlaceKind, TypeId};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    soul_error_internal,
    span::Span,
};

use crate::HirContext;
mod array;
mod r#if;

impl<'a> HirContext<'a> {
    pub fn lower_expression(&mut self, expression: &ast::Expression) -> hir::ExpressionId {
        let id = self.alloc_expression(expression.span);

        let span = expression.span;
        let hir_expression = match &expression.node {
            ast::ExpressionKind::Null(_node_id) => hir::Expression {
                id,
                ty: self.null_ty(span),
                kind: hir::ExpressionKind::Null,
            },
            ast::ExpressionKind::Literal((_id, literal)) => hir::Expression {
                id,
                ty: self.type_from_literal(literal),
                kind: hir::ExpressionKind::Literal(literal.clone()),
            },
            ast::ExpressionKind::Index(index) => {
                let place = Place::new(
                    PlaceKind::Index {
                        base: Box::new(self.lower_place(&index.collection)),
                        index: self.lower_expression(&index.index),
                    },
                    span,
                );

                hir::Expression {
                    id,
                    ty: self.new_infer_type(span),
                    kind: hir::ExpressionKind::Load(place),
                }
            }
            ast::ExpressionKind::FunctionCall(function_call) => self.lower_call(id, function_call),
            ast::ExpressionKind::Variable {
                ident,
                resolved: _,
                id: option_id,
            } => self.lower_expression_variable(id, ident, *option_id),
            ast::ExpressionKind::If(r#if) => self.lower_if(id, r#if, span),
            ast::ExpressionKind::As(cast) => {
                let value = self.lower_expression(&cast.left);
                let cast_to = self.lower_type(&cast.type_cast);
                hir::Expression {
                    id,
                    ty: cast_to,
                    kind: hir::ExpressionKind::Cast { value, cast_to },
                }
            }
            ast::ExpressionKind::Unary(unary) => {
                let expression = self.lower_expression(&unary.expression);
                let operator = unary.operator.clone();
                hir::Expression {
                    id,
                    ty: self.new_infer_type(span),
                    kind: hir::ExpressionKind::Unary {
                        operator,
                        expression,
                    },
                }
            }
            ast::ExpressionKind::Array(array) => self.lower_array(id, array, span),
            ast::ExpressionKind::Block(block) => return self.lower_block_expression(block),
            ast::ExpressionKind::While(r#while) => {
                let condition = r#while
                    .condition
                    .as_ref()
                    .map(|value| self.lower_expression(value));

                let body = self.lower_block(&r#while.block);
                hir::Expression {
                    id,
                    ty: self.add_type(HirType::none_type()),
                    kind: hir::ExpressionKind::While { condition, body },
                }
            }
            ast::ExpressionKind::Binary(binary) => {
                let left = self.lower_expression(&binary.left);
                let operator = binary.operator.clone();
                let right = self.lower_expression(&binary.right);
                hir::Expression {
                    id,
                    ty: self.new_infer_type(span),
                    kind: hir::ExpressionKind::Binary {
                        left,
                        operator,
                        right,
                    },
                }
            }
            ast::ExpressionKind::Deref { id: _, inner } => hir::Expression {
                id,
                ty: self.new_infer_type(span),
                kind: hir::ExpressionKind::DeRef(self.lower_expression(inner)),
            },
            ast::ExpressionKind::Ref {
                id: _,
                is_mutable,
                expression,
            } => self.lower_ref(id, expression, is_mutable, span),
            ast::ExpressionKind::Default(_) => {
                todo!("desugar Default")
            }
            ast::ExpressionKind::ExternalExpression(external) => {
                let _module_id = match self.hir.imports.get_id(&external.path) {
                    Some(id) => id,
                    None => self
                        .hir
                        .imports
                        .insert(&mut self.id_generator.module, external.path.clone()),
                };

                todo!("impl externalExpression")
            }
            ast::ExpressionKind::ReturnLike(_) => {
                panic!("return_like should be unreachable")
            }
        };

        self.insert_expression(id, hir_expression)
    }

    fn lower_block_expression(&mut self, block: &ast::Block) -> hir::ExpressionId {
        let body = self.lower_block(block);

        let ty = match &self.hir.blocks[body].terminator {
            Some(value) => self.hir.expressions[*value].ty,
            None => self.add_type(HirType::none_type()),
        };

        let id = self.alloc_expression(block.span);
        let return_value = hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Block(body),
        };

        self.insert_expression(id, return_value)
    }

    fn lower_ref(
        &mut self,
        id: hir::ExpressionId,
        expression: &ast::Expression,
        is_mutable: &bool,
        span: Span,
    ) -> hir::Expression {
        let inner = self.lower_expression(expression);
        let of_type = self.hir.expressions[inner].ty;

        let local = match &expression.node {
            ast::ExpressionKind::Variable { ident, .. } => match self.find_local(ident) {
                Some(val) => val,
                None => {
                    self.log_error(SoulError::new(
                        format!("'{}' not found in scope", ident.as_str()),
                        SoulErrorKind::NotFoundInScope,
                        Some(ident.span),
                    ));
                    LocalId::error()
                }
            },
            _ => {
                let temp_local = self.id_generator.alloc_local();

                let variable = hir::Variable {
                    ty: of_type,
                    local: temp_local,
                    value: Some(inner),
                };
                self.insert_desugar_variable(variable, span);
                temp_local
            }
        };

        let place = Place::new(PlaceKind::Local(local), span);
        let ty = self.add_type(HirType::new(HirTypeKind::Ref {
            of_type,
            mutable: *is_mutable,
        }));
        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Ref {
                place,
                mutable: *is_mutable,
            },
        }
    }

    fn lower_expression_variable(
        &mut self,
        id: hir::ExpressionId,
        ident: &Ident,
        option_id: Option<NodeId>,
    ) -> hir::Expression {
        let node_id = option_id.expect("node_id should be Some(_) in hir");
        let var_type_kind = self.ast_store.get_variable_type(node_id);

        let ty = match var_type_kind {
            None => self.new_infer_type(ident.span),
            Some(VarTypeKind::NonInveredType(ty)) => self.lower_type(ty),
            Some(VarTypeKind::InveredType(modifier)) => {
                let modifier = *modifier;
                self.new_infer_with_modifier(modifier, ident.span)
            }
        };

        let local = match self.find_local(ident) {
            Some(val) => val,
            None => {
                #[cfg(debug_assertions)]
                self.log_error(soul_error_internal!(
                    format!("local('{}') not found", ident.as_str()),
                    Some(ident.span)
                ));

                LocalId::error()
            }
        };

        let place = Place::new(PlaceKind::Local(local), ident.span);

        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Load(place),
        }
    }

    fn lower_call(
        &mut self,
        id: hir::ExpressionId,
        function_call: &ast::FunctionCall,
    ) -> hir::Expression {
        let resolved = match function_call.resolved {
            Some(val) => val,
            None => {
                return hir::Expression {
                    id,
                    ty: TypeId::error(),
                    kind: hir::ExpressionKind::Null,
                };
            }
        };

        let ty = match self.ast_store.get_function(resolved) {
            Some(signature) => Self::convert_type(&signature.return_type, &mut self.hir.types),
            None => self.new_infer_type(function_call.name.span),
        };

        let callee = function_call
            .callee
            .as_ref()
            .map(|el| self.lower_expression(el));

        let arguments = function_call
            .arguments
            .iter()
            .map(|el| self.lower_expression(el))
            .collect();

        let function = match self.find_function(&function_call.name) {
            Some(val) => val,
            None => FunctionId::error(),
        };

        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Call {
                callee,
                arguments,
                function,
            },
        }
    }

    fn null_ty(&mut self, span: Span) -> TypeId {
        let infer = self.new_infer_type(span);
        self.add_type(HirType {
            kind: HirTypeKind::Optional(infer),
            modifier: None,
        })
    }
}
