//! Abstract Syntax Tree (AST) representation for the Soul language.
//!
//! This module contains all the AST node types representing parsed Soul code,
//! including expressions, statements, types, functions, and control structures.

pub mod block;
pub mod conditionals;
pub mod enum_like;
pub mod expression;
pub mod expression_groups;
pub mod function;
pub mod literal;
pub mod objects;
pub mod operator;
pub mod soul_type;
pub mod spanned;
pub mod statment;
pub mod syntax_display;

pub use crate::abstract_syntax_tree::block::*;
pub use crate::abstract_syntax_tree::conditionals::*;
pub use crate::abstract_syntax_tree::enum_like::*;
pub use crate::abstract_syntax_tree::expression::*;
pub use crate::abstract_syntax_tree::expression_groups::*;
pub use crate::abstract_syntax_tree::function::*;
pub use crate::abstract_syntax_tree::literal::*;
pub use crate::abstract_syntax_tree::objects::*;
pub use crate::abstract_syntax_tree::operator::*;
pub use crate::abstract_syntax_tree::soul_type::*;
pub use crate::abstract_syntax_tree::spanned::*;
pub use crate::abstract_syntax_tree::statment::*;
pub use crate::abstract_syntax_tree::syntax_display::*;

/// The root of an Abstract Syntax Tree representing a parsed Soul program.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AbstractSyntaxTree {
    /// The root block containing all top-level statements.
    pub root: Block,
}
