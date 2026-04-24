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
