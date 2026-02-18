use ast::ArrayKind;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{PrimitiveTypes, TypeModifier, TypeWrapper},
    span::Span,
    symbool_kind::SymbolKind,
    vec_map::VecMapIndex,
};
use std::fmt::Write;

use crate::{InferTypeId, TypeId, TypesMap};

pub enum UnifyResult {
    /// fully unifyable
    Ok,
    /// error if auto copy not impl
    NeedsAutoCopy,
}

type MishmatchReason = String;
#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HirType {
    pub kind: HirTypeKind,
    pub modifier: Option<TypeModifier>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirTypeKind {
    None,
    Type,
    Primitive(PrimitiveTypes),
    Array {
        element: TypeId,
        kind: ArrayKind,
    },
    Ref {
        of_type: TypeId,
        mutable: bool,
    },
    Pointer(TypeId),
    Optional(TypeId),

    Error,
    /// special type for unkown hir type (should not exist in Thir and further)
    InferType(InferTypeId),
}

impl HirType {
    pub const fn index_type() -> Self {
        Self {
            kind: HirTypeKind::Primitive(PrimitiveTypes::Uint),
            modifier: None,
        }
    }

    pub const fn none_type() -> Self {
        Self {
            kind: HirTypeKind::None,
            modifier: None,
        }
    }

    pub const fn error_type() -> Self {
        Self {
            kind: HirTypeKind::Error,
            modifier: None,
        }
    }

    pub const fn bool_type() -> Self {
        Self {
            kind: HirTypeKind::Primitive(PrimitiveTypes::Boolean),
            modifier: None,
        }
    }

    pub const fn infer_type(infer_id: InferTypeId) -> Self {
        Self {
            kind: HirTypeKind::InferType(infer_id),
            modifier: None,
        }
    }

    pub const fn new(kind: HirTypeKind) -> Self {
        Self {
            kind,
            modifier: None,
        }
    }

    pub const fn is_infertype(&self) -> bool {
        matches!(self.kind, HirTypeKind::InferType(_))
    }

    pub fn with_modifier(mut self, modifier: TypeModifier) -> Self {
        self.modifier = Some(modifier);
        self
    }

    pub fn display(&self, types: &TypesMap) -> String {
        let mut sb = String::new();
        self.write_display(types, &mut sb)
            .expect("no format errors");
        sb
    }

    pub fn write_display(&self, types: &TypesMap, sb: &mut String) -> std::fmt::Result {
        if let Some(modifier) = self.modifier {
            sb.push_str(modifier.as_str());
            sb.push(' ');
        }

        self.kind.write_display(types, sb)
    }

    pub const fn is_untyped_interger(&self) -> bool {
        self.kind.is_untyped_interger()
    }

    pub const fn is_boolean(&self) -> bool {
        matches!(self.kind, HirTypeKind::Primitive(PrimitiveTypes::Boolean))
    }

    pub const fn is_signed_interger(&self) -> bool {
        if let HirTypeKind::Primitive(prim) = self.kind {
            prim.is_signed_interger()
        } else {
            false
        }
    }

    pub const fn is_unsigned_interger(&self) -> bool {
        if let HirTypeKind::Primitive(prim) = self.kind {
            prim.is_unsigned_interger()
        } else {
            false
        }
    }

    pub const fn is_float(&self) -> bool {
        if let HirTypeKind::Primitive(prim) = self.kind {
            prim.is_float()
        } else {
            false
        }
    }

    pub const fn is_numeric(&self) -> bool {
        self.is_float() || self.is_unsigned_interger() || self.is_signed_interger()
    }

    pub const fn is_primitive(&self) -> bool {
        matches!(self.kind, HirTypeKind::Primitive(_))
    }

