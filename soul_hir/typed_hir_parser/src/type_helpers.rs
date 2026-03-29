use ast::ArrayKind;
use hir::{DisplayType, HirType, HirTypeKind, InferTypesMap, LazyTypeId, TypesMap};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult}, soul_names::{PrimitiveTypes, TypeModifier, TypeWrapper}, span::Span
};

use crate::infer_table::UnifyResult;

pub(crate) type MishmatchReason = String;

pub(crate) trait ArrayKindCompatible {
    fn arraykind_compatible(is: &ArrayKind, should_be: &ArrayKind) -> Result<(), MishmatchReason>;
}

pub(crate) trait TypeCompatible {
    fn compatible_type_kind(&self, should_be: &Self) -> Result<UnifyResult, MishmatchReason>;
}

pub(crate) trait TypeKindCompatible {
    fn compatible_type_kind(&self, should_be: &Self) -> Result<(), MishmatchReason>;
}

pub(crate) trait TypeHelpers {
    fn try_deref(
        &self,
        types: &TypesMap,
        infers: &InferTypesMap,
        span: Span,
    ) -> SoulResult<LazyTypeId>;
}

pub(crate) trait UnifyPrimitiveCast {
    fn unify_primitive_cast(
        &self,
        types: &TypesMap,
        infers: &InferTypesMap,
        should_be: &Self,
    ) -> Result<(), MishmatchReason>;
}

pub(crate) trait GetPriority {
    fn get_priority(&self, other: &Self) -> Priority;
}

pub enum Priority {
    Left,
    Right,
}

impl GetPriority for HirType {
    fn get_priority(&self, other: &Self) -> Priority {
        fn number_precendence(ty: &HirType) -> Option<u8> {
            match &ty.kind {
                HirTypeKind::Primitive(val) => number_precedence(val),
                _ => None,
            }
        }

        if self.is_untyped_interger_type() && other.is_untyped_interger_type() {
            if number_precendence(self) < number_precendence(other) {
                Priority::Left
            } else {
                Priority::Right
            }
        } else if self.is_untyped_interger_type() {
            Priority::Right
        } else {
            Priority::Left
        }
    }
}

impl TypeHelpers for HirType {
    fn try_deref(
        &self,
        types: &TypesMap,
        infers: &InferTypesMap,
        span: Span,
    ) -> SoulResult<LazyTypeId> {
        match self.kind {
            HirTypeKind::Ref { of_type, .. } => Ok(of_type),
            HirTypeKind::Pointer(hir_type) => Ok(hir_type),
            other => Err(SoulError::new(
                format!("type {} can not be derefed", other.display(types, infers)),
                SoulErrorKind::TypeInferenceError,
                Some(span),
            )),
        }
    }
}

impl TypeCompatible for HirType {
    fn compatible_type_kind(&self, should_be: &Self) -> Result<UnifyResult, MishmatchReason> {

        let result = match (self.modifier, should_be.modifier) {
            (Some(self_modifier), Some(should_be_modifier)) => {
                if !modifier_compatible(self_modifier, should_be_modifier) {
                    Some(UnifyResult::NeedsAutoCopy)
                } else {
                    None
                }
            }
            _ => None,
        };

        if self.is_error() || should_be.is_error() {
            return Ok(UnifyResult::Ok);
        }

        self.kind.compatible_type_kind(&should_be.kind)?;
        Ok(result.unwrap_or(UnifyResult::Ok))
    }
}

impl TypeKindCompatible for HirTypeKind {
    fn compatible_type_kind(&self, should_be: &Self) -> Result<(), MishmatchReason> {
        match (self, should_be) {
            (HirTypeKind::Primitive(a), HirTypeKind::Primitive(b)) => {
                if !primitive_compatible(a, b) {
                    return Err(format!(
                        "'{}' is not compatible with '{}'",
                        a.as_str(),
                        b.as_str()
                    ));
                }
                Ok(())
            }

            (HirTypeKind::Primitive(PrimitiveTypes::None), HirTypeKind::None)
            | (HirTypeKind::None, HirTypeKind::Primitive(PrimitiveTypes::None))
            | (HirTypeKind::None, HirTypeKind::None)
            | (HirTypeKind::Type, HirTypeKind::Type)
            | (HirTypeKind::Error, _)
            | (_, HirTypeKind::Error) => Ok(()),

            _ => {
                if self == should_be {
                    Ok(())
                } else {
                    Err(format!(
                        "typekind '{}' not compatible with '{}'",
                        self.display_variant(),
                        should_be.display_variant()
                    ))
                }
            }
        }
    }
}

impl ArrayKindCompatible for HirTypeKind {
    fn arraykind_compatible(is: &ArrayKind, should_be: &ArrayKind) -> Result<(), MishmatchReason> {
        let default_format = |a: &ArrayKind, b: &ArrayKind| {
            format!(
                "arraykind '{}' is not compatible with arraykind '{}'",
                a.to_string(),
                b.to_string(),
            )
        };

        match (is, should_be) {
            (ArrayKind::MutSlice, ArrayKind::MutSlice)
            | (ArrayKind::HeapArray, ArrayKind::HeapArray)
            | (ArrayKind::ConstSlice, ArrayKind::ConstSlice) => Ok(()),

            (ArrayKind::StackArray(a_num), ArrayKind::StackArray(b_num)) => {
                if a_num != b_num {
                    Err(default_format(is, should_be))
                } else {
                    Ok(())
                }
            }
            (ArrayKind::StackArray(_), ArrayKind::HeapArray) => Err(format!(
                "{} (maybe try 'new:[....]')",
                default_format(is, should_be)
            )),
            _ => Err(default_format(is, should_be)),
        }
    }
}

