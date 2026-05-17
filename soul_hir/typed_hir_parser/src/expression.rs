use std::collections::HashMap;

use crate::{
    TypedHirContext,
    type_helpers::{TypeHelpers, UnifyPrimitiveCastLazy},
};
use ast::{ArrayKind, BinaryOperator, BinaryOperatorKind, FunctionKind, UnaryOperator};
use hir::{
    Binary, BlockId, DisplayType, ExpressionId, HirType, HirTypeKind, LazyTypeId, PlaceId, Struct,
    TypeId, Unary,
};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind, SoulResult},
    ids::{FunctionId, IdAlloc},
    soul_error_internal,
    soul_names::{PrimitiveTypes, TypeModifier},
    span::Span,
    vec_map::VecMap,
};
impl<'a> TypedHirContext<'a> {
    pub(crate) fn infer_expression(&mut self, expression_id: hir::ExpressionId) -> LazyTypeId {
        if self.expressions.get(expression_id) == Some(&LazyTypeId::error()) {
            return LazyTypeId::error();
        }

        let value = &self.hir.nodes.expressions[expression_id];
        let span = self.expression_span(expression_id);
        let ty = match &value.kind {
            hir::ExpressionKind::Array(array) => self.infer_array(
                array.collection_type,
                array.collection_type,
                &array.values,
                ArrayKind::StackArray(array.values.len() as u64),
                span,
            ),
            hir::ExpressionKind::Sizeof(ty) => {
                self.sizeofs.insert(expression_id, *ty);
                self.add_type(HirType::primitive_type(PrimitiveTypes::Uint32))
                    .to_lazy()
            }
            hir::ExpressionKind::PtrOffset { pointer, offset } => {
                self.infer_ptr_offset(*pointer, *offset, span)
            }
            hir::ExpressionKind::StackArrayIndex { array, index } => {
                self.infer_stack_array_index(*array, *index, span)
            }
            hir::ExpressionKind::Error => LazyTypeId::error(),
            hir::ExpressionKind::Null => self.new_infer(span),
            hir::ExpressionKind::New(value) => {
                let inner_type = self.infer_expression(*value);
                self.add_type(HirType::pointer_type(inner_type)).to_lazy()
            }
            hir::ExpressionKind::NewArray { values, .. } => {
                self.infer_array(None, None, values, ArrayKind::HeapArray, span)
            }
            hir::ExpressionKind::Drop { value } => {
                self.infer_expression(*value);
                self.common_types.none_type.to_lazy()
            }
            hir::ExpressionKind::NewHeapArray { ptr, len } => {
                self.infer_new_heap_array(*ptr, *len, span)
            }
            hir::ExpressionKind::Alloc { size } => {
                self.infer_alloc(*size, span)
            }
            hir::ExpressionKind::Dealloc { ptr } => {
                self.infer_dealloc(*ptr)
            }
            hir::ExpressionKind::Realloc { ptr, size } => {
                self.infer_realloc(*ptr, *size, span)
            }
            hir::ExpressionKind::Exit { exit_code } => {
                let value_type = self.infer_expression(*exit_code);
                self.unify(
                    *exit_code,
                    value_type,
                    self.common_types.c_int_type.to_lazy(),
                    span,
                );
                self.common_types.never_type.to_lazy()
            }
            hir::ExpressionKind::Load(place) => self.infer_place(*place),
            hir::ExpressionKind::Block(body) => self.infer_block_expression(*body),
            hir::ExpressionKind::Local(local) => self.locals[*local],
            hir::ExpressionKind::Literal(_) => value.ty, /*already handled by hir*/
            hir::ExpressionKind::StructConstructor {
                ty: _,
                values,
                defaults: _,
            } => self.infer_struct_constructor(value.ty, values, span),
            hir::ExpressionKind::EnumVariant {
                enum_id,
                variant_name,
            } => self.infer_enum_variant(*enum_id, variant_name, span),
            hir::ExpressionKind::DeRef(inner) => self.infer_deref(*inner, span),
            hir::ExpressionKind::Function(function) => self.functions[*function].to_lazy(),
            hir::ExpressionKind::Ref { place, mutable } => {
                self.infer_ref(*place, *mutable, value.ty, span)
            }
            hir::ExpressionKind::Cast { value, cast_to } => self.infer_cast(*value, *cast_to),
            hir::ExpressionKind::While { condition, body } => {
                self.infer_while(*condition, *body, span)
            }
            hir::ExpressionKind::Match { scrutinee, arms } => {
                self.infer_match(*scrutinee, arms.clone(), span)
            }
            hir::ExpressionKind::Unary(Unary {
                operator,
                expression,
            }) => self.infer_unary(operator, *expression, span),
            hir::ExpressionKind::Binary(Binary {
                left,
                operator,
                right,
            }) => self.infer_binary(*left, operator, *right, span),
            hir::ExpressionKind::Call {
                has_callee,
                function,
                generics,
                arguments,
                ..
            } => self.infer_call(*function, *has_callee, generics, arguments, span),
            hir::ExpressionKind::ExternalCall { .. } => self.infer_external_call(),
            hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
                ends_with_else,
            } => self.infer_if(
                expression_id,
                *condition,
                *then_block,
                *else_block,
                *ends_with_else,
                span,
            ),
        };

