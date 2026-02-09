use std::u8;

use parser_models::{ast::ArrayKind, scope::NodeId};
use soul_utils::{Ident, error::{SoulError, SoulErrorKind, SoulResult}, soul_names::{InternalPrimitiveTypes, TypeModifier, TypeWrapper}, span::Span};

use crate::{Visibility};

const BIT8: PrimitiveSize = PrimitiveSize::Bit8;
const BIT16: PrimitiveSize = PrimitiveSize::Bit16;
const BIT32: PrimitiveSize = PrimitiveSize::Bit32;
const BIT64: PrimitiveSize = PrimitiveSize::Bit64;
const BIT124: PrimitiveSize = PrimitiveSize::Bit128;

const SYSTEM_SIZE: PrimitiveSize = PrimitiveSize::SystemSize;
/// Resolved HIR type with generic arguments.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirType {
    /// Core type kind.
    pub kind: HirTypeKind,
    pub modifier: Option<TypeModifier>,
    pub span: Span,
}

pub type TupleType = Vec<HirType>;
pub type NamedTupleType = Vec<FieldType>;

/// Core type kinds in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HirTypeKind {
    /// Reference type (const=`@T` or mut=`&T`).
    Ref {
        ty: Box<HirType>,
        mutable: bool,
    },
    Type,
    Str,
    /// can be null (`?T`).
    Optional(Box<HirType>),
    /// Raw pointer (`*T`).
    Pointer(Box<HirType>),
    /// Primitive type.
    Primitive(Primitive),
    Array(ArrayType),
    /// Empty type `none`.
    None,
    Untyped,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArrayType {
    pub type_of: Box<HirType>,
    pub kind: ArrayKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldType {
    pub id: NodeId,
    pub name: Ident, 
    pub ty: HirType, 
    pub vis: Visibility,
}
impl FieldType {
    pub fn new(id: NodeId, name: Ident, ty: HirType, vis: Visibility) -> Self {
        Self {
            id,
            name,
            ty,
            vis,
        }
    }
}

/// Primitive types with size information.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Primitive {
    Nil,
    /// Boolean type.
    Boolean,
    UntypedInt,
    UntypedUint,
    UntypedFloat,
    /// Signed integer.
    Int(PrimitiveSize),
    /// Character (fixed-width).
    Char(PrimitiveSize),
    /// Unsigned integer.
    Uint(PrimitiveSize),
    /// Floating-point.
    Float(PrimitiveSize),
}

/// Primitive type bit widths.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveSize {
    /// system specific size (same as c_int size default normaly is 32-bit)
    SystemSize = 0,
    /// 8-bit.
    Bit8 = 8,
    /// 16-bit.
    Bit16 = 16,
    /// 32-bit.
    Bit32 = 32,
    /// 64-bit.
    Bit64 = 64,
    /// 128-bit.
    Bit128 = 128,
}

pub enum UnifyResult {
    /// fully unifyable
    Ok,
    /// error if auto copy not impl 
    AutoCopy,
}
impl HirType {
    pub fn new_priority(&self, other: &Self) -> Self {
        match self.inner_priority(other) {
            Priority::This => self.clone(),
            Priority::Other => other.clone(),
        }
    }

    pub fn consume_new_priority(self, other: &Self) -> Self {
        match self.inner_priority(other) {
            Priority::This => self,
            Priority::Other => other.clone(),
        }
    }

    fn inner_priority(&self, other: &Self) -> Priority {
        fn number_precendence(ty: &HirType) -> Option<u8> {
            match &ty.kind {
                HirTypeKind::Primitive(val) => val.number_precedence(),
                _ => None,
            }
        }
        
        if self.is_untyped_primitive() && other.is_untyped_primitive() {

            if number_precendence(self) < number_precendence(other) {
                Priority::This
            } else {
                Priority::Other
            }
        }
        else if self.is_untyped_primitive() || self.kind.is_untyped() {
            Priority::Other
        } else {
            Priority::This
        }

    }

    pub fn new_optional(inner: HirType, span: Span) -> Self {
        Self {
            kind: HirTypeKind::Optional(
                Box::new(inner)
            ),
            modifier: None,
            span,
        }
    }
    
    pub fn new_untyped(span: Span) -> Self {
        Self {
            kind: HirTypeKind::Untyped,
            modifier: None,
            span,
        }
    }

