use hir::{Expression, ExpressionId, LazyTypeId, TypeId};

#[cfg(debug_assertions)]
use soul_utils::soul_error_internal;
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    ids::IdAlloc,
    span::Span,
};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_call(
        &mut self,
        id: hir::ExpressionId,
        function_call: &ast::FunctionCall,
    ) -> hir::Expression {
        let resolved = match function_call.resolved {
            Some(val) => val,
            None => {
                return hir::Expression::error(id);
            }
        };

        let signature = match self.ast_store.get_function(resolved) {
            Some(signature) => signature,
            None => {
                #[cfg(debug_assertions)]
                self.log_error(soul_error_internal!(
                    "could not find function",
                    Some(function_call.name.span)
                ));
                return hir::Expression::error(id);
            }
        };

        let callee = function_call
            .callee
            .as_ref()
            .map(|el| self.lower_expression(el));

        let mut arguments = vec![];
        arguments.resize(signature.parameters.len(), ExpressionId::error());

        for (i, argument) in function_call.arguments.iter().enumerate() {
            let parameter_i = match self.get_parameter_index(
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
            arguments[parameter_i] = self.lower_expression(&argument.value);
        }

        for (i, argument) in arguments.iter_mut().enumerate() {
            if *argument != ExpressionId::error() {
                continue;
            }

            *argument = match &signature.parameters[i].default {
                Some(val) => self.lower_expression(val),
                None => {
                    let span = function_call.name.span;
                    self.log_error(SoulError::new(
                        format!("argument {} not found in function declation", i + 1),
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
            let ty = self.lower_type(soul_type);
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
            &mut self.tree.info.infers,
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
                callee,
                generics,
                arguments,
                function: resolved,
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
        .filter(|(_, parameter)| parameter.name.as_str() == name)
        .next()
}