    pub fn try_deref(&self, types: &TypesMap, span: Span) -> SoulResult<TypeId> {
        match self.kind {
            HirTypeKind::Ref { of_type, .. } => Ok(of_type),
            HirTypeKind::Pointer(hir_type) => Ok(hir_type),
            other => Err(SoulError::new(
                format!("type {} can not be derefed", other.display(types)),
                SoulErrorKind::TypeInferenceError,
                Some(span),
            )),
        }
    }

    pub fn compatible_type_kind(
        &self,
        should_be: &Self,
    ) -> Result<UnifyResult, MishmatchReason> {
        debug_assert!(!matches!(self.kind, HirTypeKind::InferType(_)), "this fn should be used after infer typed are resolved");
        debug_assert!(!matches!(should_be.kind, HirTypeKind::InferType(_)), "this fn should be used after infer typed are resolved");

        let result = match (self.modifier, should_be.modifier) {
            (Some(self_modifier), Some(should_be_modifier)) => {
                if !HirTypeKind::modifier_compatible(self_modifier, should_be_modifier) {
                    Some(UnifyResult::NeedsAutoCopy)
                } else {
                    None
                }
            }
            _ => None,
        };

        self.kind.compatible_type_kind(&should_be.kind)?;
        Ok(result.unwrap_or(UnifyResult::Ok))
    }

    pub fn unify_primitive_cast(
        &self,
        types: &TypesMap,
        should_be: &Self,
        is_in_unsafe: bool,
    ) -> Result<(), MishmatchReason> {
        
        match (self.modifier, should_be.modifier) {
            (Some(self_modifier), Some(should_be_modifier)) => {
                if !HirTypeKind::modifier_compatible(self_modifier, should_be_modifier) {
                    return Err(format!(
                        "can not cast from modifier {} to modifier {}",
                        self_modifier.as_str(),
                        should_be_modifier.as_str(),
                    ))
                }
            }
            _ => (),
        };

        self.kind.unify_primitive_cast(types, &should_be.kind, is_in_unsafe)
    }

    pub fn resolve_untyped(&mut self, should_be: &Self) {
        match (&mut self.kind, &should_be.kind) {
            (HirTypeKind::Primitive(a), HirTypeKind::Primitive(b)) => {
                a.resolve_untyped(b);
            }
            _ => (),
        }
    }

    pub fn get_priority(&self, other: &Self) -> Priority {
        fn number_precendence(ty: &HirType) -> Option<u8> {
            match &ty.kind {
                HirTypeKind::Primitive(val) => val.number_precedence(),
                _ => None,
            }
        }

        if self.is_untyped_interger() && other.is_untyped_interger() {
            if number_precendence(self) < number_precendence(other) {
                Priority::Left
            } else {
                Priority::Right
            }
        } else if self.is_untyped_interger() || self.kind.is_unknown() {
            Priority::Right
        } else {
            Priority::Left
        }
    }
}
impl HirTypeKind {
    pub fn write_display(&self, types: &TypesMap, sb: &mut String) -> std::fmt::Result {
        const CONST_REF_STR: &str = SymbolKind::ConstRef.as_str();
        const OPTIONAL_STR: &str = SymbolKind::Question.as_str();
        const POINTER_STR: &str = SymbolKind::Star.as_str();
        const MUT_REF_STR: &str = SymbolKind::And.as_str();

        match self {
            HirTypeKind::None => write!(sb, "{}", PrimitiveTypes::None.as_str()),
            HirTypeKind::Type => write!(sb, "type"),
            HirTypeKind::Primitive(prim) => write!(sb, "{}", prim.as_str()),
            HirTypeKind::Array { element, kind } => {
                kind.write_to_string(sb)?;
                write_display_from_id(types, *element, sb)
            }
            HirTypeKind::Ref { of_type, mutable } => {
                let ref_str = match *mutable {
                    true => MUT_REF_STR,
                    false => CONST_REF_STR,
                };

                sb.push_str(ref_str);
                write_display_from_id(types, *of_type, sb)
            }
            HirTypeKind::Pointer(type_id) => {
                sb.push_str(POINTER_STR);
                write_display_from_id(types, *type_id, sb)
            }
            HirTypeKind::Optional(type_id) => {
                sb.push_str(OPTIONAL_STR);
                write_display_from_id(types, *type_id, sb)
            }
            HirTypeKind::Error => write!(sb, "<error>"),
            HirTypeKind::InferType(_) => write!(sb, "<infer>"),
        }
    }

