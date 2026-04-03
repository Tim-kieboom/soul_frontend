use hir::TypeId;
use soul_utils::{soul_names::PrimitiveTypes, symbool_kind::SymbolKind};

use crate::{ThirType, ThirTypeKind, ThirTypesMap};

pub trait DisplayThirType {
    fn write_display_no_spaces(&self, types: &ThirTypesMap, sb: &mut String) -> std::fmt::Result;
    fn write_display(&self, types: &ThirTypesMap, sb: &mut String) -> std::fmt::Result;
    fn display(&self, types: &ThirTypesMap) -> String;
}

impl DisplayThirType for ThirType {
    fn write_display_no_spaces(&self, types: &ThirTypesMap, sb: &mut String) -> std::fmt::Result {
        if let Some(modifier) = self.modifier {
            sb.push_str(modifier.as_str());
            sb.push(' ');
        }

        self.kind.write_display_no_spaces(types, sb)
    }

    fn write_display(&self, types: &ThirTypesMap, sb: &mut String) -> std::fmt::Result {
        if let Some(modifier) = self.modifier {
            sb.push_str(modifier.as_str());
            sb.push(' ');
        }

        self.kind.write_display(types, sb)
    }

    fn display(&self, types: &ThirTypesMap) -> String {
        let mut sb = "".to_string();
        self.write_display(types, &mut sb).expect("no fmt error");
        sb
    }
}
impl DisplayThirType for ThirTypeKind {
    fn write_display_no_spaces(&self, types: &ThirTypesMap, sb: &mut String) -> std::fmt::Result {
        self.write_display(types, sb)
    }

    fn write_display(&self, types: &ThirTypesMap, sb: &mut String) -> std::fmt::Result {
        use std::fmt::Write;
        const CONST_REF_STR: &str = SymbolKind::ConstRef.as_str();
        const OPTIONAL_STR: &str = SymbolKind::Question.as_str();
        const POINTER_STR: &str = SymbolKind::Star.as_str();
        const MUT_REF_STR: &str = SymbolKind::And.as_str();

        match self {
            ThirTypeKind::None => write!(sb, "{}", PrimitiveTypes::None.as_str()),
            ThirTypeKind::Type => write!(sb, "type"),
            ThirTypeKind::Generic(id) => match types.id_to_generic(*id) {
                None => write!(sb, "{:?}", id),
                Some(name) => {
                    sb.push_str(name);
                    Ok(())
                }
            },
            ThirTypeKind::Struct(id) => {
                match types.id_to_struct(*id) {
                    Some(s) => sb.push_str(&s.name),
                    None => sb.push_str("<error>"),
                }
                Ok(())
            }
            ThirTypeKind::Primitive(prim) => write!(sb, "{}", prim.as_str()),
            ThirTypeKind::Array { element, kind } => {
                kind.write_to_string(sb)?;
                write_display_from_id(types, *element, sb)
            }
            ThirTypeKind::Ref { of_type, mutable } => {
                let ref_str = match *mutable {
                    true => MUT_REF_STR,
                    false => CONST_REF_STR,
                };

                sb.push_str(ref_str);
                write_display_from_id(types, *of_type, sb)
            }
            ThirTypeKind::Pointer(type_id) => {
                sb.push_str(POINTER_STR);
                write_display_from_id(types, *type_id, sb)
            }
            ThirTypeKind::Optional(type_id) => {
                sb.push_str(OPTIONAL_STR);
                write_display_from_id(types, *type_id, sb)
            }
            ThirTypeKind::Error => write!(sb, "<error>"),
        }
    }

    fn display(&self, types: &ThirTypesMap) -> String {
        let mut sb = "".to_string();
        self.write_display(types, &mut sb).expect("no fmt error");
        sb
    }
}
impl ThirTypeKind {
    pub const fn display_variant(&self) -> &'static str {
        match self {
            ThirTypeKind::Type => "type",
            ThirTypeKind::None => "none",
            ThirTypeKind::Error => "<error>",
            ThirTypeKind::Ref { .. } => "<ref>",
            ThirTypeKind::Array { .. } => "<array>",
            ThirTypeKind::Pointer(_) => "<pointer>",
            ThirTypeKind::Generic(_) => "<generic>",
            ThirTypeKind::Optional(_) => "<optional>",
            ThirTypeKind::Struct(_) => "<struct>",
            ThirTypeKind::Primitive(primitive) => primitive.as_str(),
        }
    }
}

const ERROR: ThirType = ThirType {
    kind: ThirTypeKind::Error,
    generics: vec![],
    modifier: None,
};

fn write_display_from_id(types: &ThirTypesMap, ty: TypeId, sb: &mut String) -> std::fmt::Result {
    match inner_write_display_from_id(types, ty, sb) {
        Some(val) => val,
        None => ERROR.write_display(types, sb),
    }
}

fn inner_write_display_from_id(
    types: &ThirTypesMap,
    ty: TypeId,
    sb: &mut String,
) -> Option<std::fmt::Result> {
    Some(types.id_to_type(ty)?.write_display(types, sb))
}
