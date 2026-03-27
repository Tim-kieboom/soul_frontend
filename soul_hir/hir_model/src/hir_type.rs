use ast::ArrayKind;
use soul_utils::{
    Ident, ids::IdAlloc, soul_names::{PrimitiveTypes, TypeModifier}, symbool_kind::SymbolKind, vec_map::VecMapIndex,
};

use crate::{FieldId, GenericId, InferTypeId, InferTypesMap, StructId, TypeId, TypesMap};

pub type HirType = InnerType<HirTypeKind, PossibleTypeId>;
pub type InferType = InnerType<InferTypeId, PossibleTypeId>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InnerType<Kind, TyId> {
    pub kind: Kind,
    pub generics: Vec<TyId>,
    pub modifier: Option<TypeModifier>,
}
impl<T, V> InnerType<T, V> {
    pub const fn new(kind: T) -> Self {
        Self { kind, generics: vec![], modifier: None }
    }
}
impl HirType {
    pub const fn index_type() -> Self {
        Self::new(HirTypeKind::Primitive(PrimitiveTypes::Uint))
    }

    pub const fn none_type() -> Self {
        Self::new(HirTypeKind::Primitive(PrimitiveTypes::None))
    }

    pub const fn error_type() -> Self {
        Self::new(HirTypeKind::Error)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirTypeKind {
    None,
    Type,
    Primitive(PrimitiveTypes),
    Array {
        element: PossibleTypeId,
        kind: ArrayKind,
    },
    Ref {
        of_type: PossibleTypeId,
        mutable: bool,
    },
    Pointer(PossibleTypeId),
    Optional(PossibleTypeId),
    Generic(GenericId),
    Struct(StructId),

    Error,
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
pub enum PossibleTypeId {
    Known(TypeId),
    Infer(InferTypeId),
}
impl PossibleTypeId {
    pub fn error() -> Self {
        Self::Known(TypeId::error())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub name: Ident,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub id: FieldId,
    /// The name of the field (for readability, debugging, codegen).
    pub name: String,
    pub ty: PossibleTypeId,
}

pub trait DisplayType {
    fn display(&self, types: &TypesMap, infers: &InferTypesMap) -> String;
    fn write_display(&self, types: &TypesMap, infers: &InferTypesMap, sb: &mut String) -> std::fmt::Result;
}

impl<K: DisplayType, I> DisplayType for InnerType<K, I> {
    fn write_display(&self, types: &TypesMap, infers: &InferTypesMap, sb: &mut String) -> std::fmt::Result {
        if let Some(modifier) = self.modifier {
            sb.push_str(modifier.as_str());
            sb.push(' ');
        }

        self.kind.write_display(types, infers, sb)
    }
    
    fn display(&self, types: &TypesMap, infers: &InferTypesMap) -> String {
        let mut sb = "".to_string();
        self.write_display(types, infers, &mut sb).expect("no fmt error");
        sb
    }
}
impl DisplayType for InferTypeId {
    fn write_display(&self, _types: &TypesMap, _infers: &InferTypesMap, sb: &mut String) -> std::fmt::Result {
        use std::fmt::Write;
        write!(sb, "<infer_{}>", self.index())
    }

    fn display(&self, types: &TypesMap, infers: &InferTypesMap) -> String {
        let mut sb = "".to_string();
        self.write_display(types, infers, &mut sb).expect("no fmt error");
        sb
    }
}
impl DisplayType for HirTypeKind {
    fn write_display(&self, types: &TypesMap, infers: &InferTypesMap, sb: &mut String) -> std::fmt::Result {
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
            },
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
        self.write_display(types, infers, &mut sb).expect("no fmt error");
        sb
    }
    
}
fn write_display_from_id(types: &TypesMap, infers: &InferTypesMap, ty: PossibleTypeId, sb: &mut String) -> std::fmt::Result {
    match inner_write_display_from_id(types, infers, ty, sb) {
        Some(val) => val,
        None => HirType::error_type().write_display(types, infers, sb),
    }
}

fn inner_write_display_from_id(types: &TypesMap, infers: &InferTypesMap, ty: PossibleTypeId, sb: &mut String) -> Option<std::fmt::Result> {
    Some(match ty {
        PossibleTypeId::Known(type_id) => types.id_to_type(type_id)?.write_display(types, infers, sb),
        PossibleTypeId::Infer(infer_type_id) => infers.get_infer(infer_type_id)?.write_display(types, infers, sb),
    })
}
