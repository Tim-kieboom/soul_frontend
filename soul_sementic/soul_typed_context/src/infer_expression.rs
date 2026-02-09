use std::iter::zip;

use hir_model::{
    Array, ArrayType, DeRef, ExpressionId, ExpressionKind, HirType, HirTypeKind, If, IfArm, Index, Primitive, PrimitiveSize, ReturnLike
};
use parser_models::{ast::ArrayKind, scope::NodeId};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult}, sementic_level::SementicFault, soul_names::TypeModifier, span::Span
};

use crate::{
    TypedContextAnalyser,
    model::{InferType, Place},
    utils::{
        empty_array_ty, known_bool, known_none, kown_from_literal, none_ty, primitive_ty,
        to_array_ty,
    },
};

impl<'a> TypedContextAnalyser<'a> {
    pub(crate) fn infer_rvalue(&mut self, expression_id: ExpressionId) -> InferType {
        if let Some(ty) = self.expression_types.get(expression_id) {
            return ty.clone();
        }

        let expression = self.get_expression(expression_id);
        let span = expression.get_span();

        let ty = match &expression.node {
            ExpressionKind::AsCastType(type_cast) => {
                let cast_type = InferType::Known(type_cast.cast_type.clone());

                let left_infer = self.infer_rvalue(type_cast.left);
                let left_type = match left_infer {
                    InferType::Known(hir_type) => hir_type,
                    InferType::Variable(_, span) => {
                        self.log_error(SoulError::new(
                            "type should be kown at this time",
                            SoulErrorKind::UnifyTypeError,
                            Some(span),
                        ));
                        return cast_type;
                    }
                };

                let result = self.unify_primitive_cast(&left_type, &type_cast.cast_type, span);
                match result {
                    Ok(()) => (),
                    Err(err) => {
                        self.log_error(err);
                        return cast_type;
                    }
                };

                cast_type
            }
            ExpressionKind::If(r#if) => {
                let condition_span = self.get_expression(r#if.condition).get_span();

                let condition = self.infer_rvalue(r#if.condition);
                self.unify(
                    r#if.condition,
                    &condition,
                    &known_bool(None, condition_span),
                    condition_span,
                );

                let mut then_ty = self.infer_block(r#if.body);
                self.infer_if_arms(r#if, &mut then_ty);

                then_ty
            }
            ExpressionKind::While(r#while) => {
                if let Some(condition_id) = r#while.condition {
                    let span = self.get_expression(condition_id).get_span();
                    let condition = self.infer_rvalue(condition_id);
                    self.unify(condition_id, &condition, &known_bool(None, span), span);
                }

                self.infer_block(r#while.body);
                known_none(span)
            }
            ExpressionKind::Block(block_id) => self.infer_block(*block_id),
            ExpressionKind::Binary(binary) => {
                let span = binary.operator.get_span();
                let ltype = self.infer_rvalue(binary.left);
                let rtype = self.infer_rvalue(binary.right);
                self.resolve_binary_type(binary, &ltype, &rtype, span)
            }
            ExpressionKind::Literal(literal) => kown_from_literal(literal, span),
            ExpressionKind::ResolvedVariable(_) => {
                let place = self.infer_place(expression_id);
                place.get_type().clone()
            }
            ExpressionKind::FunctionCall(function_call) => {
                let signature = self.get_function_signature(function_call.resolved);
                
                
                let parameter_len = signature.parameters.len();
                let argument_len = function_call.arguments.len();
                if parameter_len != argument_len {
                    let last_arg_span = function_call
                        .arguments
                        .last()
                        .map(|arg| arg.get_span())
                        .unwrap_or(function_call.name.get_span());
                    
                    self.log_error(SoulError::new(
                        format!("expected {parameter_len} arguments but has {argument_len} arguments"),
                        SoulErrorKind::InvalidEscapeSequence, 
                        Some(last_arg_span),
                    ));
                }

                for (value, field) in zip(&function_call.arguments, &signature.parameters) {
                    let ty = &field.ty;
                    let expression_id = value.node;
                    let arg_span = self.get_expression(expression_id).get_span();
                    let argument = self.infer_rvalue(expression_id);
                    self.unify_rtype(expression_id, &argument, ty, arg_span);
                }
                InferType::Known(signature.return_type.clone())
            }
            ExpressionKind::Continue(_) => known_none(span),
            ExpressionKind::Array(array) => self.infer_array(array, span),

            ExpressionKind::Fall(return_like)
            | ExpressionKind::Break(return_like)
            | ExpressionKind::Return(return_like) => self.infer_return_like(return_like, span),
            ExpressionKind::Ref(r#ref) => {
                const MUT: bool = true;
                const CONST: bool = false;

                let infer = self.infer_rvalue(r#ref.expression);
                let mut ty = match expect_known_type(infer) {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        none_ty(span)
                    }
                };

                if r#ref.mutable && !matches!(ty.modifier, Some(TypeModifier::Mut)) {

                    self.log_error(SoulError::new(
                        format!("trying to mutRef (so '&') on expression type '{}' but type has to be 'mut'", ty.display()), 
                        SoulErrorKind::UnifyTypeError, 
                        Some(span),
                    ));
                }
                ty.modifier = None;

                let ref_span = self.get_expression(r#ref.expression).get_span();
                let span = ref_span.combine(span);
                let kind = match ty.kind {
                    HirTypeKind::Array(array)
                        if matches!(
                            array.kind,
                            ArrayKind::StackArray(_) | ArrayKind::HeapArray
                        ) =>
                    {
                        let kind = match r#ref.mutable {
                            MUT => ArrayKind::MutSlice,
                            CONST => ArrayKind::ConstSlice,
                        };
                        HirTypeKind::Array(ArrayType {
                            type_of: array.type_of,
                            kind,
                        })
                    }
                    _ => HirTypeKind::Ref {
                        ty: Box::new(ty),
                        mutable: r#ref.mutable,
                    },
                };

                InferType::Known(HirType {
                    kind,
                    modifier: None,
                    span,
                })
            }
            ExpressionKind::Unary(unary) => {
                // TODO add unary checking
                self.infer_rvalue(unary.expression)
            }
            ExpressionKind::DeRef(deref) => {
                let infer = self.infer_rvalue(deref.inner);
                let ty = match expect_known_type(infer) {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        return known_none(span);
                    }
                };

                let deref_ty = match ty.try_deref(span) {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        return known_none(span);
                    }
                };

                InferType::Known(deref_ty)
            }
            ExpressionKind::Index(index) => {
                self.infer_index(index, span)
            }

            ExpressionKind::Null => {
                let ty = HirType::new_untyped(span);
                InferType::Known(HirType::new_optional(ty, span))
            }
            ExpressionKind::Default => self.environment.alloc_variable(span),
        };

        self.expression_types.insert(expression_id, ty.clone());
        ty
    }

    pub(crate) fn infer_place(&mut self, expression_id: ExpressionId) -> Place {
        fn empty_place<'a>(this: &mut TypedContextAnalyser<'a>, span: Span) -> Place {
            Place::Local(
                NodeId::internal_new(0),
                this.environment.alloc_variable(span),
            )
        }

        let expression = self.get_expression(expression_id);
        let span = expression.get_span();

        match &expression.node {
            ExpressionKind::ResolvedVariable(node_id) => {
                let ty = match self.locals.get(*node_id) {
                    Some(val) => val.clone(),
                    None => self.environment.alloc_variable(span),
                };

                Place::Local(*node_id, ty)
            }
            ExpressionKind::Index(index) => {
                let ty = self.infer_index(index, span);
                self.locals.insert(index.id, ty.clone());

                Place::Local(index.id, ty)
            }
            ExpressionKind::DeRef(DeRef{ id, inner }) => {
                let infer = match self.locals.get(*inner) {
                    Some(val) => val.clone(),
                    None => self.environment.alloc_variable(span),
                };
                let ty = match expect_known_type(infer).map(|ty| ty.try_deref(span)) {
                    Ok(Ok(val)) => val,
                    
                    Err(err)
                    | Ok(Err(err)) => {
                        self.log_error(err);
                        return empty_place(self, span);
                    }
                };
                
                Place::Local(*id, InferType::Known(ty))
            }
            _ => {
                self.faults.push(SementicFault::error(SoulError::new(
                    "expression is not assignable",
                    SoulErrorKind::PlaceTypeError,
                    Some(span),
                )));

                empty_place(self, span)
            }
        }
    }

    
    fn infer_return_like(&mut self, return_like: &ReturnLike, span: Span) -> InferType {
        self.current_return_count += 1;
        let expected_type = match &self.current_return_type {
            Some(val) => val.clone(),
            None => {
                self.log_error(SoulError::new(
                    format!(
                        "trying to {} in a scope without return type",
                        return_like.kind.as_keyword().as_str()
                    ),
                    SoulErrorKind::UnifyTypeError,
                    Some(span),
                ));
                return known_none(span);
            }
        };
        
        let (id, return_type) = match return_like.value {
            Some(value) => (value, self.infer_rvalue(value)),
            None => (return_like.id, known_none(span)),
        };
        
        self.unify(id, &return_type, &expected_type, span);
        return_type
    }
    
