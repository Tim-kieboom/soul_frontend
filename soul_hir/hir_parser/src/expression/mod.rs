use ast::{ArrayContructor, ArrayKind, FieldAccess, Literal};
use ast::{AsTypeCast, VarTypeKind, scope::NodeId};
use hir::{
    CustomTypeId, EnumId, ExpressionId, HirType, HirTypeKind, LocalId, Place, PlaceKind, Terminator,
};
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
mod r#match;

impl<'a> HirContext<'a> {
    pub(crate) fn lower_expression(&mut self, expression: &ast::Expression) -> hir::ExpressionId {
        let span = expression.span;
        let id = self.alloc_expression(span);

        let value = match &expression.node {
            ast::ExpressionKind::Sizeof(ty) => {
                let ty = self.lower_type(ty, span);
                hir::Expression {
                    id,
                    ty,
                    kind: hir::ExpressionKind::Sizeof(ty),
                }
            }
            ast::ExpressionKind::ArrayContructor(ctor) => {
                self.desugar_array_contructor(id, ctor, span)
            }
            ast::ExpressionKind::If(ast_if) => self.lower_if(id, ast_if, span),
            ast::ExpressionKind::Unary(unary) => self.lower_unary(id, unary, span),
            ast::ExpressionKind::Array(array) => self.lower_array(id, array, span),
            ast::ExpressionKind::Block(block) => return self.lower_block_expression(block),
            ast::ExpressionKind::Index(index) => self.lower_index(id, index, span),
            ast::ExpressionKind::Null(_node_id) => self.lower_null(id, span),
            ast::ExpressionKind::Binary(binary) => self.lower_binary(id, binary, span),
            ast::ExpressionKind::While(ast_while) => self.lower_while(id, ast_while),
            ast::ExpressionKind::Match(ast_match) => self.lower_match(id, ast_match, span),
            ast::ExpressionKind::As(as_type_cast) => self.lower_cast(id, as_type_cast),
            ast::ExpressionKind::Deref { id: _, inner } => self.lower_deref(id, inner),
            ast::ExpressionKind::FieldAccess(field_access) => {
                self.lower_field_access(id, field_access, span)
            }
            ast::ExpressionKind::FunctionCall(function_call) => {
                match self.try_lower_union_constructor(id, function_call) {
                    Some(expr) => expr,
                    None => self.lower_call(id, function_call),
                }
            }
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
            ast::ExpressionKind::New(expr) => self.lower_new(id, expr, span),
            ast::ExpressionKind::NewArray(any_array) => self.lower_new_array(id, any_array, span),

            ast::ExpressionKind::ExternalExpression(_) => {
                self.log_error(soul_error_internal!(
                    "ExternalExpression expression is unstable",
                    Some(span)
                ));
                hir::Expression::error(id)
            }
            ast::ExpressionKind::Default(_) => {
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

    fn lower_field_access(
        &mut self,
        id: hir::ExpressionId,
        field_access: &FieldAccess,
        span: Span,
    ) -> hir::Expression {
        if field_access.is_enum_variant {
            let variant_name = field_access.field.clone();
            let enum_id = self
                .tree
                .info
                .types
                .enums_entries()
                .find(|(_, e)| {
                    e.variants
                        .iter()
                        .any(|v| v.as_str() == variant_name.as_str())
                })
                .map(|(id, _)| id)
                .unwrap_or_else(|| EnumId::error());

            if enum_id == EnumId::error() {
                self.log_error(SoulError::new(
                    "Enum not found",
                    SoulErrorKind::NotFoundInScope,
                    Some(span),
                ));
            }

            let variant = hir::Expression {
                id,
                ty: self.new_infer_type(vec![], None, span),
                kind: hir::ExpressionKind::EnumVariant {
                    enum_id,
                    variant_name,
                },
            };

            return variant;
        }

        let place = self.lower_field(field_access, span);
        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, span),
            kind: hir::ExpressionKind::Load(place),
        }
    }

    fn try_lower_union_constructor(
        &mut self,
        id: hir::ExpressionId,
        function_call: &ast::FunctionCall,
    ) -> Option<hir::Expression> {
        let callee = function_call.callee.as_ref()?;

        let type_name = match &callee.node {
            ast::ExpressionKind::Variable { ident, .. } => ident.clone(),
            _ => return None,
        };

        let union_id = {
            let mut found = None;
            for store in self.scopes.iter().rev() {
                if let Some(CustomTypeId::Union(id)) = store.custom_types.get(type_name.as_str()) {
                    found = Some(*id);
                    break;
                }
            }
            found?
        };

        let (variant_index, variant_field_id) = {
            let union_def = self.tree.info.types.id_to_union(union_id)?;
            let variant_index = union_def
                .variants
                .iter()
                .position(|v| v.name.as_str() == function_call.name.as_str())?;
            let variant_field_id = union_def.variants[variant_index].id;
            (variant_index, variant_field_id)
        };

        let value = match function_call.arguments.first() {
            Some(arg) => self.lower_expression(&arg.value),
            None => {
                let err_id = self.alloc_expression(function_call.name.span);
                self.insert_expression(err_id, hir::Expression::error(err_id))
            }
        };

        let union_hir_type = HirType::new(HirTypeKind::CustomType(CustomTypeId::Union(union_id)));
        let ty = self.add_type(union_hir_type).to_lazy();

        Some(hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::UnionConstructor {
                union_id,
                variant_index,
                variant_field_id,
                value,
            },
        })
    }