    pub fn is_primitive(&self) -> bool {
        match &self.kind {
            HirTypeKind::Primitive(_) => true,
            _ => false,
        }
    }

    pub fn is_pointer(&self) -> bool {
        self.kind.is_pointer()
    }

    pub fn is_any_ref(&self) -> bool {
        self.kind.is_any_ref()
    }

    pub fn is_untyped_primitive(&self) -> bool {
        self.kind.is_untyped_primitive()
    }

    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb);
        sb
    }

    pub fn inner_display(&self, sb: &mut String) {
        if let Some(modifier) = &self.modifier {
            sb.push_str(modifier.as_str());
            sb.push(' ');
        }
        self.kind.inner_display(sb);
    }

    pub fn try_deref(self, span: Span) -> SoulResult<Self> {
        match self.kind {
            HirTypeKind::Ref { ty, .. } => Ok(*ty),
            HirTypeKind::Pointer(hir_type) => Ok(*hir_type),
            other => {
                Err(
                    SoulError::new(
                    format!("type {} can not be derefed", other.display()),
                    SoulErrorKind::TypeInferenceError,
                    Some(span), 
                ))
            }
        }
    }

    pub fn unify_compatible(&self, should_be: &Self) -> Result<UnifyResult, String> {
        
        let result = match (self.modifier, should_be.modifier) {

            (Some(self_modifier), Some(should_be_modifier)) => {
                if !modifier_compatible(self_modifier, should_be_modifier) {
                    Some(UnifyResult::AutoCopy)
                } else {
                    None
                }
            }
            _ => None,
        };

        self.kind.unify_compatible(&should_be.kind)?;
        Ok(result.unwrap_or(UnifyResult::Ok))
    }

    pub fn unify_primitive_cast(&self, should_be: &Self) -> Result<(), String> {
        fn err_non_primitive(ty: &HirType) -> String {
            format!(
                "can only use primitive types for casting '{}' is not primitve",
                ty.display(),
            )
        }
        
        if !self.is_primitive() {
            return Err(err_non_primitive(self))
        }
        if !should_be.is_primitive() {
            return Err(err_non_primitive(should_be))
        }

        Ok(())
    }

    pub fn resolve_untyped(&mut self, should_be: &Self) {
        match (&mut self.kind, &should_be.kind) {
            (HirTypeKind::Primitive(a), HirTypeKind::Primitive(b)) => {
                a.resolve_untyped(b);
            }
            _ => (),
        }
    }
}
impl HirTypeKind {

    pub fn unify_compatible(&self, should_be: &Self) -> Result<UnifyResult, String> {
        
        Ok(match (self, should_be) {
            (_, HirTypeKind::Untyped) => UnifyResult::Ok,

            (HirTypeKind::Ref { ty: a, mutable: mut_a }, HirTypeKind::Ref { ty: b, mutable: mut_b }) => {
                a.unify_compatible(b)?;
                if mut_a != mut_b {
                    let display = |bool: &bool| if *bool {TypeWrapper::MutRef.as_str()} else {TypeWrapper::ConstRef.as_str()};
                    return Err(
                        format!("'{}' is not compatible with '{}'", display(mut_a), display(mut_b))
                    )
                }
                
                UnifyResult::Ok
            }
            (HirTypeKind::Array(a), HirTypeKind::Array(b)) => {
                if matches!(b.type_of.kind, HirTypeKind::Untyped) 
                    || matches!(a.type_of.kind, HirTypeKind::Untyped)
                {
                    return Ok(UnifyResult::Ok)
                }

                if let Some(msg) = arraykind_compatible(a.kind, b.kind) {
                    return Err(msg)
                } 
                a.type_of.unify_compatible(&b.type_of)?
            }

            (HirTypeKind::Pointer(a), HirTypeKind::Pointer(b))
            | (HirTypeKind::Optional(a), HirTypeKind::Optional(b)) => {
                if matches!(a.kind, HirTypeKind::Untyped) {
                    return Ok(UnifyResult::Ok)
                }

                a.unify_compatible(b)?
            }
            
            (HirTypeKind::Primitive(a), HirTypeKind::Primitive(b)) => {
                if !a.compatible(b) {
                    return Err(
                        format!("'{}' is not compatibe with '{}'", a.display(), b.display())
                    )
                }

                UnifyResult::Ok
            }
            (HirTypeKind::Str, HirTypeKind::Str)
            | (HirTypeKind::None, HirTypeKind::None)
            | (HirTypeKind::Type, HirTypeKind::Type) => UnifyResult::Ok,

            (a, HirTypeKind::Optional(b)) => {
                if matches!(a, HirTypeKind::Untyped) {
                    return Ok(UnifyResult::Ok)
                }

                a.unify_compatible(&b.kind)?
            }
            _ => return Err(
                format!("typekind '{}' not compatible with typekind '{}'", self.display_variant(), should_be.display_variant())
            ),
        })

    }