    pub const fn is_untyped_interger(&self) -> bool {
        match self {
            HirTypeKind::Primitive(prim) => prim.is_unsigned_interger(),
            _ => false,
        }
    }

    pub const fn is_unknown(&self) -> bool {
        matches!(self, HirTypeKind::InferType(_))
    }

    pub fn display(&self, types: &TypesMap) -> String {
        let mut sb = String::new();
        self.write_display(types, &mut sb).expect("no format error");
        sb
    }

    pub const fn display_variant(&self) -> &'static str {
        match self {
            HirTypeKind::Type => "type",
            HirTypeKind::None => "none",
            HirTypeKind::Error => "<error>",
            HirTypeKind::Ref { .. } => "<ref>",
            HirTypeKind::Array { .. } => "<array>",
            HirTypeKind::InferType(_) => "<unknown>",
            HirTypeKind::Pointer(_) => "<pointer>",
            HirTypeKind::Optional(_) => "<optional>",
            HirTypeKind::Primitive(primitive) => primitive.as_str(),
        }
    }

    pub fn compatible_type_kind(
        &self,
        should_be: &Self,
    ) -> Result<(), MishmatchReason> {
        match (self, should_be) {
            (HirTypeKind::Primitive(a), HirTypeKind::Primitive(b)) => {
                if !a.compatible(b) {
                    return Err(format!(
                        "'{}' is not compatible with '{}'",
                        a.as_str(),
                        b.as_str()
                    ));
                }
                Ok(())
            }

            (HirTypeKind::None, HirTypeKind::None)
            | (HirTypeKind::Type, HirTypeKind::Type)
            | (HirTypeKind::Error, _)
            | (_, HirTypeKind::Error) => Ok(()),

            _ => if self == should_be {
                Ok(())
            } else {
                Err(format!(
                "typekind '{}' not compatible with '{}'",
                self.display_variant(),
                should_be.display_variant()
            ))},
        }
    }

    pub fn unify_primitive_cast(
        &self,
        types: &TypesMap,
        should_be: &Self,
        is_in_unsafe: bool,
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
                let a = get_type(types, *a_id);
                let b = get_type(types, *b_id);

                a.unify_primitive_cast(types, b, is_in_unsafe)?;
                if mut_a != mut_b {
                    let display = |bool: &bool| {
                        if *bool {
                            TypeWrapper::MutRef.as_str()
                        } else {
                            TypeWrapper::ConstRef.as_str()
                        }
                    };
                    return Err(format!(
                        "'{}' can not be bast to '{}'",
                        display(mut_a),
                        display(mut_b)
                    ));
                }
            }
            (HirTypeKind::Array { .. }, HirTypeKind::Array { .. }) => {
                return Err("can only type cast primitive types".to_string());
            }

            (HirTypeKind::Pointer(a_id), HirTypeKind::Pointer(b_id)) => {
                let a = get_type(types, *a_id);
                let b = get_type(types, *b_id);

                if !is_in_unsafe {
                    return Err("can only type cast pointers in unsafe".to_string());
                }

                if matches!(a.kind, HirTypeKind::InferType(_)) {
                    return Ok(());
                }

                a.unify_primitive_cast(types, b, is_in_unsafe)?
            }

            (HirTypeKind::Optional(a_id), HirTypeKind::Optional(b_id)) => {
                let a = get_type(types, *a_id);
                let b = get_type(types, *b_id);

                if matches!(a.kind, HirTypeKind::InferType(_)) {
                    return Ok(());
                }

                a.unify_primitive_cast(types, b, is_in_unsafe)?
            }

            (HirTypeKind::None, HirTypeKind::None)
            | (HirTypeKind::Type, HirTypeKind::Type)
            | (HirTypeKind::Primitive(_), HirTypeKind::Primitive(_)) => (),

            (a, HirTypeKind::Optional(b_id)) => {
                let b = get_type(types, *b_id);

                if matches!(a, HirTypeKind::InferType(_)) {
                    return Ok(());
                }

                a.unify_primitive_cast(types, &b.kind, is_in_unsafe)?
            }
            _ => {
                return Err(format!(
                    "typekind '{}' not compatible with typekind '{}'",
                    self.display_variant(),
                    should_be.display_variant()
                ));
            }
        })
    }

    pub fn modifier_compatible(this: TypeModifier, should_be: TypeModifier) -> bool {
        match (this, should_be) {
            (TypeModifier::Mut, TypeModifier::Const)
            | (TypeModifier::Mut, TypeModifier::Literal)
            | (TypeModifier::Const, TypeModifier::Literal) => false,
            _ => true,
        }
    }

    pub fn arraykind_compatible(is: &ArrayKind, should_be: &ArrayKind) -> Option<String> {
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
            | (ArrayKind::ConstSlice, ArrayKind::ConstSlice) => None,

            (ArrayKind::StackArray(a_num), ArrayKind::StackArray(b_num)) => {
                if a_num != b_num {
                    Some(default_format(is, should_be))
                } else {
                    None
                }
            }
            (ArrayKind::StackArray(_), ArrayKind::HeapArray) => Some(format!(
                "{} (maybe try 'new:[....]')",
                default_format(is, should_be)
            )),
            _ => Some(default_format(is, should_be)),
        }
    }
}

