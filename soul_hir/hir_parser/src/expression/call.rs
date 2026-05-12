use ast::{FunctionKind, Intrinsic, IntrinsicValue, Literal};
use hir::{Expression, ExpressionId, HirType, LazyTypeId, TypeId};

#[cfg(debug_assertions)]
use soul_utils::soul_error_internal;
use soul_utils::{
    IdAlloc,
    error::{SoulError, SoulErrorKind},
    ids::FunctionId,
    span::Span,
};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_call(
        &mut self,
        id: hir::ExpressionId,
        function_call: &ast::FunctionCall,
    ) -> hir::Expression {
        if let Some(intrinsic) = function_call.intrinsic {
            if intrinsic == Intrinsic::PtrOffset {
                return self.lower_ptr_offset(id, &function_call.arguments, function_call.name.span);
            }
            if intrinsic == Intrinsic::StackArrayIndex {
                return self.lower_stack_array_index(id, &function_call.arguments, function_call.name.span);
            }
            if intrinsic == Intrinsic::Drop {
                return self.lower_drop(id, &function_call.arguments, function_call.name.span);
            }
            return self.lower_intrinsic(
                id,
                intrinsic,
                function_call.name.span,
                &function_call.intrinsic_value,
            );
        }

        if let Some(external_ref) = &function_call.external_ref {
            return self.lower_external_call(
                id,
                function_call,
                &external_ref.crate_name,
                &external_ref.module_path,
            );
        }

        let resolved = match function_call.resolved {
            Some(val) => val,
            None => {
                return hir::Expression::error(id);
            }
        };

        if resolved == FunctionId::error() {
            return hir::Expression::error(id);
        }

        let signature = match self.ast_context.store.get_function(resolved) {
            Some((signature, _)) => signature,
            None => {
                #[cfg(debug_assertions)]
                self.log_error(soul_error_internal!(
                    "could not find function",
                    Some(function_call.name.span)
                ));
                return hir::Expression::error(id);
            }
        };

        let has_this = !matches!(signature.function_kind, FunctionKind::Static);
        let positional_offset: usize = if has_this { 1 } else { 0 };

        let mut arguments = vec![];
        arguments.resize(
            signature.parameters.len() + positional_offset,
            ExpressionId::error(),
        );

        if let Some(callee) = &function_call.callee {
            let span = function_call.name.span;
            let receiver_id = match signature.function_kind {
                FunctionKind::Static => None,
                FunctionKind::ConstRef | FunctionKind::MutRef => {
                    let mutable = signature.function_kind == FunctionKind::MutRef;
                    let of_type = self.lower_type(&signature.methode_type, span);
                    let ty = self
                        .add_type(HirType::new(hir::HirTypeKind::Ref { of_type, mutable }))
                        .to_lazy();
                    let place = self.lower_place(callee);
                    let id = self.alloc_expression(span);
                    self.insert_expression(
                        id,
                        Expression {
                            id,
                            ty,
                            kind: hir::ExpressionKind::Ref { place, mutable },
                        },
                    );
                    Some(id)
                }
                FunctionKind::Consume => Some(self.lower_expression(callee)),
            };

            if let Some(receiver_id) = receiver_id {
                arguments[0] = receiver_id;
            }
        }

        for (i, argument) in function_call.arguments.iter().enumerate() {
            let ast_param_idx = match self.get_parameter_index(
                i,
                argument,
                &signature.parameters,
                function_call.name.span,
            ) {
                Ok(val) => val,
                Err(span) => {
                    self.log_error(SoulError::new(
                        format!("parameters of {} argument not found", i),
                        SoulErrorKind::InvalidContext,
                        Some(span),
                    ));
                    continue;
                }
            };
            let slot = ast_param_idx + positional_offset;
            arguments[slot] = self.lower_expression(&argument.value);
        }

        for (slot, argument) in arguments.iter_mut().enumerate() {
            if *argument != ExpressionId::error() {
                continue;
            }

            if positional_offset > 0 && slot < positional_offset {
                let span = function_call.name.span;
                self.log_error(SoulError::new(
                    "missing receiver for method call".to_string(),
                    SoulErrorKind::InvalidContext,
                    Some(span),
                ));
                let id = self.alloc_expression(span);
                *argument = {
                    let err = Expression::error(id);
                    self.insert_expression(id, err)
                };
                continue;
            }

            let ast_i = slot - positional_offset;
            *argument = match &signature.parameters[ast_i].default {
                Some(val) => {
                    self.lower_default_expression(val, function_call.name.span)
                }
                None => {
                    let span = function_call.name.span;
                    self.log_error(SoulError::new(
                        format!("argument {} not found in function declation", slot + 1),
                        SoulErrorKind::InvalidContext,
                        Some(span),
                    ));
                    let id = self.alloc_expression(span);
                    let err = Expression::error(id);
                    self.insert_expression(id, err)
                }
            };
        }

        let mut generics = vec![];
        for soul_type in &function_call.generics {
            let ty = self.lower_type(soul_type, soul_type.span);
            let kown = match ty {
                LazyTypeId::Known(type_id) => type_id,
                LazyTypeId::Infer(_) => {
                    self.log_error(SoulError::new(
                        "type should be known at this point",
                        SoulErrorKind::TypeInferenceError,
                        Some(soul_type.span),
                    ));
                    TypeId::error()
                }
            };

            generics.push(kown);
        }

        let call_generics = signature
            .generics
            .iter()
            .map(|generic| generic.name.to_string())
            .zip(generics.iter().copied())
            .collect();

        let ty = match Self::convert_type(
            &signature.return_type,
            &self.scopes,
            &call_generics,
            &mut self.tree.info.types,
            signature.return_type.span,
        ) {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                LazyTypeId::error()
            }
        };

        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Call {
                generics,
                arguments,
                function: resolved,
                has_callee: function_call.callee.is_some(),
            },
        }
    }

    fn get_parameter_index(
        &mut self,
        i: usize,
        argument: &ast::Argument,
        parameters: &Vec<ast::NamedTupleElement>,
        span: Span,
    ) -> Result<usize, Span> {
        if let Some(name) = &argument.name {
            let (parameter_i, parameter) =
                find_default_parameter(name.as_str(), parameters).ok_or(name.span)?;
            if parameter.default.is_none() {
                self.log_error(SoulError::new(
                    format!("{} is not a default parameter", name.as_str()),
                    SoulErrorKind::InvalidContext,
                    Some(argument.value.span),
                ));
            }
            Ok(parameter_i)
        } else {
            let parameter = parameters.get(i).ok_or(span)?;
            if parameter.default.is_some() {
                let name = &parameter.name;
                self.log_error(SoulError::new(
                    format!(
                        "argument {} is a default parameter should add name (so '{}: <value>')",
                        i + 1,
                        name.as_str()
                    ),
                    SoulErrorKind::InvalidContext,
                    Some(argument.value.span),
                ));
            }
            Ok(i)
        }
    }
}