    pub fn unify_primitive_cast(&self, should_be: &Self, is_in_unsafe: bool) -> Result<(), String> {
        
        Ok(match (self, should_be) {
            (HirTypeKind::Ref { ty: a, mutable: mut_a }, HirTypeKind::Ref { ty: b, mutable: mut_b }) => {
                a.unify_primitive_cast(b)?;
                if mut_a != mut_b {
                    let display = |bool: &bool| if *bool {TypeWrapper::MutRef.as_str()} else {TypeWrapper::ConstRef.as_str()};
                    return Err(
                        format!("'{}' can not be bast to '{}'", display(mut_a), display(mut_b))
                    )
                }
                
            
            }
            (HirTypeKind::Array(_), HirTypeKind::Array(_)) => {
                return Err("can only type cast primitive types".to_string())
            }

            (HirTypeKind::Pointer(a), HirTypeKind::Pointer(b)) => {
                if !is_in_unsafe {
                    return Err("can only type cast pointers in unsafe".to_string())
                }
                
                if matches!(a.kind, HirTypeKind::Untyped) {
                    return Ok(())
                }

                a.unify_primitive_cast(b)?
            }
            
            (HirTypeKind::Optional(a), HirTypeKind::Optional(b)) => {
                if matches!(a.kind, HirTypeKind::Untyped) {
                    return Ok(())
                }

                a.unify_primitive_cast(b)?
            }
            
            (HirTypeKind::Str, HirTypeKind::Str)
            | (HirTypeKind::None, HirTypeKind::None)
            | (HirTypeKind::Type, HirTypeKind::Type) 
            | (HirTypeKind::Primitive(_), HirTypeKind::Primitive(_)) => (),

            | (a, HirTypeKind::Optional(b)) => {
                if matches!(a, HirTypeKind::Untyped) {
                    return Ok(())
                }

                a.unify_primitive_cast(&b.kind, is_in_unsafe)?
            }
            _ => return Err(
                format!("typekind '{}' not compatible with typekind '{}'", self.display_variant(), should_be.display_variant())
            ),
        })

    }

    pub fn is_any_ref(&self) -> bool {
        match self {
            HirTypeKind::Ref { .. } => true,
            _ => false,
        }
    }

    pub fn is_pointer(&self) -> bool {
        match self {
            HirTypeKind::Pointer(_) => true,
            _ => false,
        }
    }

    pub fn is_untyped(&self) -> bool {
        match self {
            HirTypeKind::Untyped => true,
            _ => false,
        }
    }

    pub fn is_untyped_primitive(&self) -> bool {
        match self {
            HirTypeKind::Primitive(primitive) => primitive.is_untyped(),
            _ => false,
        }
    }

    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb);
        sb
    }

    pub fn inner_display(&self, sb: &mut String) {
        const MUT: bool = true;
        const CONST: bool = false;

        match self {
            HirTypeKind::Ref { ty, mutable } => {
                let sym = match *mutable {
                    MUT => TypeWrapper::MutRef.as_str(),
                    CONST => TypeWrapper::ConstRef.as_str(),
                };
                sb.push_str(sym);
                ty.inner_display(sb);
            },
            HirTypeKind::Type => sb.push_str("Type"),
            HirTypeKind::Str => sb.push_str("str"),
            HirTypeKind::Optional(hir_type) => {
                sb.push('?');
                hir_type.inner_display(sb);
            }
            HirTypeKind::Pointer(hir_type) => {
                sb.push('*');
                hir_type.inner_display(sb);
            }
            HirTypeKind::Primitive(primitive) => sb.push_str(
                primitive.to_internal_primitive().map(|el| el.as_str()).unwrap_or("<unkown>")
            ),
            HirTypeKind::Array(array) => {
                sb.push_str(&array.kind.to_string());
                array.type_of.inner_display(sb);
            }
            HirTypeKind::Untyped => sb.push_str("<untyped>"),
            HirTypeKind::None => sb.push_str("none"),
        }
    }

    pub fn display_variant(&self) -> &'static str {
        match self {
            HirTypeKind::Str => "str",
            HirTypeKind::Type => "Type",
            HirTypeKind::None => "none",
            HirTypeKind::Ref { .. } => "<ref>",
            HirTypeKind::Array(_) => "<array>",
            HirTypeKind::Untyped => "<untyped>",
            HirTypeKind::Pointer(_) => "<pointer>",
            HirTypeKind::Optional(_) => "<optional>", 
            HirTypeKind::Primitive(primitive) => primitive.display(),
        }
    }
}
impl Primitive {