fn write_display_from_id(types: &TypesMap, ty: TypeId, sb: &mut String) -> std::fmt::Result {
    let ty = match types.get_type(ty) {
        Some(val) => val,
        None => return write!(sb, "/*TypeId({}) not found*/", ty.index()),
    };

    ty.write_display(types, sb)
}

pub trait PrimitiveTypesHelper {
    fn number_precedence(&self) -> Option<u8>;
    fn compatible(&self, should_be: &Self) -> bool;
    fn resolve_untyped(&mut self, should_be: &Self);
}
impl PrimitiveTypesHelper for PrimitiveTypes {
    fn resolve_untyped(&mut self, should_be: &Self) {
        if !self.is_untyped_numeric() {
            return;
        }

        if self.number_precedence() > should_be.number_precedence() {
            *self = should_be.clone();
            return;
        }

        match self {
            PrimitiveTypes::UntypedInt => *self = PrimitiveTypes::Int,
            PrimitiveTypes::UntypedUint => *self = PrimitiveTypes::Int,
            PrimitiveTypes::UntypedFloat => *self = PrimitiveTypes::Float32,
            _ => unreachable!(),
        }
    }

    fn compatible(&self, should_be: &Self) -> bool {
        if self.is_untyped_numeric() || should_be.is_untyped_numeric() {
            if should_be.is_untyped_numeric() && should_be.is_untyped_numeric() {
                return true;
            }

            let a = self.number_precedence();
            let b = should_be.number_precedence();
            let both_numbers = a.is_some() && b.is_some();
            if both_numbers && a >= b {
                return true;
            }
        }

        self == should_be
    }

    fn number_precedence(&self) -> Option<u8> {
        if self.is_float() {
            Some(1)
        } else if self.is_signed_interger() {
            Some(2)
        } else if self.is_unsigned_interger() {
            Some(3)
        } else {
            None
        }
    }
}
pub enum Priority {
    Left,
    Right,
}

fn get_type(types: &TypesMap, ty: TypeId) -> &HirType {
    types.get_type(ty).expect("should have TypeId")
}