impl UnifyPrimitiveCast for HirType {
    fn unify_primitive_cast(
        &self,
        types: &TypesMap,
        infers: &InferTypesMap,
        should_be: &Self,
    ) -> Result<(), MishmatchReason> {
        match (self.modifier, should_be.modifier) {
            (Some(self_modifier), Some(should_be_modifier)) => {
                if !modifier_compatible(self_modifier, should_be_modifier) {
                    return Err(format!(
                        "can not cast from modifier {} to modifier {}",
                        self_modifier.as_str(),
                        should_be_modifier.as_str(),
                    ));
                }
            }
            _ => (),
        };

        self.kind.unify_primitive_cast(types, infers, &should_be.kind) 
    }
}

impl UnifyPrimitiveCast for HirTypeKind {
    fn unify_primitive_cast(
        &self,
        types: &TypesMap,
        infers: &InferTypesMap,
        should_be: &Self,
    ) -> Result<(), MishmatchReason> {
        Ok(match (self, should_be) {
            (
                HirTypeKind::Ref {
                    of_type: a_id,
                    mutable: mut_a,
                },
                HirTypeKind::Ref {
                    of_type: b_id,
                    mutable: mut_b,
                },
            ) => {
                let a = match try_get_type(types, *a_id) {
                    Some(val) => val,
                    None => return Err(display_msg(self, should_be)),
                };
                
                let b = match try_get_type(types, *b_id) {
                    Some(val) => val,
                    None => return Err(display_msg(self, should_be)),
                };

                a.unify_primitive_cast(types, infers, b)?;
                if mut_a != mut_b {
                    let display = |bool: &bool| {
                        if *bool {
                            TypeWrapper::MutRef.as_str()
                        } else {
                            TypeWrapper::ConstRef.as_str()
                        }
                    };
                    return Err(format!(
                        "'{}' can not be cast to '{}'",
                        display(mut_a),
                        display(mut_b)
                    ));
                }
            }
            (HirTypeKind::Array { .. }, HirTypeKind::Array { .. }) => {
                return Err("can only type cast primitive types".to_string());
            }

            (_, HirTypeKind::Pointer(_)) => return Ok(()),

            (HirTypeKind::Optional(a_id), HirTypeKind::Optional(b_id)) => {
                let a = match try_get_type(types, *a_id) {
                    Some(val) => val,
                    None => return Ok(()),
                };
                
                let b = match try_get_type(types, *b_id) {
                    Some(val) => val,
                    None => return Err(display_msg(self, should_be)),
                };

                a.unify_primitive_cast(types, infers, b)?
            }

            (HirTypeKind::None, HirTypeKind::None)
            | (HirTypeKind::Type, HirTypeKind::Type)
            | (HirTypeKind::Primitive(_), HirTypeKind::Primitive(_)) => (),

            (a, HirTypeKind::Optional(b_id)) => {
                let b = match try_get_type(types, *b_id) {
                    Some(val) => val,
                    None => return Ok(()),
                };

                a.unify_primitive_cast(types, infers, &b.kind)?
            }
            _ => {
                return Err(display_msg(self, should_be));
            }
        })
    }
}


fn display_msg(this: &HirTypeKind, should_be: &HirTypeKind) -> MishmatchReason {
    format!(
        "typekind '{}' not compatible with typekind '{}'",
        this.display_variant(),
        should_be.display_variant()
    )
}

fn try_get_type(types: &TypesMap, ty: LazyTypeId) -> Option<&HirType> {
    let ty = match ty {
        LazyTypeId::Known(val) => val,
        LazyTypeId::Infer(_) => return None,
    };

    types.id_to_type(ty)
}


const fn modifier_compatible(this: TypeModifier, should_be: TypeModifier) -> bool {
    match (this, should_be) {
        (TypeModifier::Mut, TypeModifier::Const)
        | (TypeModifier::Mut, TypeModifier::Literal)
        | (TypeModifier::Const, TypeModifier::Literal) => false,
        _ => true,
    }
}

fn primitive_compatible(is: &PrimitiveTypes, should_be: &PrimitiveTypes) -> bool {
    if is.is_untyped_numeric() || should_be.is_untyped_numeric() {
        if should_be.is_untyped_numeric() && should_be.is_untyped_numeric() {
            return true;
        }

        let a = number_precedence(is);
        let b = number_precedence(should_be);
        let both_numbers = a.is_some() && b.is_some();
        if both_numbers && a >= b {
            return true;
        }
    }

    is == should_be
}

fn number_precedence(this: &PrimitiveTypes) -> Option<u8> {
    if this.is_float() {
        Some(1)
    } else if this.is_signed_interger() {
        Some(2)
    } else if this.is_unsigned_interger() {
        Some(3)
    } else {
        None
    }
}
