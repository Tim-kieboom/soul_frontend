pub mod char_colors;
pub mod define_enums;
pub mod error;
pub mod print_breakpoint;
pub mod sementic_level;
pub mod soul_import_path;
pub mod soul_names;
pub mod span;
pub mod symbool_kind;
pub mod try_result;
pub mod vec_map;
pub mod vec_set;

#[cfg(test)]
mod vec_set_tests;
#[cfg(test)]
mod vec_map_tests;

pub type Ident = crate::span::Spanned<String>;
impl Ident {
    pub fn as_str(&self) -> &str {
        &self.node
    }

    pub fn to_string(&self) -> String {
        self.node.clone()
    }
}
