use std::iter::zip;

use hir_model::{Array, ExpressionId, ExpressionKind, HirType, HirTypeKind, If, IfArm, Primitive, ReturnLike};
use parser_models::scope::NodeId;
use soul_utils::{error::{SoulError, SoulErrorKind, SoulResult}, sementic_level::SementicFault, span::Span};

use crate::{TypedContext, model::{InferType, Place}, utils::{known_bool, known_none, kown_from_literal, none_ty, primitive_ty}};

impl<'a> TypedContext<'a> {

    pub(crate) fn infer_rvalue(&mut self, expression_id: ExpressionId) -> InferType {
        if let Some(ty) = self.expression_types.get(expression_id) {
            return ty.clone()
        }

        let expression = self.get_expression(expression_id);
        let span = expression.get_span();
        
        let ty = match &expression.node {
            ExpressionKind::If(r#if) => {
                let condition_span = self.get_expression(r#if.condition).get_span();
    
                let mut condition = self.infer_rvalue(r#if.condition);
                self.unify(&mut condition, &known_bool(None, condition_span), condition_span);

                let mut then_ty = self.infer_block(r#if.body, None);
                self.infer_if_arms(r#if, &mut then_ty);

                then_ty 
            }
            ExpressionKind::While(r#while) => {
                if let Some(condition) = r#while.condition {
                    let span = self.get_expression(condition).get_span();
                    let mut condition = self.infer_rvalue(condition);
                    self.unify(&mut condition, &known_bool(None, span), span);
                }

                self.infer_block(r#while.body, None);
                known_none(span)
            }
            ExpressionKind::Block(block_id) => self.infer_block(*block_id, None),
            ExpressionKind::Binary(binary) => {
                let ltype = self.infer_rvalue(binary.left);
                let rtype = self.infer_rvalue(binary.right);
                // TODO add binary checking
                self.unify(&ltype, &rtype, binary.operator.get_span());
                ltype
            }
            ExpressionKind::Literal(literal) => kown_from_literal(literal, span),
            ExpressionKind::ResolvedVariable(_) => {
                let place = self.infer_place(expression_id);
                place.get_type().clone()
            }
            ExpressionKind::FunctionCall(function_call) => {
                let signature = self.get_function_signature(function_call.resolved);
                for (expression_id, field) in zip(&function_call.arguments, &signature.parameters) {
                    let ty = &field.ty;
                    let arg_span = self.get_expression(*expression_id).get_span();
                    let argument = self.infer_rvalue(*expression_id);
                    self.unify_ltype(ty, &argument, arg_span);
                }
                InferType::Known(signature.return_type.clone())
            }
            ExpressionKind::Continue(_) => known_none(span),
            ExpressionKind::Array(array) => self.infer_array(array, span),
            ExpressionKind::Fall(return_like) => self.infer_return_like(return_like, span),
            ExpressionKind::Break(return_like) => self.infer_return_like(return_like, span),
            ExpressionKind::Return(return_like) => self.infer_return_like(return_like, span),
            
            ExpressionKind::Ref(r#ref) => {
                let infer = self.infer_rvalue(r#ref.expression);
                let mut ty = match expect_known_type(infer) {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        none_ty(span)
                    }
                };

                let modifier = ty.modifier.take();
                InferType::Known(HirType { 
                    kind: HirTypeKind::Ref { 
                        ty: Box::new(ty), 
                        mutable: r#ref.mutable, 
                    }, 
                    modifier, 
                    span,
                })
            }
            ExpressionKind::Unary(unary) => {
                // TODO add unary checking
                self.infer_rvalue(unary.expression)
            }
            ExpressionKind::DeRef(deref) => {
                let infer = self.infer_rvalue(*deref);
                let ty = match expect_known_type(infer) {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        return known_none(span)
                    }
                };

                let deref_ty = match ty.try_deref() {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        return known_none(span)
                    }
                };

                InferType::Known(deref_ty)
            }
            ExpressionKind::Index(index) => {
                const UINT: Primitive = hir_model::Primitive::Uint(hir_model::PrimitiveSize::SystemSize);
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
                    Some(HirType { kind: HirTypeKind::Array(element_type), .. }) => InferType::Known(*element_type),
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

                if self.try_resolve_untyped(&mut index_infer, Some(UINT), span) {

                    self.unify_rtype(&index_infer, &uint(span), span);
                }

                element_type
            }

            ExpressionKind::Default => self.environment.alloc_variable(span),
        };

        self.expression_types.insert(expression_id, ty.clone());
        ty
    }

    pub(crate) fn infer_place(&mut self, expression_id: ExpressionId) -> Place {
        let expression = self.get_expression(expression_id);
        let span = expression.get_span();

        match &expression.node {
            ExpressionKind::ResolvedVariable(node_id) => {
                let ty = self.locals
                    .get(*node_id)
                    .cloned()
                    .unwrap_or_else(|| self.environment.alloc_variable(span));

                Place::Local(*node_id, ty)
            }
            _ => {
                self.faults.push(SementicFault::error(SoulError::new(
                    "expression is not assignable",
                    SoulErrorKind::PlaceTypeError,
                    Some(span),
                )));

                Place::Local(NodeId::internal_new(0), self.environment.alloc_variable(span))
            }
        }
    }

    fn infer_return_like(&mut self, return_like: &ReturnLike, span: Span) -> InferType {
        if let Some(value) = return_like.value {
            self.infer_rvalue(value)
        } else {
            known_none(span)
        }
    }

    fn infer_array(&mut self, array: &Array, span: Span) -> InferType {

        let values = &array.values;
        let element_type = match values.first() {
            Some((id, _)) => self.infer_rvalue(*id),
            None => self.environment.alloc_variable(span),
        };

        for (id, span) in values {
            let ty = self.infer_rvalue(*id);
            self.unify(&element_type, &ty, *span);
        }

        let mut hir_type = match expect_known_type(element_type) {
            Ok(val) => val,
            Err(err) =>  {
                self.log_error(err);
                return known_none(span)
            }
        };

        let modifier = hir_type.modifier.take();
        InferType::Known(HirType { 
            kind: HirTypeKind::Array(Box::new(hir_type)), 
            modifier, 
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
                    self.infer_block(*body_id, None)
                }
                IfArm::ElseIf(elif) => {
                    span = self.get_expression(elif.condition).get_span();
                    let mut condition = self.infer_rvalue(elif.condition);
                    self.unify(&mut condition, &known_none(span), span);

                    current = elif.else_arm.as_ref();
                    self.infer_block(elif.body, None)
                }
            };

            self.unify(if_type, &ty, span);
        }
    }


}

fn expect_known_type(infer: InferType) -> SoulResult<HirType> {
    match infer {
        InferType::Known(hir_type) => Ok(hir_type),
        InferType::Variable(_, span) => {
            Err(SoulError::new(
                "could not infer type",
                SoulErrorKind::TypeInferenceError,
                Some(span),
            ))
        }
    }
}