    fn desugar_array_contructor(
        &mut self,
        id: ExpressionId,
        ctor: &ArrayContructor,
        span: Span,
    ) -> hir::Expression {
        let amount = match &ctor.amount.node {
            ast::ExpressionKind::Literal((_, literal)) => match literal {
                Literal::Uint(num) => *num,
                _ => {
                    self.log_error(SoulError::new(
                        "expression needs to be a uint literal (so no negative and no decimal)",
                        SoulErrorKind::InvalidContext,
                        Some(ctor.amount.span),
                    ));
                    return hir::Expression::error(id);
                }
            },
            _ => {
                self.log_error(SoulError::new(
                    "expression should be a literal",
                    SoulErrorKind::NeedsToBeLiteralError,
                    Some(ctor.amount.span),
                ));
                return hir::Expression::error(id);
            }
        };

        let mut values = Vec::with_capacity(amount as usize);
        for _ in 0..amount {
            values.push(ctor.element.as_ref().clone());
        }

        let literal_array = ast::Array {
            values,
            id: ctor.id,
            element_type: ctor.element_type.clone(),
            collection_type: ctor.collection_type.clone(),
        };

        self.lower_array(id, &literal_array, span)
    }

    fn lower_new(
        &mut self,
        id: ExpressionId,
        expr: &ast::Expression,
        _span: Span,
    ) -> hir::Expression {
        let inner = self.lower_expression(expr);
        let inner_ty = self.tree.nodes.expressions[inner].ty;
        let ty = self.add_type(HirType::new(HirTypeKind::Pointer(inner_ty)));
        hir::Expression {
            id,
            ty: hir::LazyTypeId::Known(ty),
            kind: hir::ExpressionKind::New(inner),
        }
    }

    fn lower_new_array(
        &mut self,
        id: ExpressionId,
        any_array: &ast::AnyArray,
        _span: Span,
    ) -> hir::Expression {
        let (values, element_ty) = match any_array {
            ast::AnyArray::ArrayLiteral(arr) => {
                let values: Vec<_> = arr
                    .values
                    .iter()
                    .map(|v| self.lower_expression(v))
                    .collect();
                let element_ty = values
                    .first()
                    .map(|v| self.tree.nodes.expressions[*v].ty)
                    .unwrap_or(hir::LazyTypeId::error());
                (values, element_ty)
            }
            ast::AnyArray::ArrayConstructor(ctor) => {
                let amount = match &ctor.amount.node {
                    ast::ExpressionKind::Literal((_, Literal::Uint(num))) => *num,
                    _ => {
                        self.log_error(SoulError::new(
                            "expression should be a uint literal",
                            SoulErrorKind::NeedsToBeLiteralError,
                            Some(ctor.amount.span),
                        ));
                        return hir::Expression::error(id);
                    }
                };
                let element = self.lower_expression(&ctor.element);
                let element_ty = self.tree.nodes.expressions[element].ty;
                let values = vec![element; amount as usize];
                (values, element_ty)
            }
        };
        let ty = self.add_type(HirType::new(HirTypeKind::Array {
            element: element_ty,
            kind: ArrayKind::HeapArray,
        }));

        let ptr_ty = self.add_type(HirType::new(HirTypeKind::Pointer(element_ty)));
        hir::Expression {
            id,
            ty: hir::LazyTypeId::Known(ty),
            kind: hir::ExpressionKind::NewArray {
                values,
                ptr_type: ptr_ty,
            },
        }
    }

