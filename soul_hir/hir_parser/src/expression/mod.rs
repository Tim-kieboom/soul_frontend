use ast::{AsTypeCast, VarTypeKind, scope::NodeId};
use hir::{ExpressionId, HirType, HirTypeKind, LocalId, Place, PlaceKind};
#[cfg(debug_assertions)]
use soul_utils::soul_error_internal;
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::IdAlloc,
    span::Span,
};

use crate::HirContext;

mod array;
mod call;
mod r#if;

impl<'a> HirContext<'a> {
    pub(crate) fn lower_expression(&mut self, expression: &ast::Expression) -> hir::ExpressionId {
        let span = expression.span;
        let id = self.alloc_expression(span);

        let value = match &expression.node {
            ast::ExpressionKind::If(ast_if) => self.lower_if(id, ast_if),
            ast::ExpressionKind::Unary(unary) => self.lower_unary(id, unary),
            ast::ExpressionKind::Array(array) => self.lower_array(id, array, span),
            ast::ExpressionKind::Block(block) => return self.lower_block_expression(block),
            ast::ExpressionKind::Index(index) => self.lower_index(id, index, span),
            ast::ExpressionKind::Null(_node_id) => self.lower_null(id),
            ast::ExpressionKind::Binary(binary) => self.lower_binary(id, binary),
            ast::ExpressionKind::While(ast_while) => self.lower_while(id, ast_while),
            ast::ExpressionKind::As(as_type_cast) => self.lower_cast(id, as_type_cast),
            ast::ExpressionKind::Deref { id: _, inner } => self.lower_deref(id, inner),
            ast::ExpressionKind::FieldAccess(field_access) => {
                self.lower_field(id, field_access, span)
            }
            ast::ExpressionKind::FunctionCall(function_call) => self.lower_call(id, function_call),
            ast::ExpressionKind::Literal((_node_id, literal)) => self.lower_literal(id, literal),
            ast::ExpressionKind::Variable {
                id: _,
                ident,
                resolved,
            } => self.lower_expression_variable(id, ident, *resolved),
            ast::ExpressionKind::Ref {
                id: _,
                is_mutable,
                expression,
            } => self.lower_ref(id, expression, is_mutable, span),
            ast::ExpressionKind::StructConstructor(struct_constructor) => {
                self.lower_struct_contructor(id, struct_constructor, span)
            }

            ast::ExpressionKind::ExternalExpression(_external_expression) => {
                self.log_error(soul_error_internal!(
                    "ExternalExpression expression is unstable",
                    Some(span)
                ));
                hir::Expression::error(id)
            }
            ast::ExpressionKind::Default(_node_id) => {
                self.log_error(soul_error_internal!(
                    "Default expression is unstable",
                    Some(span)
                ));
                hir::Expression::error(id)
            }
            ast::ExpressionKind::ReturnLike(_) => {
                self.log_error(soul_error_internal!(
                    "return like should be unreachable in HirContext::lower_expression",
                    Some(span)
                ));
                hir::Expression::error(id)
            }
        };

        self.insert_expression(id, value)
    }

