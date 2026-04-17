use ast::{ArrayKind, Literal};
use soul_utils::{
    ids::IdAlloc,
    soul_names::{PrimitiveTypes, TypeModifier},
    symbool_kind::SymbolKind,
    vec_map::VecMapIndex,
    Ident,
};

use crate::{FieldId, GenericId, InferTypeId, InferTypesMap, StructId, TypeId, TypesMap};

pub type HirType = InnerType<HirTypeKind>;
pub type InferType = InnerType<InferTypeId>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ComplexLiteral {
    Basic(Literal),
    Struct {
        struct_id: StructId,
        struct_type: TypeId,
        values: Vec<(ComplexLiteral, TypeId)>,
        all_fields_const: bool,
    },
}
impl ComplexLiteral {
    /// for example `Struct{mut field: i32}` should be alloced becouse you can change value of field
    pub fn is_mutable(&self) -> bool {
        match self {
            ComplexLiteral::Basic(_) => false,
            ComplexLiteral::Struct {
                all_fields_const, ..
            } => !*all_fields_const,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InnerType<Kind> {
    pub kind: Kind,
    pub generics: Vec<TypeId>,
    pub modifier: Option<TypeModifier>,
}
impl<T> InnerType<T> {
    pub const fn new(kind: T) -> Self {
        Self {
            kind,
            generics: vec![],
            modifier: None,
        }
    }

    pub const fn apply_modfier(mut self, modifier: Option<TypeModifier>) -> Self {
        self.modifier = modifier;
        self
    }

    pub fn apply_generics(mut self, generics: Vec<TypeId>) -> Self {
        self.generics = generics;
        self
    }
}
impl InferType {
    pub fn is_mutable(&self) -> bool {
        self.modifier == Some(TypeModifier::Mut)
    }
    pub fn is_modifier_none(&self) -> bool {
        self.modifier.is_none()
    }
}
impl HirType {
    pub const fn index_type() -> Self {
        Self::primitive_type(PrimitiveTypes::Uint)
    }

    pub const fn none_type() -> Self {
        Self::new(HirTypeKind::None)
    }

    pub const fn bool_type() -> Self {
        Self::primitive_type(PrimitiveTypes::Boolean)
    }

    pub const fn primitive_type(prim: PrimitiveTypes) -> Self {
        Self::new(HirTypeKind::Primitive(prim))
    }

    pub const fn pointer_type(inner: LazyTypeId) -> Self {
        Self::new(HirTypeKind::Pointer(inner))
    }

    pub const fn generic_type(id: GenericId) -> Self {
        Self::new(HirTypeKind::Generic(id))
    }

    pub const fn error_type() -> Self {
        Self::new(HirTypeKind::Error)
    }

    pub fn is_mutable(&self) -> bool {
        self.modifier == Some(TypeModifier::Mut)
    }

    pub fn is_modifier_none(&self) -> bool {
        self.modifier.is_none()
    }

    pub const fn is_untyped_interger_type(&self) -> bool {
        self.kind.is_untyped_interger()
    }

    pub const fn is_any_int_type(&self) -> bool {
        if let HirTypeKind::Primitive(prim) = self.kind {
            prim.is_signed_interger()
        } else {
            false
        }
    }

    pub const fn is_any_uint_type(&self) -> bool {
        if let HirTypeKind::Primitive(prim) = self.kind {
            prim.is_unsigned_interger()
        } else {
            false
        }
    }

    pub const fn is_boolean_type(&self) -> bool {
        matches!(self.kind, HirTypeKind::Primitive(PrimitiveTypes::Boolean))
    }

    pub const fn is_pointer(&self) -> bool {
        matches!(self.kind, HirTypeKind::Pointer(_))
    }

    pub const fn is_float_type(&self) -> bool {
        if let HirTypeKind::Primitive(prim) = self.kind {
            prim.is_float()
        } else {
            false
        }
    }

    pub const fn is_numeric_type(&self) -> bool {
        self.is_float_type() || self.is_any_uint_type() || self.is_any_int_type()
    }

    pub const fn is_non_float_numeric_type(&self) -> bool {
        self.is_any_uint_type() || self.is_any_int_type()
    }

    pub const fn is_error(&self) -> bool {
        matches!(self.kind, HirTypeKind::Error)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirTypeKind {
    None,
    Type,
    Primitive(PrimitiveTypes),
    Array {
        element: LazyTypeId,
        kind: ArrayKind,
    },
    Ref {
        of_type: LazyTypeId,
        mutable: bool,
    },
    Pointer(LazyTypeId),
    Optional(LazyTypeId),
    Generic(GenericId),
    Struct(StructId),

    Error,
}
impl HirTypeKind {
    pub const fn is_untyped_interger(&self) -> bool {
        match self {
            HirTypeKind::Primitive(prim) => prim.is_unsigned_interger(),
            _ => false,
        }
    }

    pub const fn is_error(&self) -> bool {
        matches!(self, HirTypeKind::Error)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CreatedTypes {
    Struct(StructId),
}
impl CreatedTypes {
    pub fn to_hir_kind(self) -> HirTypeKind {
        match self {
            CreatedTypes::Struct(struct_id) => HirTypeKind::Struct(struct_id),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LazyTypeId {
    Known(TypeId),
    Infer(InferTypeId),
}
impl LazyTypeId {
    pub fn error() -> Self {
        Self::Known(TypeId::error())
    }
}
impl TypeId {
    pub fn to_lazy(self) -> LazyTypeId {
        LazyTypeId::Known(self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub name: Ident,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub struct_id: StructId,
    pub id: FieldId,
    /// The name of the field (for readability, debugging, codegen).
    pub name: String,
    pub ty: LazyTypeId,
}

pub trait DisplayType {
    fn display(&self, types: &TypesMap, infers: &InferTypesMap) -> String;
    fn write_display(
        &self,
        types: &TypesMap,
        infers: &InferTypesMap,
        sb: &mut String,
    ) -> std::fmt::Result;
}

impl<K: DisplayType> DisplayType for InnerType<K> {
    fn write_display(
        &self,
        types: &TypesMap,
        infers: &InferTypesMap,
        sb: &mut String,
    ) -> std::fmt::Result {
        if let Some(modifier) = self.modifier {
            sb.push_str(modifier.as_str());
            sb.push(' ');
        }

        self.kind.write_display(types, infers, sb)
    }

    fn display(&self, types: &TypesMap, infers: &InferTypesMap) -> String {
        let mut sb = "".to_string();
        self.write_display(types, infers, &mut sb)
            .expect("no fmt error");
        sb
    }
}
impl DisplayType for InferTypeId {
    fn write_display(
        &self,
        _types: &TypesMap,
        _infers: &InferTypesMap,
        sb: &mut String,
    ) -> std::fmt::Result {
        use std::fmt::Write;
        write!(sb, "<infer_{}>", self.index())
    }

    fn display(&self, types: &TypesMap, infers: &InferTypesMap) -> String {
        let mut sb = "".to_string();
        self.write_display(types, infers, &mut sb)
            .expect("no fmt error");
        sb
    }
}
impl DisplayType for HirTypeKind {
    fn write_display(
        &self,
        types: &TypesMap,
        infers: &InferTypesMap,
        sb: &mut String,
    ) -> std::fmt::Result {
        use std::fmt::Write;
        const CONST_REF_STR: &str = SymbolKind::ConstRef.as_str();
        const OPTIONAL_STR: &str = SymbolKind::Question.as_str();
        const POINTER_STR: &str = SymbolKind::Star.as_str();
        const MUT_REF_STR: &str = SymbolKind::And.as_str();

        match self {
            HirTypeKind::None => write!(sb, "{}", PrimitiveTypes::None.as_str()),
            HirTypeKind::Type => write!(sb, "type"),
            HirTypeKind::Generic(id) => match types.id_to_generic(*id) {
                None => write!(sb, "{:?}", id),
                Some(name) => {
                    sb.push_str(name);
                    Ok(())
                }
            },
            HirTypeKind::Struct(id) => {
                match types.id_to_struct(*id) {
                    Some(val) => sb.push_str(val.name.as_str()),
                    None => sb.push_str("<error>"),
                }
                Ok(())
            }
            HirTypeKind::Primitive(prim) => write!(sb, "{}", prim.as_str()),
            HirTypeKind::Array { element, kind } => {
                kind.write_to_string(sb)?;
                write_display_from_id(types, infers, *element, sb)
            }
            HirTypeKind::Ref { of_type, mutable } => {
                let ref_str = match *mutable {
                    true => MUT_REF_STR,
                    false => CONST_REF_STR,
                };

                sb.push_str(ref_str);
                write_display_from_id(types, infers, *of_type, sb)
            }
            HirTypeKind::Pointer(type_id) => {
                sb.push_str(POINTER_STR);
                write_display_from_id(types, infers, *type_id, sb)
            }
            HirTypeKind::Optional(type_id) => {
                sb.push_str(OPTIONAL_STR);
                write_display_from_id(types, infers, *type_id, sb)
            }
            HirTypeKind::Error => write!(sb, "<error>"),
        }
    }

    fn display(&self, types: &TypesMap, infers: &InferTypesMap) -> String {
        let mut sb = "".to_string();
        self.write_display(types, infers, &mut sb)
            .expect("no fmt error");
        sb
    }
}
impl HirTypeKind {
    pub const fn display_variant(&self) -> &'static str {
        match self {
            HirTypeKind::Type => "type",
            HirTypeKind::None => "none",
            HirTypeKind::Error => "<error>",
            HirTypeKind::Ref { .. } => "<ref>",
            HirTypeKind::Array { .. } => "<array>",
            HirTypeKind::Pointer(_) => "<pointer>",
            HirTypeKind::Generic(_) => "<generic>",
            HirTypeKind::Optional(_) => "<optional>",
            HirTypeKind::Struct(_) => "<struct>",
            HirTypeKind::Primitive(primitive) => primitive.as_str(),
        }
    }
}

fn write_display_from_id(
    types: &TypesMap,
    infers: &InferTypesMap,
    ty: LazyTypeId,
    sb: &mut String,
) -> std::fmt::Result {
    match inner_write_display_from_id(types, infers, ty, sb) {
        Some(val) => val,
        None => HirType::error_type().write_display(types, infers, sb),
    }
}

fn inner_write_display_from_id(
    types: &TypesMap,
    infers: &InferTypesMap,
    ty: LazyTypeId,
    sb: &mut String,
) -> Option<std::fmt::Result> {
    Some(match ty {
        LazyTypeId::Known(type_id) => types.id_to_type(type_id)?.write_display(types, infers, sb),
        LazyTypeId::Infer(infer_type_id) => infers
            .get_infer(infer_type_id)?
            .write_display(types, infers, sb),
    })
}