fn find_default_parameter<'a>(
    name: &str,
    parameters: &'a ast::NamedTupleType,
) -> Option<(usize, &'a ast::NamedTupleElement)> {
    parameters
        .iter()
        .enumerate()
        .find(|(_, parameter)| parameter.name.as_str() == name)
}

impl<'a> HirContext<'a> {
    fn lower_intrinsic(
        &mut self,
        id: hir::ExpressionId,
        intrinsic: Intrinsic,
        span: Span,
        intrinsic_value: &Option<IntrinsicValue>,
    ) -> hir::Expression {
        match intrinsic {
            Intrinsic::InFile => {
                
                let path_str = match intrinsic_value {
                    Some(IntrinsicValue::Literal(Literal::Str(val))) => val.clone(),
                    _ if self.current.module == self.root_id => {
                        "main.soul".to_string()
                    }
                    _ => {

                        self.module_to_source_path
                            .get(&self.current.module)
                            .map(|p| p.strip_prefix(&self.source_folder).ok())
                            .flatten()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    }
                };

                let literal = Literal::Str(path_str);
                let ty = self.type_from_literal(&literal);
                hir::Expression {
                    id,
                    ty: LazyTypeId::Known(ty),
                    kind: hir::ExpressionKind::Literal(literal),
                }
            }
            Intrinsic::InLine => {
                let line = match intrinsic_value {
                    Some(IntrinsicValue::Literal(Literal::Int(line))) => *line,
                    _ => span.start_line as i128,
                };

                let ty = self.type_from_literal(&Literal::Int(line));
                hir::Expression {
                    id,
                    ty: LazyTypeId::Known(ty),
                    kind: hir::ExpressionKind::Literal(Literal::Int(line)),
                }
            }
            Intrinsic::PtrOffset => unreachable!("PtrOffset should be handled in lower_call"),
            Intrinsic::StackArrayIndex => unreachable!("StackArrayIndex should be handled in lower_call"),
            Intrinsic::Drop => unreachable!("Drop should be handled in lower_call"),
        }
    }

