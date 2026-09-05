use ast_model::{
    CustomType,
    expression::{AnyArray, Binary, ExpressionId, ExpressionKind, StructConstructor},
    literal::Literal,
    operators::BinaryOperatorKind,
    soul_type::{Mutable, ReferenceType, SoulType, Stub},
    statements::{Field, Struct, VarPattern},
};
use soul_utils::{Ident, TypeModifier, fault::Fault, soul_names::PrimitiveTypes, span::Span};

use super::function_call::{generic_name_of, is_generic_parameter};
use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(crate) fn check_struct_constructor(&mut self, struct_constructor: &StructConstructor) {
        let SoulType::Stub(stub) = &struct_constructor.struct_type else {
            return;
        };

        let Some(entry) = self.lookup_type(&stub.name, self.current.module) else {
            return;
        };

        let custom_type = self.declares.get_custom_type(entry.node_id);
        let Some((CustomType::Struct(struct_), _)) = custom_type else {
            return;
        };

        let faults = self.check_struct_fields(struct_, stub, struct_constructor);
        for fault in faults {
            self.log_fault(fault);
        }
    }

    fn check_struct_fields(
        &self,
        struct_: &Struct,
        stub: &Stub,
        struct_constructor: &StructConstructor,
    ) -> Vec<Fault> {
        fn eq_field_name(field: &Field, field_name: &Ident) -> bool {
            matches!(&field.value.pattern, VarPattern::Simple { binding, .. } if binding.ident.as_str() == field_name.as_str())
        }

        let mut faults = vec![];
        let mut generic_bindings = vec![];
        for (field_name, value_id) in &struct_constructor.values {
            let Some(field) = struct_
                .fields
                .iter()
                .find(|field| eq_field_name(field, field_name))
            else {
                faults.push(Fault::error(
                    format!(
                        "struct `{}` has no field `{}`",
                        stub.name.as_str(),
                        field_name.as_str()
                    ),
                    Some(field_name.span()),
                ));
                continue;
            };

            let Some(field_ty) = &field.value.ty else {
                continue;
            };

            let span = self.store.expressions.get(*value_id).map(|expr| expr.span);

            if let Some(generic_name) = generic_name_of(field_ty, &struct_.generics) {
                let Some(value_ty) = self.expression_type(*value_id) else {
                    continue;
                };

                let generic = generic_bindings
                    .iter()
                    .find(|(name, _)| *name == generic_name);

                match generic {
                    Some((_, bound_ty)) => {
                        if self.combine_operand_types(&value_ty, bound_ty).is_none() {
                            faults.push(Fault::error(
                                format!(
                                    "generic parameter `{generic_name}` inferred as both `{bound_ty:?}` and `{value_ty:?}`"
                                ),
                                span,
                            ));
                        }
                    }
                    None => generic_bindings.push((generic_name, value_ty)),
                }
                continue;
            }

            if is_generic_parameter(field_ty, &struct_.generics) {
                continue;
            }

            let Some(value_ty) = self.expression_type(*value_id) else {
                continue;
            };

            if self.combine_operand_types(&value_ty, field_ty).is_some() {
                continue;
            }

            faults.push(Fault::error(
                format!(
                    "field `{}` type mismatch: expected `{field_ty:?}`, got `{value_ty:?}`",
                    field_name.as_str()
                ),
                span,
            ));
        }

        faults
    }

    pub(crate) fn check_binary_expression(
        &mut self,
        expression_id: ExpressionId,
        span: Span,
        binary: &Binary,
    ) {
        let Some(left_ty) = self.expression_type(binary.left) else {
            return;
        };
        let Some(right_ty) = self.expression_type(binary.right) else {
            return;
        };

        let Some(combined) = self.combine_operand_types(&left_ty, &right_ty) else {
            self.log_fault(Fault::error(
                format!(
                    "type mismatch in binary expression: left is `{left_ty:?}`, right is `{right_ty:?}`"
                ),
                Some(span),
            ));
            return;
        };

        let result_ty = if is_comparison_operator(binary.operator.value) {
            SoulType::Primitive(PrimitiveTypes::Boolean)
        } else {
            combined
        };
        self.declares
            .insert_expression_type(expression_id, result_ty);
    }

    pub(crate) fn expression_type(&self, expression_id: ExpressionId) -> Option<SoulType> {
        if let Some(ty) = self.declares.get_expression_type(expression_id) {
            return Some(ty.clone());
        }

        let expression = self.store.expressions.get(expression_id)?;
        match &expression.node {
            ExpressionKind::Literal((_, literal)) => Some(literal_type(literal)),
            ExpressionKind::Variable(variable) => {
                let resolved = self.declares.get_variable_resolve(variable.id)?;
                let (_, ty, _) = self.declares.get_variable_type(resolved)?;
                ty.clone()
            }
            ExpressionKind::Lambda(lambda) => {
                let return_type = self
                    .first_lambda_return_type(lambda.body)
                    .unwrap_or(SoulType::None);
                Some(SoulType::Function {
                    arity: lambda.parameters.len(),
                    return_type: Box::new(return_type),
                })
            }
            ExpressionKind::FieldAccess(field_access) => {
                let object_ty = self.expression_type(field_access.object)?;
                self.struct_field_type(&object_ty, field_access.field.as_str())
            }
            _ => None,
        }
    }

    pub(crate) fn foreach_collection_element_type(
        &self,
        collection: ExpressionId,
    ) -> Option<SoulType> {
        let expression = self.store.expressions.get(collection)?;
        match &expression.node {
            ExpressionKind::Array(any_array) | ExpressionKind::NewArray(any_array) => {
                self.array_literal_element_type(any_array)
            }
            _ => match self.expression_type(collection)? {
                SoulType::Array(array_ty) => Some(*array_ty.of_type),
                _ => None,
            },
        }
    }

    fn array_literal_element_type(&self, any_array: &AnyArray) -> Option<SoulType> {
        match any_array {
            AnyArray::Array(array) => match &array.element_type {
                Some(ty) => Some(ty.clone()),
                None => Some(default_concrete_type(
                    self.expression_type(*array.values.first()?)?,
                )),
            },
            AnyArray::ArrayFiller(filler) => match &filler.element_type {
                Some(ty) => Some(ty.clone()),
                None => Some(default_concrete_type(self.expression_type(filler.element)?)),
            },
        }
    }

    fn struct_field_type(&self, ty: &SoulType, field_name: &str) -> Option<SoulType> {
        let SoulType::Stub(stub) = ty else {
            return None;
        };
        let entry = self.lookup_type(&stub.name, self.current.module)?;
        let (CustomType::Struct(struct_), _) = self.declares.get_custom_type(entry.node_id)? else {
            return None;
        };

        struct_.fields.iter().find_map(|field| {
            let VarPattern::Simple { binding, .. } = &field.value.pattern else {
                return None;
            };
            if binding.ident.as_str() != field_name {
                return None;
            }
            field.value.ty.clone()
        })
    }

    pub(crate) fn variable_lvalue(
        &self,
        expression_id: ExpressionId,
    ) -> Option<(TypeModifier, SoulType)> {
        let expression = self.store.expressions.get(expression_id)?;
        let ExpressionKind::Variable(variable) = &expression.node else {
            return None;
        };
        let resolved = self.declares.get_variable_resolve(variable.id)?;
        let (modifier, ty, _) = self.declares.get_variable_type(resolved)?;
        Some((*modifier, ty.clone()?))
    }

    pub(crate) fn combine_operand_types(
        &self,
        left: &SoulType,
        right: &SoulType,
    ) -> Option<SoulType> {
        let left = self.resolve_type_alias(left);
        let right = self.resolve_type_alias(right);
        combine_resolved_operand_types(&left, &right)
    }

    fn resolve_type_alias(&self, ty: &SoulType) -> SoulType {
        let mut current = ty.clone();
        for _ in 0..8 {
            let SoulType::Stub(stub) = &current else {
                return current;
            };
            let Some(underlying) = self.declares.get_type_alias(stub.name.as_str()) else {
                return current;
            };
            current = underlying.clone();
        }
        current
    }
}

