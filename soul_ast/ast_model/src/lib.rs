use soul_utils::{vec_map::VecMap};

mod ast;
pub mod scope;
pub mod meta_data;
pub mod syntax_display;
pub use ast::*;

use crate::{meta_data::AstMetadata, scope::NodeId};

/// The root of an Abstract Syntax Tree representing a parsed Soul program.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AbstractSyntaxTree {
    /// The root block containing all top-level statements.
    pub root: ast::Block,
}

pub struct ParseResponse {
    pub store: DeclareStore,
    pub meta_data: AstMetadata,
    pub tree: AbstractSyntaxTree,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclareStore {
    functions: VecMap<NodeId, FunctionSignature>,
    variable_type: VecMap<NodeId, VarTypeKind>,
}
impl DeclareStore {
    pub const fn new() -> Self {
        Self {
            functions: VecMap::const_default(),
            variable_type: VecMap::const_default(),
        }
    }

    pub fn get_function(&self, index: NodeId) -> Option<&FunctionSignature> {
        self.functions.get(index)
    }

    pub fn insert_functions(&mut self, index: NodeId, function: FunctionSignature) {
        self.functions.insert(index, function);
    }

    pub fn get_variable_type(&self, index: NodeId) -> Option<&VarTypeKind> {
        self.variable_type.get(index)
    }

    pub fn insert_variable_type(&mut self, index: NodeId, ty: VarTypeKind) {
        self.variable_type.insert(index, ty);
    }
}