    fn infer_index(&mut self, index: &Index, span: Span) -> InferType {
        const UINT: Primitive = Primitive::Uint(PrimitiveSize::SystemSize);
        fn uint(span: Span) -> HirType {
            primitive_ty(UINT, None, span)
        }
        
        let collection_infer = self.infer_rvalue(index.collection);
        let mut index_infer = self.infer_rvalue(index.index);
        
        let collection_type = match expect_known_type(collection_infer) {
            Ok(val) => Some(val),
            Err(err) => {
                self.log_error(err);
                None
            }
        };
        
        let element_type = match collection_type {
            Some(HirType {
                kind: HirTypeKind::Array(element_type),
                ..
            }) => InferType::Known(*element_type.type_of),
            Some(ty) => {
                self.log_error(SoulError::new(
                    format!("index collection should be array type is {}", ty.display()),
                    SoulErrorKind::UnifyTypeError,
                    Some(ty.span),
                ));
                known_none(ty.span)
            }
            None => known_none(span),
        };
        
        if self.try_resolve_untyped_number(&mut index_infer, Some(UINT), span) {
            self.unify_rtype(index.index, &index_infer, &uint(span), span);
        }
        
        element_type
    }
    
    fn infer_array(&mut self, array: &Array, span: Span) -> InferType {
        let values = &array.values;
        if values.is_empty() {
            return match array.element_type.clone() {
                Some(element_type) => {
                    InferType::Known(to_array_ty(element_type, ArrayKind::StackArray(0)))
                }
                None => InferType::Known(empty_array_ty(span)),
            };
        }
        
        let first_infer = match values.first() {
            Some((id, _)) => self.infer_rvalue(*id),
            None => self.environment.alloc_variable(span),
        };
        
        let mut element_type = match first_infer {
            InferType::Known(val) => val,
            InferType::Variable(_, span) => {
                self.log_error(SoulError::new(
                    "type should be known at this point",
                    SoulErrorKind::UnifyTypeError, 
                    Some(span),
                ));
                none_ty(span)
            }
        };
        
        for (id, span) in values.iter().skip(1) {
            let ty = self.infer_rvalue(*id);
            self.unify_rtype(*id, &ty, &element_type, *span);
            
            match ty {
                InferType::Known(ty) => {
                    element_type = ty.consume_new_priority(&element_type);
                }
                _ => self.log_error(SoulError::new(
                    "type should be known at this point",
                    SoulErrorKind::UnifyTypeError, 
                    Some(*span),
                )),
            }
        }
        
        let mut element_infer = InferType::Known(element_type);
        if let Some(ty) = &array.element_type {
            self.unify_rtype(array.id, &element_infer, ty, span);
            element_infer = InferType::Known(ty.clone());
        }
        
        let mut hir_type = match expect_known_type(element_infer) {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                return known_none(span);
            }
        };
        
