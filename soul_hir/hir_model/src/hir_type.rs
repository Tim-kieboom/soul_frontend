use ast::ArrayKind;
use soul_utils::{soul_names::{InternalPrimitiveTypes, TypeModifier}, symbool_kind::SymbolKind, vec_map::VecMapIndex};
use std::fmt::Write;

use crate::{InferVarId, TypeId, TypedContext};

#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HirType {
    pub kind: HirTypeKind,
    pub modifier: Option<TypeModifier>,
}
impl HirType {

    pub const fn index_ty() -> Self {
        Self { kind: HirTypeKind::Primitive(InternalPrimitiveTypes::Uint), modifier: None }
    }

    pub const fn none_ty() -> Self {
        Self { kind: HirTypeKind::None, modifier: None }
    }

    pub const fn error_ty() -> Self {
        Self { kind: HirTypeKind::Error, modifier: None }
    }

    pub const fn bool_ty() -> Self {
        Self { kind: HirTypeKind::Primitive(InternalPrimitiveTypes::Boolean), modifier: None }
    }

    pub const fn infer_ty(infer_id: InferVarId) -> Self {
        Self { kind: HirTypeKind::Infer(infer_id), modifier: None }
    }

    pub const fn new(kind: HirTypeKind) -> Self {
        Self { kind, modifier: None }
    }

    pub fn with_modifier(mut self, modifier: TypeModifier) -> Self {
        self.modifier = Some(modifier);
        self
    }

    pub fn display(&self, types: &TypedContext) -> String {
        let mut sb = String::new();
        self.write_display(types, &mut sb).expect("no format errors");
        sb
    }

    pub fn write_display(&self, types: &TypedContext, sb: &mut String) -> std::fmt::Result  {
        if let Some(modifier) = self.modifier {
            sb.push_str(modifier.as_str());
            sb.push(' ');
        }

        self.kind.write_display(types, sb)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirTypeKind {
    None,
    Type,
    Primitive(InternalPrimitiveTypes),
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
    Infer(InferVarId),
}
impl HirTypeKind {
    pub fn write_display(&self, types: &TypedContext, sb: &mut String) -> std::fmt::Result {
        const CONST_REF_STR: &str = SymbolKind::ConstRef.as_str();
        const OPTIONAL_STR: &str = SymbolKind::Question.as_str();
        const POINTER_STR: &str = SymbolKind::Star.as_str();
        const MUT_REF_STR: &str = SymbolKind::And.as_str();
        
        match self {
            HirTypeKind::None => write!(sb, "{}", InternalPrimitiveTypes::None.as_str()),
            HirTypeKind::Type => write!(sb, "type"),
            HirTypeKind::Primitive(prim) => write!(sb, "{}", prim.as_str()),
            HirTypeKind::Array { element, kind } => {
                kind.write_to_string(sb)?;
                write_display_from_id(types, *element, sb)
            },
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
            },
            HirTypeKind::Optional(type_id) => {
                sb.push_str(OPTIONAL_STR);
                write_display_from_id(types, *type_id, sb)
            },
            HirTypeKind::Error => write!(sb, "<error>"),
            HirTypeKind::Infer(_) => write!(sb, "<infer>"),
        }
    }

}

fn write_display_from_id(types: &TypedContext, ty: TypeId, sb: &mut String) -> std::fmt::Result {
    let ty = match types.get_type(ty) {
        Some(val) => val,
        None => {
            return write!(
                sb, 
                "/*TypeId({}) not found*/", 
                ty.index()
            )
        }
    };

    ty.write_display(types, sb)
}