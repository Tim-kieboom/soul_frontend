//! Abstract Syntax Tree (AST) representation for the Soul language.
//!
//! This module contains all the AST node types representing parsed Soul code,
//! including expressions, statements, types, functions, and control structures.

pub mod block;
pub mod spanned;
pub mod objects;
pub mod literal;
pub mod statment;
pub mod function;
pub mod operator;
pub mod enum_like;
pub mod soul_type;
pub mod expression;
pub mod conditionals;
pub mod expression_groups;

use crate::abstract_syntax_tree::block::Block;

/// The root of an Abstract Syntax Tree representing a parsed Soul program.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AbstractSyntaxTree {
    /// The root block containing all top-level statements.
    pub root: Block,
}
