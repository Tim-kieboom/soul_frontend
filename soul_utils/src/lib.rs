pub mod span;
pub mod error;
pub mod soul_names;
pub mod char_colors; 
pub mod symbool_kind;
pub mod define_enums;
pub mod sementic_level;
pub mod soul_import_path;

pub type Ident = crate::span::Spanned<String>;
impl Ident {
    pub fn as_str(&self) -> &str {
        &self.node
    }
}