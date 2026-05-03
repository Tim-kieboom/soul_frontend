use std::collections::HashMap;

use soul_utils::{
    Ident, ids::{FunctionId, IdGenerator}, span::ModuleId, vec_map::VecMap, vec_set::VecSet
};

mod ast;
pub mod meta_data;
pub mod scope;
pub use ast::*;

use crate::{meta_data::AstMetadata, scope::NodeId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AbtractSyntaxTree {
    /// The declaration store containing all functions and variables.
    pub store: DeclareStore,
    /// Metadata associated with the AST nodes.
    pub meta_data: AstMetadata,
    /// all the modules of the project
    pub modules: AstModuleStore,
    /// ID generator for functions.
    pub function_generators: IdGenerator<FunctionId>,
}
impl AbtractSyntaxTree {
    pub fn new(module: ModuleId) -> Self {
        Self {
            store: DeclareStore::new(),
            meta_data: AstMetadata::new(module),
            modules: AstModuleStore::new(),
            function_generators: IdGenerator::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub global: Block,
    pub parent: Option<ModuleId>,
    pub modules: VecSet<ModuleId>,
    pub visibility: Visibility,
    pub header: HashMap<String, HeaderEntry>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HeaderEntry {
    pub variable: Option<EntryKind<NodeId>>,
    pub struct_type: Option<EntryKind<CustomType>>,
    pub function: Option<EntryKind<FunctionId>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CustomType {
    Struct(Struct),
    Enum(Enum),
}
impl CustomType {
    pub fn id(&self) -> Option<NodeId> {
        match self {
            CustomType::Struct(obj) => obj.id,
            CustomType::Enum(obj) => obj.id,
        }
    }

    pub fn name(&self) -> &Ident {
        match self {
            CustomType::Struct(obj) => &obj.name,
            CustomType::Enum(obj) => &obj.name,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EntryKind<T> {
    pub value: T,
    pub is_public: bool,
}
impl<T: Copy> Copy for EntryKind<T> {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AstModuleStore {
    map: VecMap<ModuleId, Module>,
}
impl AstModuleStore {
    pub const fn new() -> Self {
        Self {
            map: VecMap::const_default(),
        }
    }

    pub fn insert(&mut self, id: ModuleId, module: Module) -> Option<Module> {
        self.map.insert(id, module)
    }

    pub fn keys(&self) -> impl Iterator<Item = ModuleId> {
        self.map.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Module> {
        self.map.values()
    }

    pub fn get(&self, id: ModuleId) -> Option<&Module> {
        self.map.get(id)
    }

    pub fn contains(&self, id: ModuleId) -> bool {
        self.get(id).is_some()
    }

    pub fn get_mut(&mut self, id: ModuleId) -> Option<&mut Module> {
        self.map.get_mut(id)
    }
}
impl std::ops::Index<ModuleId> for AstModuleStore {
    type Output = Module;

    fn index(&self, index: ModuleId) -> &Self::Output {
        &self.map[index]
    }
}
impl std::ops::IndexMut<ModuleId> for AstModuleStore {
    fn index_mut(&mut self, index: ModuleId) -> &mut Self::Output {
        &mut self.map[index]
    }
}

/// A store of all declarations in a module.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeclareStore {
    /// The main function (entry point), if defined.
    pub main_function: Option<FunctionId>,
    /// All structs declarations, indexed by their ID.
    custom_types: VecMap<NodeId, (CustomType, ModuleId)>,
    /// All function declarations, indexed by their ID.
    functions: VecMap<FunctionId, (FunctionSignature, ModuleId)>,
    /// Variable type information, indexed by node ID.
    variable_type: VecMap<NodeId, (VarTypeKind, ModuleId)>,
    /// Variable owner hints (for method resolution), indexed by node ID.
    variable_owner_hint: VecMap<NodeId, (TypeKind, ModuleId)>,
}
impl DeclareStore {
    /// Creates a new empty declaration store.
    pub const fn new() -> Self {
        Self {
            main_function: None,
            custom_types: VecMap::const_default(),
            functions: VecMap::const_default(),
            variable_type: VecMap::const_default(),
            variable_owner_hint: VecMap::const_default(),
        }
    }

    pub fn iter_structs(&self) -> impl Iterator<Item = &(CustomType, ModuleId)> {
        self.custom_types.values()
    }

    pub fn find_struct_by_name(&self, name: &str) -> Option<(&Struct, ModuleId)> {
        self.custom_types
            .values()
            .filter_map(|(ty, id)| match ty {
                CustomType::Struct(obj) => Some((obj, *id)),
                _ => None
            }).find(|(obj, _)| obj.name.as_str() == name)
    }

    pub fn find_enum_by_name(&self, name: &str) -> Option<(&Enum, ModuleId)> {
        self.custom_types
            .values()
            .filter_map(|(ty, id)| match ty {
                CustomType::Enum(obj) => Some((obj, *id)),
                _ => None
            }).find(|(obj, _)| obj.name.as_str() == name)
    }

    /// Retrieves a function by its ID.
    pub fn get_function(&self, index: FunctionId) -> Option<&(FunctionSignature, ModuleId)> {
        self.functions.get(index)
    }

    /// Finds a function by name and optional owner type (for method resolution).
    pub fn find_function(&self, name: &str, owner_kind: Option<&TypeKind>) -> Option<FunctionId> {
        self.find_function_with_module(name, owner_kind)
            .map(|(id, _)| id)
    }

    pub fn find_function_with_module(
        &self,
        name: &str,
        owner_kind: Option<&TypeKind>,
    ) -> Option<(FunctionId, ModuleId)> {
        self.functions
            .entries()
            .find_map(|(id, (signature, module))| {
                if signature.name.as_str() != name {
                    return None;
                }

                match owner_kind {
                    Some(owner) if &signature.methode_type.kind == owner => Some((id, *module)),
                    None if matches!(signature.methode_type.kind, TypeKind::None) => {
                        Some((id, *module))
                    }
                    _ => None,
                }
            })
    }

    /// Inserts a function into the store.
    pub fn insert_functions(
        &mut self,
        index: FunctionId,
        function: FunctionSignature,
        module: ModuleId,
    ) {
        self.functions.insert(index, (function, module));
    }

    pub fn find_function_in_module(&self, name: &str, module: ModuleId) -> Option<FunctionId> {
        self.functions.entries().find_map(|(id, (sig, mod_id))| {
            if sig.name.as_str() == name && *mod_id == module {
                Some(id)
            } else {
                None
            }
        })
    }

    pub fn all_functions(
        &self,
    ) -> impl Iterator<Item = (FunctionId, &(FunctionSignature, ModuleId))> {
        self.functions.entries()
    }

    /// try Inserts a struct into the store.
    pub fn try_insert_struct(&mut self, index: NodeId, obj: &Struct, module: ModuleId) {
        if self.custom_types.contains(index) {
            return;
        }

        self.custom_types.insert(index, (CustomType::Struct(obj.clone()), module));
    }

    pub fn try_insert_enum(&mut self, index: NodeId, obj: &Enum, module: ModuleId) {
        if self.custom_types.contains(index) {
            return;
        }

        self.custom_types.insert(index, (CustomType::Enum(obj.clone()), module));
    }

    /// Gets the type of a struct by its node ID.
    pub fn get_struct(&self, index: NodeId) -> Option<(&Struct, ModuleId)> {
        let (CustomType::Struct(obj), module_id) = self.custom_types.get(index)? else {
            return None
        };

        Some((obj, *module_id))
    }

    /// Gets the type of a struct by its node ID.
    pub fn get_enum(&self, index: NodeId) -> Option<(&Enum, ModuleId)> {
        let (CustomType::Enum(obj), module_id) = self.custom_types.get(index)? else {
            return None
        };

        Some((obj, *module_id))
    }

    /// Gets the type of a variable by its node ID.
    pub fn get_variable_type(&self, index: NodeId) -> Option<&(VarTypeKind, ModuleId)> {
        self.variable_type.get(index)
    }

    /// Sets the type of a variable.
    pub fn insert_variable_type(&mut self, index: NodeId, ty: VarTypeKind, module: ModuleId) {
        self.variable_type.insert(index, (ty, module));
    }

    /// Gets the owner hint for a variable.
    pub fn get_variable_owner_hint(&self, index: NodeId) -> Option<&(TypeKind, ModuleId)> {
        self.variable_owner_hint.get(index)
    }

    /// Sets the owner hint for a variable.
    pub fn insert_variable_owner_hint(&mut self, index: NodeId, kind: TypeKind, module: ModuleId) {
        self.variable_owner_hint.insert(index, (kind, module));
    }
}
impl Default for DeclareStore {
    fn default() -> Self {
        Self::new()
    }
}
