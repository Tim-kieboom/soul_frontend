use std::fmt::Write;

use soul_utils::soul_names::TypeWrapper;

use crate::{
    ast::{ArrayType, SoulType, TypeKind, syntax_display::DisplayKind},
    syntax_display::SyntaxDisplay,
};

impl SyntaxDisplay for SoulType {
    fn display(&self, kind: &DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, false);
        sb
    }

    fn inner_display(&self, sb: &mut String, _kind: &DisplayKind, _tab: usize, _is_last: bool) {
        if let Some(modifier) = &self.modifier {
            sb.push_str(modifier.as_str());
            sb.push(' ');
        }
        self.kind.inner_display(sb);
    }
}

impl TypeKind {
    pub const fn display_variant(&self) -> &'static str {
        match self {
            TypeKind::None => "None",
            TypeKind::Type => "Type",
            TypeKind::Array(_) => "array",
            TypeKind::Pointer(_) => "pointer",
            TypeKind::Optional(_) => "optional",
            TypeKind::Reference(_) => "reference",
            TypeKind::Primitive(internal_primitive_types) => internal_primitive_types.as_str(),
        }
    }

    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb);
        sb
    }

    pub fn inner_display(&self, sb: &mut String) {
        let kind = &DisplayKind::Parser;
        match self {
            TypeKind::Type => sb.push_str("Type"),
            TypeKind::Reference(r) => {
                let ref_str = if r.mutable {
                    TypeWrapper::MutRef.as_str()
                } else {
                    TypeWrapper::ConstRef.as_str()
                };
                _ = write!(sb, "{}{}", ref_str, r.inner.display(kind));
            }
            TypeKind::Pointer(inner) => _ = write!(sb, "*{}", inner.display(kind)),
            TypeKind::Optional(inner) => _ = write!(sb, "?{}", inner.display(kind)),
            TypeKind::Array(array) => _ = array.inner_display(sb),
            TypeKind::None => sb.push_str("none"),
            TypeKind::Primitive(internal_primitive_types) => {
                sb.push_str(
                    internal_primitive_types.as_str()
                )
            }
        }
    }
}

impl ArrayType {
    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb);
        sb
    }

    pub fn inner_display(&self, sb: &mut String) {
        match self.kind {
            crate::ast::ArrayKind::HeapArray => sb.push_str("[*]"),
            crate::ast::ArrayKind::MutSlice => sb.push_str("[&]"),
            crate::ast::ArrayKind::ConstSlice => sb.push_str("[@]"),
            crate::ast::ArrayKind::StackArray(number) => sb.push_str(&format!("[{number}]")),
        }
        self.of_type.inner_display(sb, &DisplayKind::Parser, 0, false);
    }
}
