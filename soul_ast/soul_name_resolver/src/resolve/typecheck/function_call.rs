use ast_model::{
    CustomType,
    expression::{ExpressionId, FunctionCall},
    soul_type::{Generic, SoulType},
    statements::{EnumVariant, UnionKind},
};
use soul_utils::{FunctionId, fault::Fault};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(crate) fn finish_call_resolution(
        &mut self,
        expression_id: ExpressionId,
        call: &FunctionCall,
        function_id: FunctionId,
    ) {
        if function_id == FunctionId::ERROR {
            return;
        }
        let Some((signature, _)) = self.declares.get_function(function_id) else {
            return;
        };
        let return_type = signature.return_type.clone();
        let parameters = signature.parameters.clone();
        let generics = signature.generics.clone();

        // Only checkable when arity matches positionally and no argument is named.
        let checkable = call.arguments.len() == parameters.len()
            && call.arguments.iter().all(|arg| arg.name.is_none());

        if checkable {
            let mut generic_bindings: Vec<(&str, SoulType)> = Vec::new();

            for (argument, parameter) in call.arguments.iter().zip(&parameters) {
                if matches!(parameter.ty, SoulType::ImplTrait(_)) {
                    continue;
                }

                let Some(arg_ty) = self.expression_type(argument.value) else {
                    continue;
                };

                let span = self
                    .store
                    .expressions
                    .get(argument.value)
                    .map(|expr| expr.span)
                    .unwrap_or(call.name.span());

                if let Some(generic_name) = generic_name_of(&parameter.ty, &generics) {
                    match generic_bindings
                        .iter()
                        .find(|(name, _)| *name == generic_name)
                    {
                        Some((_, bound_ty)) => {
                            if self.combine_operand_types(&arg_ty, bound_ty).is_none() {
                                self.log_fault(Fault::error(
                                    format!(
                                        "generic parameter `{generic_name}` inferred as both `{bound_ty:?}` and `{arg_ty:?}`"
                                    ),
                                    Some(span),
                                ));
                            }
                        }
                        None => generic_bindings.push((generic_name, arg_ty)),
                    }
                    continue;
                }

                if self.combine_operand_types(&arg_ty, &parameter.ty).is_some() {
                    continue;
                }

                self.log_fault(Fault::error(
                    format!(
                        "argument type mismatch: expected `{:?}`, got `{arg_ty:?}`",
                        parameter.ty
                    ),
                    Some(span),
                ));
            }
        }

        self.declares
            .insert_expression_type(expression_id, return_type);
    }

    pub(crate) fn check_enum_variant_construction(
        &mut self,
        owner_type: &SoulType,
        call: &FunctionCall,
    ) {
        let SoulType::Stub(stub) = owner_type else {
            return;
        };

        let Some(entry) = self.lookup_type(&stub.name, self.current.module) else {
            return;
        };

        let node_id = entry.node_id;
        let Some((CustomType::Enum(enum_), _)) = self.declares.get_custom_type(node_id) else {
            return;
        };

        let variant_name = call.name.as_str();
        let Some(EnumVariant::Union(UnionKind::Tuple { parameters, .. })) = enum_
            .variants
            .iter()
            .find(|variant| enum_variant_name(variant) == variant_name)
        else {
            return;
        };
        let parameters = parameters.clone();

        if call.arguments.len() != parameters.len() {
            self.log_fault(Fault::error(
                format!(
                    "variant `{}.{}` expects {} argument(s), got {}",
                    stub.name,
                    variant_name,
                    parameters.len(),
                    call.arguments.len()
                ),
                Some(call.name.span()),
            ));
            return;
        }

        for (argument, param_ty) in call.arguments.iter().zip(&parameters) {
            let Some(arg_ty) = self.expression_type(argument.value) else {
                continue;
            };
            if self.combine_operand_types(&arg_ty, param_ty).is_some() {
                continue;
            }

            let span = self.store.expressions.get(argument.value).map(|e| e.span);
            self.log_fault(Fault::error(
                format!(
                    "variant `{}.{}` argument type mismatch: expected `{param_ty:?}`, got `{arg_ty:?}`",
                    stub.name.as_str(),
                    variant_name
                ),
                span,
            ));
        }
    }
}

pub(crate) fn is_generic_parameter(ty: &SoulType, generics: &[Generic]) -> bool {
    match ty {
        SoulType::ImplTrait(_) => true,
        SoulType::Stub(stub) => generics
            .iter()
            .any(|generic| generic.name.as_str() == stub.name.as_str()),
        _ => false,
    }
}

/// The declared generic's name if `ty` is a bare reference to it (e.g. `T`
/// in `foo<T>(a: T)`), so repeated uses of the same generic within one call
/// can be checked against each other.
pub(crate) fn generic_name_of<'g>(ty: &SoulType, generics: &'g [Generic]) -> Option<&'g str> {
    let SoulType::Stub(stub) = ty else {
        return None;
    };
    generics
        .iter()
        .find(|generic| generic.name.as_str() == stub.name.as_str())
        .map(|generic| generic.name.as_str())
}

fn enum_variant_name(variant: &EnumVariant) -> &str {
    match variant {
        EnumVariant::Normal(name) => name.as_str(),
        EnumVariant::Assigned { name, .. } => name.as_str(),
        EnumVariant::Union(UnionKind::Tuple { name, .. } | UnionKind::NamedTuple { name, .. }) => {
            name.as_str()
        }
    }
}
