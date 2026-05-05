pub use crate::crate_store::{Crate, CrateContext, CrateExports, CrateStore, TypeId};
pub use crate::ids::IdAlloc;
pub use crate::soul_manifest::SoulToml;
pub use crate::span::{CrateId, ModuleId, Span};

pub mod bimap;
pub mod char_colors;
pub mod compile_options;
pub mod crate_store;
pub mod define_enums;
pub mod error;
pub mod ids;
pub mod precedence;
pub mod print_breakpoint;
pub mod sementic_level;
pub mod soul_import_path;
pub mod soul_manifest;
pub mod soul_names;
pub mod span;
pub mod symbool_kind;
pub mod try_result;
pub mod vec_map;
pub mod vec_set;

#[cfg(test)]
mod vec_map_tests;
#[cfg(test)]
mod vec_set_tests;

#[derive(Clone, PartialEq)]
pub enum StringLiteral {
    Normal(String),
    CStr(String),
}
impl std::fmt::Debug for StringLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringLiteral::Normal(str) => f.write_fmt(format_args!("{:?}", str)),
            StringLiteral::CStr(str) => f.write_fmt(format_args!("c{:?}", str)),
        }
    }
}
impl std::fmt::Display for StringLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringLiteral::Normal(str) => f.write_str(str),
            StringLiteral::CStr(str) => f.write_str(str),
        }
    }
}
impl StringLiteral {
    pub fn display_len(&self) -> usize {
        match self {
            StringLiteral::CStr(str) | StringLiteral::Normal(str) => str.len(),
        }
    }

    pub fn to_tag(&self) -> Option<StringTag> {
        match self {
            StringLiteral::Normal(_) => None,
            StringLiteral::CStr(_) => Some(StringTag::CStr),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringTag {
    CStr, // c
}
impl StringTag {
    pub fn from_char(ch: char) -> Option<Self> {
        match ch {
            'c' => Some(Self::CStr),
            _ => None,
        }
    }
}

pub type Ident = crate::span::Spanned<String>;
impl Ident {
    pub fn as_str(&self) -> &str {
        &self.node
    }

    pub fn to_string(&self) -> String {
        self.node.clone()
    }

    pub fn new_owned(name: String, module: ModuleId) -> Self {
        crate::span::Spanned::new(name, Span::default(module))
    }

    pub fn new_dummy(name: &str, module: ModuleId) -> Self {
        Self::new_owned(name.to_string(), module)
    }
}
