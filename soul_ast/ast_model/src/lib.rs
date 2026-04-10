use soul_utils::{
    ids::{FunctionId, IdGenerator},
    vec_map::VecMap,
};

mod ast;
pub mod meta_data;
pub mod scope;
pub mod syntax_display;
pub use ast::*;

use crate::{meta_data::AstMetadata, scope::NodeId};

/// The root of an Abstract Syntax Tree representing a parsed Soul program.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AbstractSyntaxTree {
    /// The root block containing all top-level statements.
    pub root: ast::Block,
}

pub struct AstResponse {
    pub store: DeclareStore,
    pub meta_data: AstMetadata,
    pub tree: AbstractSyntaxTree,
    pub function_generators: IdGenerator<FunctionId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclareStore {
    pub main_function: Option<FunctionId>,
    functions: VecMap<FunctionId, FunctionSignature>,
    variable_type: VecMap<NodeId, VarTypeKind>,
    #[serde(default)]
    variable_owner_hint: VecMap<NodeId, TypeKind>,
}
impl DeclareStore {
    pub const fn new() -> Self {
        Self {
            main_function: None,
            functions: VecMap::const_default(),
            variable_type: VecMap::const_default(),
            variable_owner_hint: VecMap::const_default(),
        }
    }

    pub fn get_function(&self, index: FunctionId) -> Option<&FunctionSignature> {
        self.functions.get(index)
    }

    pub fn find_function_by_name_and_owner_kind(
        &self,
        name: &str,
        owner_kind: Option<&TypeKind>,
    ) -> Option<FunctionId> {
        self.functions.entries().find_map(|(id, signature)| {
            if signature.name.as_str() != name {
                return None;
            }

            match owner_kind {
                Some(owner) if &signature.methode_type.kind == owner => Some(id),
                None if matches!(signature.methode_type.kind, TypeKind::None) => Some(id),
                _ => None,
            }
        })
    }

    pub fn insert_functions(&mut self, index: FunctionId, function: FunctionSignature) {
        self.functions.insert(index, function);
    }

    pub fn get_variable_type(&self, index: NodeId) -> Option<&VarTypeKind> {
        self.variable_type.get(index)
    }

    pub fn insert_variable_type(&mut self, index: NodeId, ty: VarTypeKind) {
        self.variable_type.insert(index, ty);
    }

    pub fn get_variable_owner_hint(&self, index: NodeId) -> Option<&TypeKind> {
        self.variable_owner_hint.get(index)
    }

    pub fn insert_variable_owner_hint(&mut self, index: NodeId, kind: TypeKind) {
        self.variable_owner_hint.insert(index, kind);
    }
}
