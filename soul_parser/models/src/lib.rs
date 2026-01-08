pub mod ast;
pub mod scope;
pub mod meta_data;
pub mod syntax_display;

/// The root of an Abstract Syntax Tree representing a parsed Soul program.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AbstractSyntaxTree {
    /// The root block containing all top-level statements.
    pub root: ast::Block,
}

pub struct ParseResponse {
    pub tree: AbstractSyntaxTree,
    pub meta_data: crate::meta_data::AstMetadata,
}