fn literal_type(literal: &Literal) -> SoulType {
    match literal {
        Literal::Int(_) => SoulType::Primitive(PrimitiveTypes::UntypedInt),
        Literal::Uint(_) => SoulType::Primitive(PrimitiveTypes::UntypedUint),
        Literal::Float(_) => SoulType::Primitive(PrimitiveTypes::UntypedFloat),
        Literal::Bool(_) => SoulType::Primitive(PrimitiveTypes::Boolean),
        Literal::Char(_) => SoulType::Primitive(PrimitiveTypes::Char),
        Literal::Cstr(_) => SoulType::Primitive(PrimitiveTypes::CStr),
        Literal::Str(_) => SoulType::Reference(ReferenceType::with_lifetime(
            SoulType::String,
            Ident::new("static", Span::error()),
            Mutable::Immut,
        )),
    }
}

fn is_comparison_operator(operator: BinaryOperatorKind) -> bool {
    matches!(
        operator,
        BinaryOperatorKind::Eq
            | BinaryOperatorKind::NotEq
            | BinaryOperatorKind::Lt
            | BinaryOperatorKind::Gt
            | BinaryOperatorKind::Le
            | BinaryOperatorKind::Ge
            | BinaryOperatorKind::LogAnd
            | BinaryOperatorKind::LogOr
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NumCategory {
    Int,
    Uint,
    Float,
}

fn concrete_num_category(prim: PrimitiveTypes) -> Option<NumCategory> {
    match prim {
        PrimitiveTypes::CInt
        | PrimitiveTypes::Int
        | PrimitiveTypes::Int8
        | PrimitiveTypes::Int16
        | PrimitiveTypes::Int32
        | PrimitiveTypes::Int64
        | PrimitiveTypes::Int128 => Some(NumCategory::Int),
        PrimitiveTypes::CUint
        | PrimitiveTypes::Uint
        | PrimitiveTypes::Uint8
        | PrimitiveTypes::Uint16
        | PrimitiveTypes::Uint32
        | PrimitiveTypes::Uint64
        | PrimitiveTypes::Uint128 => Some(NumCategory::Uint),
        PrimitiveTypes::Float16 | PrimitiveTypes::Float32 | PrimitiveTypes::Float64 => {
            Some(NumCategory::Float)
        }
        _ => None,
    }
}

fn untyped_targets(kind: PrimitiveTypes) -> &'static [NumCategory] {
    match kind {
        PrimitiveTypes::UntypedInt => &[NumCategory::Int, NumCategory::Float],
        PrimitiveTypes::UntypedUint => &[NumCategory::Int, NumCategory::Uint, NumCategory::Float],
        PrimitiveTypes::UntypedFloat => &[NumCategory::Float],
        _ => &[],
    }
}

fn untyped_kind_of(ty: &SoulType) -> Option<PrimitiveTypes> {
    match ty {
        SoulType::Primitive(
            kind @ (PrimitiveTypes::UntypedInt
            | PrimitiveTypes::UntypedUint
            | PrimitiveTypes::UntypedFloat),
        ) => Some(*kind),
        _ => None,
    }
}

fn combine_untyped_kinds(a: PrimitiveTypes, b: PrimitiveTypes) -> PrimitiveTypes {
    if a == PrimitiveTypes::UntypedFloat || b == PrimitiveTypes::UntypedFloat {
        PrimitiveTypes::UntypedFloat
    } else {
        PrimitiveTypes::UntypedInt
    }
}

pub(crate) fn default_concrete_type(ty: SoulType) -> SoulType {
    match ty {
        SoulType::Primitive(PrimitiveTypes::UntypedInt | PrimitiveTypes::UntypedUint) => {
            SoulType::Primitive(PrimitiveTypes::Int)
        }
        SoulType::Primitive(PrimitiveTypes::UntypedFloat) => {
            SoulType::Primitive(PrimitiveTypes::Float64)
        }
        other => other,
    }
}

fn combine_resolved_operand_types(left: &SoulType, right: &SoulType) -> Option<SoulType> {
    match (untyped_kind_of(left), untyped_kind_of(right)) {
        (None, None) if left == right => Some(left.clone()),
        (None, None) => None,
        (Some(kind), None) => coerce_untyped_to_concrete(kind, right),
        (None, Some(kind)) => coerce_untyped_to_concrete(kind, left),
        (Some(a), Some(b)) => Some(SoulType::Primitive(combine_untyped_kinds(a, b))),
    }
}

fn coerce_untyped_to_concrete(kind: PrimitiveTypes, concrete: &SoulType) -> Option<SoulType> {
    let SoulType::Primitive(prim) = concrete else {
        return None;
    };

    let category = concrete_num_category(*prim)?;
    if untyped_targets(kind).contains(&category) {
        Some(concrete.clone())
    } else {
        None
    }
}