    pub fn display(&self) -> &'static str {
        self.to_internal_primitive().map(|el| el.as_str()).unwrap_or("<unkown primitiveType>")
    }

    pub fn resolve_untyped(&mut self, should_be: &Self) {

        if !self.is_untyped() {
            return
        }

        if self.number_precedence() > should_be.number_precedence() {
            *self = should_be.clone();
            return;
        }

        match self {
            Primitive::UntypedInt => *self = Primitive::Int(PrimitiveSize::SystemSize),
            Primitive::UntypedUint => *self = Primitive::Int(PrimitiveSize::SystemSize),
            Primitive::UntypedFloat => *self = Primitive::Float(PrimitiveSize::Bit32),
            _ => unreachable!(),
        }
    }

    pub fn is_untyped(&self) -> bool {
        match self {
            Primitive::UntypedInt 
            | Primitive::UntypedUint
            | Primitive::UntypedFloat => true,
            _ => false
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            Primitive::UntypedInt 
            | Primitive::UntypedUint 
            | Primitive::UntypedFloat 
            | Primitive::Int(_)
            | Primitive::Uint(_)
            | Primitive::Float(_) => true,
            
            Primitive::Nil
            | Primitive::Boolean
            | Primitive::Char(_) => todo!(),
        }
    }

    pub fn compatible(&self, should_be: &Self) -> bool {
        if self.is_untyped() || should_be.is_untyped() {
            if should_be.is_untyped() && should_be.is_untyped() {
                return true
            }

            let a = self.number_precedence(); 
            let b = should_be.number_precedence();
            let both_numbers = a.is_some() && b.is_some();
            if both_numbers && a >= b {
                return true
            }
        }
        
        match (self, should_be) {
            (Primitive::Int(a_size), Primitive::Int(b_size)) 
            | (Primitive::Char(a_size), Primitive::Char(b_size)) 
            | (Primitive::Uint(a_size), Primitive::Uint(b_size)) 
            | (Primitive::Float(a_size), Primitive::Float(b_size)) => a_size == b_size,

            (Primitive::Boolean, Primitive::Boolean) => true,
            _ => false,
        } 
    }

    pub fn number_precedence(&self) -> Option<u8> {
        match self {
            Primitive::Nil 
            | Primitive::Boolean 
            | Primitive::Char(_) => None,

            Primitive::Float(_) | Primitive::UntypedFloat => Some(1),
            Primitive::Int(_) | Primitive::UntypedInt => Some(2),
            Primitive::Uint(_) | Primitive::UntypedUint => Some(3),
        }
    }

    pub fn from_internal_primitive(prim: InternalPrimitiveTypes) -> Primitive {

        
        match prim {
            InternalPrimitiveTypes::None => Primitive::Nil,
            InternalPrimitiveTypes::Boolean => Primitive::Boolean,

            InternalPrimitiveTypes::Char => Primitive::Char(BIT8),
            InternalPrimitiveTypes::Char8 => Primitive::Char(BIT8),
            InternalPrimitiveTypes::Char16 => Primitive::Char(BIT16),
            InternalPrimitiveTypes::Char32 => Primitive::Char(BIT32),
            InternalPrimitiveTypes::Char64 => Primitive::Char(BIT64),

            InternalPrimitiveTypes::UntypedInt => Primitive::UntypedInt,
            InternalPrimitiveTypes::Int => Primitive::Int(SYSTEM_SIZE),
            InternalPrimitiveTypes::Int8 => Primitive::Int(BIT8),
            InternalPrimitiveTypes::Int16 => Primitive::Int(BIT16),
            InternalPrimitiveTypes::Int32 => Primitive::Int(BIT32),
            InternalPrimitiveTypes::Int64 => Primitive::Int(BIT64),
            InternalPrimitiveTypes::Int128 => Primitive::Int(BIT124),

            InternalPrimitiveTypes::UntypedUint => Primitive::UntypedUint,
            InternalPrimitiveTypes::Uint => Primitive::Uint(SYSTEM_SIZE),
            InternalPrimitiveTypes::Uint8 => Primitive::Uint(BIT8),
            InternalPrimitiveTypes::Uint16 => Primitive::Uint(BIT16),
            InternalPrimitiveTypes::Uint32 => Primitive::Uint(BIT32),
            InternalPrimitiveTypes::Uint64 => Primitive::Uint(BIT64),
            InternalPrimitiveTypes::Uint128 => Primitive::Uint(BIT124),

            InternalPrimitiveTypes::UntypedFloat => Primitive::UntypedFloat,
            InternalPrimitiveTypes::Float16 => Primitive::Float(BIT16),
            InternalPrimitiveTypes::Float32 => Primitive::Float(BIT32),
            InternalPrimitiveTypes::Float64 => Primitive::Float(BIT64),
        }
    }

    pub fn to_internal_primitive(&self) -> Option<InternalPrimitiveTypes> {

        Some(match self {
            Primitive::Nil => InternalPrimitiveTypes::None,
            Primitive::Boolean => InternalPrimitiveTypes::Boolean,

            Primitive::Char(SYSTEM_SIZE) => InternalPrimitiveTypes::Char,
            Primitive::Char(BIT8) => InternalPrimitiveTypes::Char8,
            Primitive::Char(BIT16) => InternalPrimitiveTypes::Char16,
            Primitive::Char(BIT32) => InternalPrimitiveTypes::Char32,
            Primitive::Char(BIT64) => InternalPrimitiveTypes::Char64,
            
            Primitive::UntypedInt => InternalPrimitiveTypes::UntypedInt,
            Primitive::Int(SYSTEM_SIZE) => InternalPrimitiveTypes::Int,
            Primitive::Int(BIT8) => InternalPrimitiveTypes::Int8,
            Primitive::Int(BIT16) => InternalPrimitiveTypes::Int16,
            Primitive::Int(BIT32) => InternalPrimitiveTypes::Int32,
            Primitive::Int(BIT64) => InternalPrimitiveTypes::Int64,
            Primitive::Int(BIT124) => InternalPrimitiveTypes::Int128,
            
            Primitive::UntypedUint => InternalPrimitiveTypes::UntypedUint,
            Primitive::Uint(SYSTEM_SIZE) => InternalPrimitiveTypes::Uint,
            Primitive::Uint(BIT8) => InternalPrimitiveTypes::Uint8,
            Primitive::Uint(BIT16) => InternalPrimitiveTypes::Uint16,
            Primitive::Uint(BIT32) => InternalPrimitiveTypes::Uint32,
            Primitive::Uint(BIT64) => InternalPrimitiveTypes::Uint64,
            Primitive::Uint(BIT124) => InternalPrimitiveTypes::Uint128,

            Primitive::UntypedFloat => InternalPrimitiveTypes::UntypedFloat,
            Primitive::Float(BIT16) => InternalPrimitiveTypes::Float16,
            Primitive::Float(BIT32) => InternalPrimitiveTypes::Float32,
            Primitive::Float(BIT64) => InternalPrimitiveTypes::Float64,

            _ => return None
        })
    }
}

fn modifier_compatible(this: TypeModifier, should_be: TypeModifier) -> bool {
    match (this, should_be) {
        (TypeModifier::Mut, TypeModifier::Const)
        | (TypeModifier::Mut, TypeModifier::Literal)
        | (TypeModifier::Const, TypeModifier::Literal) => false,
        _ => true,
    }
}

fn arraykind_compatible(is: ArrayKind, should_be: ArrayKind) -> Option<String> {
    let default_format = |a: ArrayKind, b: ArrayKind| {
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

        (ArrayKind::StackArray(a_num), ArrayKind::StackArray(b_num)) => if a_num != b_num {
            Some(default_format(is, should_be))
        } else {
            None
        },
        (ArrayKind::StackArray(_), ArrayKind::HeapArray) => Some(
            format!("{} (maybe try 'new:[....]')", default_format(is, should_be))
        ),
        _ => Some(default_format(is, should_be)),
    }
}
enum Priority {
    This,
    Other,
}