    fn lower_ptr_offset(
        &mut self,
        id: hir::ExpressionId,
        arguments: &[ast::Argument],
        span: Span,
    ) -> hir::Expression {
        let pointer = arguments
            .get(0)
            .map(|arg| self.lower_expression(&arg.value))
            .unwrap_or(ExpressionId::error());

        let offset = arguments
            .get(1)
            .map(|arg| self.lower_expression(&arg.value))
            .unwrap_or(ExpressionId::error());

        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, span),
            kind: hir::ExpressionKind::PtrOffset { pointer, offset },
        }
    }

    fn lower_stack_array_index(
        &mut self,
        id: hir::ExpressionId,
        arguments: &[ast::Argument],
        span: Span,
    ) -> hir::Expression {
        let array = arguments
            .get(0)
            .map(|arg| self.lower_expression(&arg.value))
            .unwrap_or(ExpressionId::error());

        let index = arguments
            .get(1)
            .map(|arg| self.lower_expression(&arg.value))
            .unwrap_or(ExpressionId::error());

        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, span),
            kind: hir::ExpressionKind::StackArrayIndex { array, index },
        }
    }

    fn lower_drop(
        &mut self,
        id: hir::ExpressionId,
        arguments: &[ast::Argument],
        span: Span,
    ) -> hir::Expression {
        let value = arguments
            .get(0)
            .map(|arg| self.lower_expression(&arg.value))
            .unwrap_or(ExpressionId::error());

        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, span),
            kind: hir::ExpressionKind::Drop { value },
        }
    }

    fn lower_default_expression(
        &mut self,
        expr: &ast::Expression,
        call_site_span: Span,
    ) -> ExpressionId {
        if let ast::ExpressionKind::FunctionCall(fc) = &expr.node {
            if fc.intrinsic == Some(Intrinsic::InLine) && fc.intrinsic_value.is_none() {
                let line = call_site_span.start_line as i128;
                let expr_id = self.alloc_expression(expr.span);
                let ty = self.type_from_literal(&Literal::Int(line));
                return self.insert_expression(
                    expr_id,
                    Expression {
                        id: expr_id,
                        ty: LazyTypeId::Known(ty),
                        kind: hir::ExpressionKind::Literal(Literal::Int(line)),
                    },
                );
            }
        }
        self.lower_expression(expr)
    }

    fn lower_external_call(
        &mut self,
        id: hir::ExpressionId,
        function_call: &ast::FunctionCall,
        crate_name: &str,
        full_name: &str,
    ) -> hir::Expression {
        let arguments = function_call
            .arguments
            .iter()
            .map(|arg| self.lower_expression(&arg.value))
            .collect();

        hir::Expression {
            id,
            ty: LazyTypeId::error(),
            kind: hir::ExpressionKind::ExternalCall {
                crate_name: crate_name.to_string(),
                function_name: full_name.to_string(),
                generics: vec![],
                arguments,
            },
        }
    }
}