        hir_type.modifier = None;
        let size = values.len() as u64;
        InferType::Known(HirType {
            kind: HirTypeKind::Array(ArrayType {
                type_of: Box::new(hir_type),
                kind: ArrayKind::StackArray(size),
            }),
            modifier: None,
            span,
        })
    }
    
    fn infer_if_arms(&mut self, r#if: &If, if_type: &mut InferType) {
        let mut current = r#if.else_arm.as_ref();
        
        while let Some(arm) = current {
            let span;
            let ty = match &**arm {
                IfArm::Else(body_id) => {
                    current = None;
                    span = self.get_body(*body_id).span();
                    self.infer_block(*body_id)
                }
                IfArm::ElseIf(elif) => {
                    span = self.get_expression(elif.condition).get_span();
                    let condition = self.infer_rvalue(elif.condition);
                    
                    self.unify(elif.condition, &condition, &known_bool(None, span), span);
                    
                    current = elif.else_arm.as_ref();
                    self.infer_block(elif.body)
                }
            };
            
            self.unify(r#if.condition, if_type, &ty, span);
        }
    }
}

fn expect_known_type(infer: InferType) -> SoulResult<HirType> {
    match infer {
        InferType::Known(hir_type) => Ok(hir_type),
        InferType::Variable(_, span) => Err(SoulError::new(
            "could not infer type",
            SoulErrorKind::TypeInferenceError,
            Some(span),
        )),
    }
}
