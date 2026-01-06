mod ast;
mod scope;

pub mod syntax_display;
pub use ast::*;

/// The root of an Abstract Syntax Tree representing a parsed Soul program.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AbstractSyntaxTree {
    /// The root block containing all top-level statements.
    pub root: Block,
}
