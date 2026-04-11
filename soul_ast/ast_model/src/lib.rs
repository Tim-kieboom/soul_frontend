use soul_utils::{
    ids::{FunctionId, IdGenerator},
    vec_map::VecMap,
};

mod ast;
pub mod meta_data;
pub mod scope;
pub use ast::*;

use crate::{meta_data::AstMetadata, scope::NodeId};

/// The root of an Abstract Syntax Tree representing a parsed Soul program.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AbstractSyntaxTree {
    /// The root block containing all top-level statements.
    pub root: ast::Block,
}

/// The result of parsing a Soul source file into an AST.
pub struct AstResponse {
    /// The declaration store containing all functions and variables.
    pub store: DeclareStore,
    /// Metadata associated with the AST nodes.
    pub meta_data: AstMetadata,
    /// The abstract syntax tree.
    pub tree: AbstractSyntaxTree,
    /// ID generator for functions.
    pub function_generators: IdGenerator<FunctionId>,
}

/// A store of all declarations in a module.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclareStore {
    /// The main function (entry point), if defined.
    pub main_function: Option<FunctionId>,
    /// All function declarations, indexed by their ID.
    functions: VecMap<FunctionId, FunctionSignature>,
    /// Variable type information, indexed by node ID.
    variable_type: VecMap<NodeId, VarTypeKind>,
    /// Variable owner hints (for method resolution), indexed by node ID.
    variable_owner_hint: VecMap<NodeId, TypeKind>,
}
impl DeclareStore {
    /// Creates a new empty declaration store.
    pub const fn new() -> Self {
        Self {
            main_function: None,
            functions: VecMap::const_default(),
            variable_type: VecMap::const_default(),
            variable_owner_hint: VecMap::const_default(),
        }
    }

    /// Retrieves a function by its ID.
    pub fn get_function(&self, index: FunctionId) -> Option<&FunctionSignature> {
        self.functions.get(index)
    }

    /// Finds a function by name and optional owner type (for method resolution).
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

    /// Inserts a function into the store.
    pub fn insert_functions(&mut self, index: FunctionId, function: FunctionSignature) {
        self.functions.insert(index, function);
    }

    /// Gets the type of a variable by its node ID.
    pub fn get_variable_type(&self, index: NodeId) -> Option<&VarTypeKind> {
        self.variable_type.get(index)
    }

    /// Sets the type of a variable.
    pub fn insert_variable_type(&mut self, index: NodeId, ty: VarTypeKind) {
        self.variable_type.insert(index, ty);
    }

    /// Gets the owner hint for a variable.
    pub fn get_variable_owner_hint(&self, index: NodeId) -> Option<&TypeKind> {
        self.variable_owner_hint.get(index)
    }

    /// Sets the owner hint for a variable.
    pub fn insert_variable_owner_hint(&mut self, index: NodeId, kind: TypeKind) {
        self.variable_owner_hint.insert(index, kind);
    }
}
