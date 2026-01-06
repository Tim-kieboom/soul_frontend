use soul_utils::soul_names::TypeWrapper;

use crate::{
    SoulType, TypeKind,
    syntax_display::{DisplayKind, SyntaxDisplay},
};

impl SyntaxDisplay for SoulType {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, false);
        sb
    }

    fn inner_display(&self, sb: &mut String, _kind: DisplayKind, _tab: usize, _is_last: bool) {
        sb.push_str(self.modifier.as_str());
        sb.push(' ');
        sb.push_str(&self.kind.display());
    }
}

impl TypeKind {
    pub const fn display_variant(&self) -> &'static str {
        match self {
            TypeKind::None => "None",
            TypeKind::Type => "Type",
            TypeKind::Array(_) => "array",
            TypeKind::Tuple(_) => "tuple",
            TypeKind::Stub { .. } => "Stub",
            TypeKind::Pointer(_) => "pointer",
            TypeKind::Optional(_) => "optional",
            TypeKind::Generic { .. } => "generic",
            TypeKind::Reference(_) => "reference",
            TypeKind::NamedTuple(_) => "namedTuple",
            TypeKind::Primitive(internal_primitive_types) => internal_primitive_types.as_str(),
        }
    }

    /// Returns a string representation of the type kind
    pub fn display(&self) -> String {
        let kind = DisplayKind::Parser;
        match self {
            TypeKind::Type => "Type".to_string(),
            TypeKind::Array(a) => {
                let inner = a.of_type.display(kind);
                match a.size {
                    Some(num) => format!("[{}]{}", num, inner),
                    None => format!("[]{}", inner),
                }
            }
            TypeKind::Tuple(tuple) => {
                let elems: Vec<String> = tuple.iter().map(|t| t.display(kind)).collect();
                format!("({})", elems.join(", "))
            }
            TypeKind::NamedTuple(map) => {
                let elems: Vec<String> = map
                    .iter()
                    .map(|(k, v, _)| format!("{}: {}", k.as_str(), v.display(kind)))
                    .collect();
                format!("{{{}}}", elems.join(", "))
            }
            TypeKind::Generic { node_id, .. } => node_id.display(),
            TypeKind::Reference(r) => {
                let ref_str = if r.mutable {
                    TypeWrapper::MutRef.as_str()
                } else {
                    TypeWrapper::ConstRef.as_str()
                };
                format!("{}{}", ref_str, r.inner.display(kind))
            }
            TypeKind::Pointer(inner) => format!("*{}", inner.display(kind)),
            TypeKind::Optional(inner) => format!("{}?", inner.display(kind)),
            TypeKind::Stub { ident, .. } => ident.as_str().to_string(),
            TypeKind::None => "none".to_string(),
            TypeKind::Primitive(internal_primitive_types) => {
                internal_primitive_types.as_str().to_string()
            }
        }
    }
}