    fn lower_struct_contructor(
        &mut self,
        id: ExpressionId,
        ctor: &ast::StructConstructor,
        span: Span,
    ) -> hir::Expression {
        let ty = self.lower_type(&ctor.struct_type, span);
        let kown = match ty {
            hir::LazyTypeId::Known(type_id) => type_id,
            hir::LazyTypeId::Infer(_) => {
                self.log_error(SoulError::new(
                    "struct type should be known at this point",
                    SoulErrorKind::TypeInferenceError,
                    Some(span),
                ));

                return hir::Expression::error(id);
            }
        };

        let hir_type = match self.tree.info.types.id_to_type(kown) {
            Some(val) => val,
            None => return hir::Expression::error(id),
        };

        let struct_type = match &hir_type.kind {
            HirTypeKind::CustomType(CustomTypeId::Struct(val)) => *val,
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
        // `&*expr` / `@*expr`: use a PlaceKind::Deref directly instead of
        // eagerly loading the deref into a temp and taking its address,
        // which would lose the original pointer identity.
        if let ast::ExpressionKind::Deref { inner, .. } = &expression.node {
            return self.cast_pointer_to_ref(inner, id, *is_mutable, span);
        }

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

                let variable = hir::Variable { local: temp_local };
                self.insert_desugar_variable(variable, of_type, inner, span);
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
            ty: hir::LazyTypeId::Known(ty),
            kind: hir::ExpressionKind::Ref {
                place: self.insert_place(place),
                mutable: *is_mutable,
            },
        }
    }

    /// Special case: `@*expr` or `&*expr` — create a PlaceKind::Deref directly
    /// instead of eagerly loading the deref into a temp and taking
    /// the address of that temp (which loses the original pointer).
    fn cast_pointer_to_ref(
        &mut self,
        deref_inner: &ast::Expression,
        id: ExpressionId,
        is_mutable: bool,
        span: Span,
    ) -> hir::Expression {
        let inner_expr = self.lower_expression(deref_inner);
        let inner_ty = self.tree.nodes.expressions[inner_expr].ty;

        let inner_place_kind = match &deref_inner.node {
            ast::ExpressionKind::Variable { ident, .. } => match self.find_local(&ident) {
                Some(local) => PlaceKind::Local(local),
                None => {
                    self.log_error(SoulError::new(
                        format!("'{}' not found in scope", ident.as_str()),
                        SoulErrorKind::NotFoundInScope,
                        Some(ident.span),
                    ));
                    PlaceKind::Local(LocalId::error())
                }
            },
            _ => {
                let temp_local = self.id_generator.alloc_local();
                let variable = hir::Variable { local: temp_local };
                self.insert_desugar_variable(variable, inner_ty, inner_expr, span);
                PlaceKind::Local(temp_local)
            }
        };

        let inner_place = Place::new(self.id_generator.alloc_place(), inner_place_kind, span);
        let inner_place_id = self.insert_place(inner_place);

        let deref_place_id = self.id_generator.alloc_place();
        let deref_place = Place::new(deref_place_id, PlaceKind::Deref(inner_place_id), span);
        let deref_place_id = self.insert_place(deref_place);

        let of_type = self.new_infer_type(vec![], None, span);
        let ty = self.add_type(HirType::new(HirTypeKind::Ref {
            of_type,
            mutable: is_mutable,
        }));

        hir::Expression {
            id,
            ty: hir::LazyTypeId::Known(ty),
            kind: hir::ExpressionKind::Ref {
                place: deref_place_id,
                mutable: is_mutable,
            },
        }
    }

