pub mod block;
pub mod scope;
pub mod spanned;
pub mod statment;
pub mod function;

use crate::steps::abstract_syntax_tree::block::Block;

pub struct AbstractSyntaxTree {
    pub root: Block,
}