        if let LazyTypeId::Infer(id) = value.ty {
            self.infer_table.add_infer_binding(id, ty);
        }

        self.type_expression(expression_id, ty);
        ty
    }

    fn infer_array(
        &mut self,
        collection_type: Option<LazyTypeId>,
        element_type: Option<LazyTypeId>,
        values: &Vec<ExpressionId>,
        array_kind: ArrayKind,
        span: Span,
    ) -> LazyTypeId {
        if collection_type.is_some() {
            self.log_error(soul_error_internal!(
                "collectionType in LiteralArray is unstable",
                Some(span)
            ));
            return LazyTypeId::error();
        }

        let mut element = element_type.unwrap_or(self.new_infer(span));
        for value in values {
            let span = self.expression_span(*value);
            let value_type = self.infer_expression(*value);
            self.unify(*value, element, value_type, span);

            element = self.get_priority_lazy_type(element, value_type);
        }

        element = self
            .resolve_untyped_primitive(element, span)
            .map(|hir_type| self.add_type(hir_type).to_lazy())
            .unwrap_or(element);

        let hir_type = HirType::new(HirTypeKind::Array {
            element,
            kind: array_kind,
        });

        self.add_type(hir_type).to_lazy()
    }

    fn infer_if(
        &mut self,
        if_expression_id: ExpressionId,
        condition: ExpressionId,
        then_block: BlockId,
        else_block: Option<BlockId>,
        ends_with_else: bool,
        if_span: Span,
    ) -> LazyTypeId {
        let condition_span = self.expression_span(condition);
        let condition_type = self.infer_expression(condition);
        _ = self.unify(
            condition,
            self.common_types.bool_type.to_lazy(),
            condition_type,
            condition_span,
        );

        let then_type = self.infer_block_expression(then_block);
        let else_block = match else_block {
            Some(val) => {
                let else_type = self.infer_block_expression(val);
                let else_span = self.block_span(val);
                _ = self.unify(if_expression_id, then_type, else_type, else_span);
                if !ends_with_else && then_type != self.common_types.none_type.to_lazy() {
                    self.log_error(SoulError::new(
                        "'if' should end with an 'else' if you want to return a value",
                        SoulErrorKind::InvalidContext,
                        Some(if_span),
                    ));
                    return LazyTypeId::error();
                }
                Some(else_type)
            }
            None => None,
        };

        match else_block {
            Some(else_type) => self.get_priority_lazy_type(then_type, else_type),
            None => then_type,
        }
    }

    fn infer_call(
        &mut self,
        function_id: FunctionId,
        has_callee: bool,
        generics: &Vec<TypeId>,
        arguments: &Vec<ExpressionId>,
        span: Span,
    ) -> LazyTypeId {
        let function = match self.hir.nodes.functions.get(function_id) {
            Some(val) => val,
            None => return LazyTypeId::error(),
        };

        let mut generic_defines = VecMap::new();
        for (i, generic_id) in function.generics.iter().copied().enumerate() {
            let generic_ty = match generics.get(i) {
                Some(val) => *val,
                None => {
                    let msg = match self.types.id_to_generic(generic_id) {
                        Some(name) => format!("generic {name} is not defined"),
                        None => format!("generic {:?} is not defined", generic_id),
                    };
                    self.log_error(SoulError::new(
                        msg,
                        SoulErrorKind::GenericDefineError,
                        Some(span),
                    ));
                    TypeId::error()
                }
            };
            generic_defines.insert(generic_id, generic_ty);
            self.insert_generic_define(generic_id, generic_ty);
        }

        let unresolved_return_type = self.functions[function_id].to_lazy();
        let return_type = self.resolve_generic(&generic_defines, unresolved_return_type);

        let needs_callee = !matches!(function.kind, FunctionKind::Static);
        if has_callee && !needs_callee {
            self.log_error(SoulError::new(
                format!(
                    "function '{}' is static and can not be called on an instance",
                    function.name.as_str()
                ),
                SoulErrorKind::InvalidContext,
                Some(span),
            ));
        } else if !has_callee && needs_callee {
            self.log_error(SoulError::new(
                format!(
                    "method '{}' requires a receiver (this/@this/&this)",
                    function.name.as_str()
                ),
                SoulErrorKind::InvalidContext,
                Some(span),
            ));
        }

        if function.parameters.len() != arguments.len() {
            self.log_error(SoulError::new(
                format!(
                    "functionCall has {} arguments but expects {} arguments",
                    arguments.len(),
                    function.parameters.len()
                ),
                SoulErrorKind::InvalidContext,
                Some(span),
            ));
            return return_type;
        }

        for (argument, parameter) in arguments.iter().zip(function.parameters.iter()) {
            let ty = self.infer_expression(*argument);
            let span = self.expression_span(*argument);

            let should_be = self.resolve_generic(&generic_defines, parameter.ty);
            self.unify(*argument, should_be, ty, span);
        }

        return_type
    }

    fn infer_external_call(&mut self) -> LazyTypeId {
        // External calls - for now, assume returns unit
        // In a real implementation, look up signature from typed HIR
        self.common_types.none_type.to_lazy()
    }

    fn infer_binary(
        &mut self,
        left: ExpressionId,
        operator: &BinaryOperator,
        right: ExpressionId,
        span: Span,
    ) -> LazyTypeId {
        let left_id = self.infer_expression(left);
        let right_id = self.infer_expression(right);
        let binary_typecheck = to_binary_typecheck(&operator.node);
        match binary_typecheck {
            BinaryTypeCheck::Logical => {
                let bool = self.common_types.bool_type.to_lazy();
                self.unify(left, bool, left_id, span);
                self.unify(right, bool, right_id, span);
                bool
            }
            BinaryTypeCheck::Equal | BinaryTypeCheck::Compare => {
                self.unify(right, left_id, right_id, span);
                self.common_types.bool_type.to_lazy()
            }
            BinaryTypeCheck::Bitwise => {
                self.infer_bitwise_numaric(left, left_id, operator, right, right_id, span)
            }
            BinaryTypeCheck::Numeric => {
                let left_strict = match self.resolve_type_strict(left_id, span) {
                    Some(val) => val,
                    None => return LazyTypeId::error(),
                };
                let right_strict = match self.resolve_type_strict(right_id, span) {
                    Some(val) => val,
                    None => return LazyTypeId::error(),
                };

                if self.id_to_type(left_strict).is_pointer()
                    && self.id_to_type(right_strict).is_non_float_numeric_type()
                {
                    return left_strict.to_lazy();
                }

                self.infer_bitwise_numaric(left, left_id, operator, right, right_id, span)
            }
        }
    }

    fn infer_bitwise_numaric(
        &mut self,
        left: ExpressionId,
        left_id: LazyTypeId,
        operator: &BinaryOperator,
        right: ExpressionId,
        right_id: LazyTypeId,
        span: Span,
    ) -> LazyTypeId {
        self.unify(right, left_id, right_id, span);
        let left_id = match self.resolve_type_strict(left_id, span) {
            Some(val) => val,
            None => return LazyTypeId::error(),
        };
        let right_id = match self.resolve_type_strict(right_id, span) {
            Some(val) => val,
            None => return LazyTypeId::error(),
        };

        if let Err(err) = self.is_correct_binary(left, left_id, operator, right, right_id) {
            self.log_error(err);
            return LazyTypeId::error();
        }

        self.get_priority_type(left_id, right_id).to_lazy()
    }

    fn infer_unary(
        &mut self,
        operator: &UnaryOperator,
        value: ExpressionId,
        span: Span,
    ) -> LazyTypeId {
        use ast::UnaryOperatorKind as Unary;

        match operator.node {
            Unary::Invalid => todo!("should not have invalid"),
            Unary::Neg => {
                let value_type = self.infer_expression(value);
                let value_type = match self.resolve_type_strict(value_type, span) {
                    Some(val) => val,
                    None => return LazyTypeId::error(),
                };

                let ty = self.id_to_type(value_type);
                if !ty.is_float_type() && !ty.is_any_int_type() {
                    self.log_error(SoulError::new(
                        format!(
                            "type is '{}' but can only be used for number types (f32, int, i32, ect..)", 
                            operator.node.as_str()
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                    return LazyTypeId::error();
                }

                value_type.to_lazy()
            }
            Unary::Not => {
                let value_type = self.infer_expression(value);
                let value_type = match self.resolve_type_strict(value_type, span) {
                    Some(val) => val,
                    None => return LazyTypeId::error(),
                };

                let ty = self.id_to_type(value_type);
                if !ty.is_boolean_type() {
                    self.log_error(SoulError::new(
                        format!(
                            "type is '{}' but can only be used for bool type",
                            operator.node.as_str()
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                    return LazyTypeId::error();
                }

                value_type.to_lazy()
            }
            Unary::Increment { .. } | Unary::Decrement { .. } => {
                let value_type = self.infer_expression(value);
                let value_type = match self.resolve_type_strict(value_type, span) {
                    Some(val) => val,
                    None => return LazyTypeId::error(),
                };

                let ty = self.id_to_type(value_type);
                if !ty.is_numeric_type() {
                    self.log_error(SoulError::new(
                        format!(
                            "type is '{}' but can only be used for number types (f32, uint, int, i32, ect..)",
                            operator.node.as_str()
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                    return LazyTypeId::error();
                }

                value_type.to_lazy()
            }
        }
    }

    fn infer_while(
        &mut self,
        condition: Option<ExpressionId>,
        body: BlockId,
        span: Span,
    ) -> LazyTypeId {
        let return_type = self.infer_block_expression(body);

        let condition = match condition {
            Some(val) => val,
            None => return return_type,
        };

        let condition_type = self.infer_expression(condition);

        self.unify(
            condition,
            self.common_types.bool_type.to_lazy(),
            condition_type,
            span,
        );

        if return_type != self.common_types.none_type.to_lazy() {
            self.log_error(SoulError::new(
                "while loops with a condition can not have return type",
                SoulErrorKind::InvalidContext,
                Some(span),
            ));
            return LazyTypeId::error();
        }

        self.common_types.none_type.to_lazy()
    }

    fn infer_match(
        &mut self,
        scrutinee: ExpressionId,
        arms: Vec<hir::MatchArm>,
        span: Span,
    ) -> LazyTypeId {
        let _ = self.infer_expression(scrutinee);

        let mut result_type: Option<LazyTypeId> = None;
        let mut has_wildcard = false;

        for arm in &arms {
            let arm_type = self.infer_block_expression(arm.body);

            match result_type {
                Some(prev) => {
                    let _ = self.unify(ExpressionId::error(), prev, arm_type, span);
                    result_type = Some(self.get_priority_lazy_type(prev, arm_type));
                }
                None => result_type = Some(arm_type),
            }

            match &arm.pattern {
                hir::MatchPatternHir::Wildcard => has_wildcard = true,
                hir::MatchPatternHir::Literal(_) => {}
            }
        }

        let result_type = result_type.unwrap_or(self.common_types.none_type.to_lazy());

        if !has_wildcard {
            self.log_error(SoulError::new(
                "match expression should have a wildcard arm ('_')",
                SoulErrorKind::InvalidContext,
                Some(span),
            ));
        }

        result_type
    }

    fn infer_cast(&mut self, value: ExpressionId, cast_to: LazyTypeId) -> LazyTypeId {
        let span = self.expression_span(value);
        let from = self.infer_expression(value);
        let from = match self.resolve_type_strict(from, span) {
            Some(val) => val,
            None => return LazyTypeId::error(),
        };

        let cast_to = match self.resolve_type_strict(cast_to, span) {
            Some(val) => val,
            None => return LazyTypeId::error(),
        };

        let from_type = self.id_to_type(from);
        let to_type = self.id_to_type(cast_to);
        match from_type.unify_primitive_cast(&self.types, &self.infers, to_type) {
            Ok(()) => (),
            Err(err) => self.log_error(SoulError::new(
                err,
                SoulErrorKind::UnifyTypeError,
                Some(span),
            )),
        }

        cast_to.to_lazy()
    }

    fn infer_ref(
        &mut self,
        place: PlaceId,
        mutable: bool,
        declared_type: LazyTypeId,
        span: Span,
    ) -> LazyTypeId {
        let place_type = self.infer_place(place);

        let decl_of = match self.resolve_type_lazy(declared_type, span) {
            LazyTypeId::Known(resolved_decl) => match &self.id_to_type(resolved_decl).kind {
                HirTypeKind::Ref { of_type, .. } => Some(*of_type),
                _ => None,
            },
            _ => None,
        };

        if let Some(decl_of) = decl_of {
            if let LazyTypeId::Known(resolved_place) = self.resolve_type_lazy(place_type, span) {
                if let HirTypeKind::Ref {
                    of_type: place_of, ..
                } = &self.id_to_type(resolved_place).kind
                {
                    if *place_of == decl_of {
                        return declared_type;
                    }
                }
            }
        }

        let resolved = self.resolve_type_lazy(place_type, span);
        let ty = match resolved {
            LazyTypeId::Known(val) => val,
            LazyTypeId::Infer(infer) => {
                let modifier = self.id_to_infer(infer).modifier;
                return self.create_ref(resolved, mutable, modifier, span);
            }
        };

        let modifier = self.id_to_type(ty).modifier;
        self.create_ref(ty.to_lazy(), mutable, modifier, span)
    }

    fn create_ref(
        &mut self,
        type_id: LazyTypeId,
        mutable: bool,
        inner_modifier: Option<TypeModifier>,
        span: Span,
    ) -> LazyTypeId {
        if mutable && !matches!(inner_modifier, None | Some(TypeModifier::Mut)) {
            let type_str = match type_id {
                LazyTypeId::Known(type_id) => self
                    .id_to_type(type_id)
                    .display(&self.types, &self.hir.info.infers),
                LazyTypeId::Infer(infer) => self
                    .id_to_infer(infer)
                    .display(&self.types, &self.hir.info.infers),
            };

            self.log_error(SoulError::new(
                format!(
                    "can only call mutRef on mutable variables type '{type_str}' is not mutable"
                ),
                SoulErrorKind::InvalidMutability,
                Some(span),
            ));
            return LazyTypeId::error();
        }

        let of_type = self.lazy_id_insure_modifier(type_id, None);

        let ref_type =
            HirType::new(HirTypeKind::Ref { of_type, mutable }).apply_modfier(inner_modifier);

        self.add_type(ref_type).to_lazy()
    }

    fn infer_ptr_offset(
        &mut self,
        pointer: ExpressionId,
        offset: ExpressionId,
        span: Span,
    ) -> LazyTypeId {
        let _ = self.infer_expression(offset);

        let pointer_ty = self.infer_expression(pointer);
        let resolved = match self.resolve_type_strict(pointer_ty, span) {
            Some(val) => val,
            None => return LazyTypeId::error(),
        };

        match self.id_to_type(resolved).kind {
            HirTypeKind::Pointer(_) => pointer_ty,
            _ => {
                self.log_error(SoulError::new(
                    "PtrOffset requires a pointer as the first argument".to_string(),
                    SoulErrorKind::TypeInferenceError,
                    Some(span),
                ));
                LazyTypeId::error()
            }
        }
    }

    fn infer_new_heap_array(
        &mut self,
        ptr: ExpressionId,
        len: ExpressionId,
        span: Span,
    ) -> LazyTypeId {
        let _ = self.infer_expression(len);

        let ptr_ty = self.infer_expression(ptr);
        let resolved = match self.resolve_type_strict(ptr_ty, span) {
            Some(val) => val,
            None => return LazyTypeId::error(),
        };

        match &self.id_to_type(resolved).kind {
            HirTypeKind::Pointer(element) => {
                let element_ty = match self.resolve_type_strict(*element, span) {
                    Some(val) => val,
                    None => return LazyTypeId::error(),
                };
                let array_ty = self.add_type(HirType::new(HirTypeKind::Array {
                    element: LazyTypeId::Known(element_ty),
                    kind: ArrayKind::HeapArray,
                }));
                LazyTypeId::Known(array_ty)
            }
            _ => {
                self.log_error(SoulError::new(
                    "NewHeapArray requires a pointer as the first argument".to_string(),
                    SoulErrorKind::TypeInferenceError,
                    Some(span),
                ));
                LazyTypeId::error()
            }
        }
    }

    fn infer_alloc(
        &mut self,
        size: ExpressionId,
        _span: Span,
    ) -> LazyTypeId {
        let _ = self.infer_expression(size);

        let none_ty = self.common_types.none_type;
        let ptr_none = self.add_type(HirType::pointer_type(LazyTypeId::Known(none_ty)));
        LazyTypeId::Known(ptr_none)
    }

    fn infer_dealloc(
        &mut self,
        ptr: ExpressionId,
    ) -> LazyTypeId {
        let _ = self.infer_expression(ptr);
        self.common_types.none_type.to_lazy()
    }

    fn infer_realloc(
        &mut self,
        ptr: ExpressionId,
        size: ExpressionId,
        _span: Span,
    ) -> LazyTypeId {
        let _ = self.infer_expression(ptr);
        let _ = self.infer_expression(size);

        let none_ty = self.common_types.none_type;
        let ptr_none = self.add_type(HirType::pointer_type(LazyTypeId::Known(none_ty)));
        LazyTypeId::Known(ptr_none)
    }

    fn infer_stack_array_index(
        &mut self,
        array: ExpressionId,
        index: ExpressionId,
        span: Span,
    ) -> LazyTypeId {
        let _ = self.infer_expression(index);

        let array_ty = self.infer_expression(array);
        let resolved = match self.resolve_type_strict(array_ty, span) {
            Some(val) => val,
            None => return LazyTypeId::error(),
        };

        match &self.id_to_type(resolved).kind {
            HirTypeKind::Array { element, .. } => {
                let element_ty = match self.resolve_type_strict(*element, span) {
                    Some(val) => val,
                    None => return LazyTypeId::error(),
                };
                let ptr_type = self.add_type(HirType::new(HirTypeKind::Pointer(
                    LazyTypeId::Known(element_ty),
                )));
                LazyTypeId::Known(ptr_type)
            }
            _ => {
                self.log_error(SoulError::new(
                    "StackArrayIndex requires an array as the first argument".to_string(),
                    SoulErrorKind::TypeInferenceError,
                    Some(span),
                ));
                LazyTypeId::error()
            }
        }
    }

    fn infer_deref(&mut self, inner: ExpressionId, span: Span) -> LazyTypeId {
        let inner = self.infer_expression(inner);
        let ty = match self.resolve_type_strict(inner, span) {
            Some(val) => val,
            None => return LazyTypeId::error(),
        };

        let deref = self
            .id_to_type(ty)
            .try_deref(&self.types, &self.infers, span);
        match deref {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                LazyTypeId::error()
            }
        }
    }

    fn infer_struct_constructor(
        &mut self,
        ty: LazyTypeId,
        values: &Vec<(Ident, ExpressionId)>,
        span: Span,
    ) -> LazyTypeId {
        let mut fields = match self.expect_struct(ty, span) {
            Ok(struct_info) => struct_info
                .fields
                .iter()
                .map(|field| (field.name.to_string(), field.ty))
                .collect::<HashMap<String, LazyTypeId>>(),
            Err(err) => {
                self.log_error(err);
                return LazyTypeId::error();
            }
        };

        for (name, value) in values {
            let field_type = match fields.remove(name.as_str()) {
                Some(val) => val,
                None => {
                    self.log_error(SoulError::new(
                        format!("{} is not a valid field", name.as_str()),
                        SoulErrorKind::InvalidIdent,
                        Some(name.span),
                    ));
                    continue;
                }
            };

            let value_type = self.infer_expression(*value);
            self.unify(*value, field_type, value_type, span);
        }

        for name in fields.keys() {
            self.log_error(SoulError::new(
                format!("missing {} field", name.as_str()),
                SoulErrorKind::InvalidIdent,
                Some(span),
            ));
        }

        ty
    }

    fn infer_enum_variant(
        &mut self,
        enum_id: hir::EnumId,
        variant_name: &Ident,
        _span: Span,
    ) -> LazyTypeId {
        if let Some(enum_def) = self.types.id_to_enum(enum_id) {
            if enum_def
                .variants
                .iter()
                .any(|v| v.as_str() == variant_name.as_str())
            {
                let enum_type =
                    HirType::new(HirTypeKind::CustomType(hir::CustomTypeId::Enum(enum_id)));
                return self.add_type(enum_type).to_lazy();
            }
        }

        self.log_error(SoulError::new(
            format!("variant '{}' not found in enum", variant_name.as_str()),
            SoulErrorKind::NotFoundInScope,
            Some(variant_name.span),
        ));
        LazyTypeId::error()
    }

    fn new_infer(&mut self, span: Span) -> LazyTypeId {
        let new_infer = self.infers.insert_infer(vec![], None, span);
        self.infer_table.alloc(new_infer, span);
        hir::LazyTypeId::Infer(new_infer)
    }

    fn expect_struct(&mut self, ty: LazyTypeId, span: Span) -> SoulResult<&Struct> {
        let ty = match ty {
            LazyTypeId::Known(val) => val,
            LazyTypeId::Infer(_) => {
                return Err(SoulError::new(
                    "type shoul dbe known at this point",
                    SoulErrorKind::TypeInferenceError,
                    Some(span),
                ));
            }
        };

        let hir_type = self.id_to_type(ty);
        match &hir_type.kind {
            HirTypeKind::CustomType(hir::CustomTypeId::Struct(id)) => self
                .types
                .id_to_struct(*id)
                .ok_or(soul_error_internal!(format!("{:?} not found", id), None)),
            _ => Err(SoulError::new(
                format!(
                    "{} should be a struct to use StructContructor",
                    hir_type.display(&self.types, &self.infers)
                ),
                SoulErrorKind::UnifyTypeError,
                Some(span),
            )),
        }
    }

    fn is_correct_binary(
        &mut self,
        left: ExpressionId,
        left_id: TypeId,
        operator: &BinaryOperator,
        right: ExpressionId,
        right_id: TypeId,
    ) -> SoulResult<()> {
        let err_wrong_type = |ty: &hir::InnerType<HirTypeKind>, value: ExpressionId| -> SoulError {
            SoulError::new(
                format!(
                    "type is '{}' but operator '{}' only allows number types (f32, uint, int, i32, ect..)",
                    ty.display(&self.types, &self.infers),
                    operator.node.as_str(),
                ),
                SoulErrorKind::UnifyTypeError,
                Some(self.expression_span(value)),
            )
        };

        let left_type = self.id_to_type(left_id);
        let right_type = self.id_to_type(right_id);
        if !left_type.is_error() && !left_type.is_numeric_type() {
            return Err(err_wrong_type(left_type, left));
        }

        if !right_type.is_error() && !right_type.is_numeric_type() {
            return Err(err_wrong_type(right_type, right));
        }

        Ok(())
    }
}

enum BinaryTypeCheck {
    Equal,
    Logical,
    Compare,
    Numeric,
    Bitwise,
}

fn to_binary_typecheck(operator: &BinaryOperatorKind) -> BinaryTypeCheck {
    use ast::BinaryOperatorKind as Binary;

    match operator {
        Binary::Add
        | Binary::Sub
        | Binary::Mul
        | Binary::Div
        | Binary::Log
        | Binary::Pow
        | Binary::Mod
        | Binary::Root => BinaryTypeCheck::Numeric,

        Binary::Eq | Binary::NotEq => BinaryTypeCheck::Equal,

        Binary::Lt | Binary::Gt | Binary::Le | Binary::Ge => BinaryTypeCheck::Compare,

        Binary::LogOr | Binary::LogAnd => BinaryTypeCheck::Logical,

        Binary::BitOr | Binary::BitAnd | Binary::BitXor => BinaryTypeCheck::Bitwise,

        Binary::Range | Binary::TypeOf => todo!("{} not yet impl", operator.as_str()),
        Binary::Invalid => todo!("should not be invalid"),
    }
}