    fn lower_expression_variable(
        &mut self,
        id: hir::ExpressionId,
        ident: &Ident,
        option_id: Option<NodeId>,
    ) -> hir::Expression {
        let node_id = match option_id {
            Some(val) => val,
            None => {
                return hir::Expression::error(id);
            }
        };

        let var_type_kind = self
            .ast_context
            .store
            .get_variable_type(node_id)
            .map(|(var, _)| var);

        let ty = match var_type_kind {
            None => self.new_infer_type(vec![], None, ident.span),
            Some(VarTypeKind::NonInveredType(ty)) => self.lower_type(ty, ident.span),
            Some(VarTypeKind::InveredType(modifier)) => {
                let modifier = *modifier;
                self.new_infer_type(vec![], Some(modifier), ident.span)
            }
        };

        let local = match self.find_local_by_node_id(node_id) {
            Some(val) => val,
            None => match self.find_local(ident) {
                Some(val) => val,
                None => {
                    #[cfg(debug_assertions)]
                    self.log_error(soul_error_internal!(
                        format!("local('{}') not found", ident.as_str()),
                        Some(ident.span)
                    ));

                    LocalId::error()
                }
            },
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

    pub(crate) fn lower_field(&mut self, field: &ast::FieldAccess, span: Span) -> hir::PlaceId {
        if let Some(node_id) = field.id {
            if let Some(local_id) = self.find_local_by_node_id(node_id) {
                let place = hir::Place::new(
                    self.id_generator.alloc_place(),
                    hir::PlaceKind::Local(local_id),
                    span,
                );
                return self.insert_place(place);
            }
        }

        let base = self.lower_place(&field.object);
        let field = hir::PlaceKind::Field {
            base,
            field: field.field.clone(),
        };
        let place_id = self.id_generator.alloc_place();
        self.insert_place(hir::Place::new(place_id, field, span))
    }

    fn lower_deref(&mut self, id: ExpressionId, inner: &ast::Expression) -> hir::Expression {
        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, inner.span),
            kind: hir::ExpressionKind::DeRef(self.lower_expression(inner)),
        }
    }

    fn lower_cast(&mut self, id: ExpressionId, cast: &AsTypeCast) -> hir::Expression {
        let value = self.lower_expression(&cast.left);
        let cast_to = self.lower_type(&cast.type_cast, cast.left.span);
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
            ty: hir::LazyTypeId::Known(self.add_type(HirType::none_type())),
            kind: hir::ExpressionKind::While { condition, body },
        }
    }

    fn lower_literal(&mut self, id: ExpressionId, literal: &ast::Literal) -> hir::Expression {
        hir::Expression {
            id,
            ty: hir::LazyTypeId::Known(self.type_from_literal(literal)),
            kind: hir::ExpressionKind::Literal(literal.clone()),
        }
    }

    fn lower_binary(
        &mut self,
        id: ExpressionId,
        binary: &ast::Binary,
        span: Span,
    ) -> hir::Expression {
        let left = self.lower_expression(&binary.left);
        let operator = binary.operator.clone();
        let right = self.lower_expression(&binary.right);
        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, span),
            kind: hir::ExpressionKind::Binary(hir::Binary {
                left,
                operator,
                right,
            }),
        }
    }

    fn lower_null(&mut self, id: ExpressionId, span: Span) -> hir::Expression {
        hir::Expression {
            id,
            ty: self.new_null_infer(span),
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
            ty: self.new_infer_type(vec![], None, span),
            kind: hir::ExpressionKind::Load(self.insert_place(place)),
        }
    }

    fn lower_block_expression(&mut self, block: &ast::Block) -> hir::ExpressionId {
        let body = self.lower_block(block);

        let ty = match &self.tree.nodes.blocks[body].terminator {
            Some(Terminator::Return(value)) | Some(Terminator::Expression(value)) => {
                self.tree.nodes.expressions[*value].ty
            }
            None => hir::LazyTypeId::Known(self.add_type(HirType::none_type())),
        };

        let id = self.alloc_expression(block.span);
        let return_value = hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Block(body),
        };

        self.insert_expression(id, return_value)
    }

    fn lower_unary(&mut self, id: ExpressionId, unary: &ast::Unary, span: Span) -> hir::Expression {
        let expression = self.lower_expression(&unary.expression);
        let operator = unary.operator.clone();
        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, span),
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