    fn lower_struct_contructor(
        &mut self,
        id: ExpressionId,
        ctor: &ast::StructConstructor,
        span: Span,
    ) -> hir::Expression {
        let ty = self.lower_type(&ctor.struct_type);
        let kown = match ty {
            hir::PossibleTypeId::Known(type_id) => type_id,
            hir::PossibleTypeId::Infer(_) => {
                self.log_error(SoulError::new(
                    "struct type should be known at this point",
                    SoulErrorKind::TypeInferenceError,
                    Some(span),
                ));

                return hir::Expression::error(id);
            }
        };

        let hir_type = self.tree.info.types.id_to_type(kown).expect("have type");
        let struct_type = match &hir_type.kind {
            HirTypeKind::Struct(val) => *val,
            _ => {
                self.log_error(SoulError::new(
                    "should be struct type",
                    SoulErrorKind::InvalidContext,
                    Some(span),
                ));
                return hir::Expression::error(id);
            }
        };

        let values = ctor
            .values
            .iter()
            .map(|(name, value)| (name.clone(), self.lower_expression(value)))
            .collect();

        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::StructConstructor {
                ty: struct_type,
                values,
                defaults: ctor.defaults,
            },
        }
    }

    fn lower_ref(
        &mut self,
        id: hir::ExpressionId,
        expression: &ast::Expression,
        is_mutable: &bool,
        span: Span,
    ) -> hir::Expression {
        let inner = self.lower_expression(expression);
        let of_type = self.tree.nodes.expressions[inner].ty;

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
                    is_temp: false,
                    local: temp_local,
                    value: Some(inner),
                };
                self.insert_desugar_variable(variable, span);
                temp_local
            }
        };

        let place = Place::new(
            self.id_generator.alloc_place(),
            PlaceKind::Local(local),
            span,
        );

        let ty = self.add_type(HirType::new(HirTypeKind::Ref {
            of_type,
            mutable: *is_mutable,
        }));

        hir::Expression {
            id,
            ty: hir::PossibleTypeId::Known(ty),
            kind: hir::ExpressionKind::Ref {
                place: self.insert_place(place),
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
            None => self.new_infer_type(vec![], None),
            Some(VarTypeKind::NonInveredType(ty)) => self.lower_type(ty),
            Some(VarTypeKind::InveredType(modifier)) => {
                let modifier = *modifier;
                self.new_infer_type(vec![], Some(modifier))
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

        let place_id = self.id_generator.alloc_place();
        let place_kind = match self.tree.nodes.locals.get(local) {
            Some(local_info) if local_info.is_temp() => PlaceKind::Temp(local),
            _ => PlaceKind::Local(local),
        };

        let place = Place::new(place_id, place_kind, ident.span);

        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Load(self.insert_place(place)),
        }
    }

    fn lower_field(
        &mut self,
        id: ExpressionId,
        field: &ast::FieldAccess,
        span: Span,
    ) -> hir::Expression {
        let base = self.lower_place(&field.object);
        let field = hir::PlaceKind::Field {
            base,
            field: field.field.clone(),
        };
        let place_id = self.id_generator.alloc_place();
        let place = self.insert_place(hir::Place::new(place_id, field, span));

        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None),
            kind: hir::ExpressionKind::Load(place),
        }
    }

    fn lower_deref(&mut self, id: ExpressionId, inner: &Box<ast::Expression>) -> hir::Expression {
        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None),
            kind: hir::ExpressionKind::DeRef(self.lower_expression(inner)),
        }
    }

    fn lower_cast(&mut self, id: ExpressionId, cast: &Box<AsTypeCast>) -> hir::Expression {
        let value = self.lower_expression(&cast.left);
        let cast_to = self.lower_type(&cast.type_cast);
        hir::Expression {
            id,
            ty: cast_to,
            kind: hir::ExpressionKind::Cast { value, cast_to },
        }
    }

    fn lower_while(&mut self, id: ExpressionId, ast_while: &ast::While) -> hir::Expression {
        let condition = ast_while
            .condition
            .as_ref()
            .map(|value| self.lower_expression(value));

        let body = self.lower_block(&ast_while.block);
        hir::Expression {
            id,
            ty: hir::PossibleTypeId::Known(self.add_type(HirType::none_type())),
            kind: hir::ExpressionKind::While { condition, body },
        }
    }

    fn lower_literal(&mut self, id: ExpressionId, literal: &ast::Literal) -> hir::Expression {
        hir::Expression {
            id,
            ty: hir::PossibleTypeId::Known(self.type_from_literal(literal)),
            kind: hir::ExpressionKind::Literal(literal.clone()),
        }
    }

    fn lower_binary(&mut self, id: ExpressionId, binary: &ast::Binary) -> hir::Expression {
        let left = self.lower_expression(&binary.left);
        let operator = binary.operator.clone();
        let right = self.lower_expression(&binary.right);
        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None),
            kind: hir::ExpressionKind::Binary(hir::Binary {
                left,
                operator,
                right,
            }),
        }
    }

    fn lower_null(&mut self, id: ExpressionId) -> hir::Expression {
        hir::Expression {
            id,
            ty: self.new_null_infer(),
            kind: hir::ExpressionKind::Null,
        }
    }

    fn lower_index(&mut self, id: ExpressionId, index: &ast::Index, span: Span) -> hir::Expression {
        let place = Place::new(
            self.id_generator.alloc_place(),
            PlaceKind::Index {
                base: self.lower_place(&index.collection),
                index: self.lower_expression(&index.index),
            },
            span,
        );

        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None),
            kind: hir::ExpressionKind::Load(self.insert_place(place)),
        }
    }

    fn lower_block_expression(&mut self, block: &ast::Block) -> hir::ExpressionId {
        let body = self.lower_block(block);

        let ty = match &self.tree.nodes.blocks[body].terminator {
            Some(value) => self.tree.nodes.expressions[*value].ty,
            None => hir::PossibleTypeId::Known(self.add_type(HirType::none_type())),
        };

        let id = self.alloc_expression(block.span);
        let return_value = hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Block(body),
        };

        self.insert_expression(id, return_value)
    }

    fn lower_unary(&mut self, id: ExpressionId, unary: &ast::Unary) -> hir::Expression {
        let expression = self.lower_expression(&unary.expression);
        let operator = unary.operator.clone();
        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None),
            kind: hir::ExpressionKind::Unary(hir::Unary {
                operator,
                expression,
            }),
        }
    }

    pub(crate) fn insert_expression(
        &mut self,
        id: ExpressionId,
        expression: hir::Expression,
    ) -> ExpressionId {
        self.tree.nodes.expressions.insert(id, expression);
        id
    }
}
