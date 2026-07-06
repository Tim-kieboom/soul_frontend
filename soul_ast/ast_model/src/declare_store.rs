use std::collections::HashMap;

use crate::{CustomType, NodeId, soul_type::SoulType, statements::{Enum, FunctionSignature, Struct, Trait}};
use soul_utils::{FunctionId, TypeModifier, collections::vec_map::VecMap, span::ModuleId};


/// A store of all declarations in a module.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DeclareStore {
    /// The main function (entry point), if defined.
    pub main_function: Option<FunctionId>,
    /// All variable resolutions, indexed by their ID.
    variable_resolves: VecMap<NodeId, NodeId>,
    /// All functionCall resolutions, indexed by their ID.
    function_resolves: VecMap<NodeId, FunctionResolve>,
    /// All structs declarations, indexed by their ID.
    custom_types: VecMap<NodeId, (CustomType, ModuleId)>,
    /// All function declarations, indexed by their ID.
    functions: VecMap<FunctionId, (FunctionSignature, ModuleId)>,
    /// All function declarations, indexed by their ID.
    function_names: HashMap<String, Vec<FunctionId>>,
    /// Variable type information, indexed by node ID.
    variable_type: VecMap<NodeId, (TypeModifier, Option<SoulType>, ModuleId)>,
}
impl DeclareStore {
    /// Creates a new empty declaration store.
    pub fn new() -> Self {
        Self {
            main_function: None,
            variable_resolves: VecMap::new(),
            functions: VecMap::new(),
            custom_types: VecMap::new(),
            variable_type: VecMap::new(),
            function_names: HashMap::new(),
            function_resolves: VecMap::new(),
        }
    }

    /// Inserts a function into the store.
    pub fn insert_functions(
        &mut self,
        index: FunctionId,
        function: FunctionSignature,
        module: ModuleId,
    ) {
        if let Some(entries) = self.function_names.get_mut(function.name.as_str()) {
            entries.push(index);
        } else {
            self.function_names
                .insert(function.name.to_string(), vec![index]);
        }
        self.functions.insert(index, (function, module));
    }

    /// Retrieves a function by its ID.
    pub fn get_function(&self, index: FunctionId) -> Option<&(FunctionSignature, ModuleId)> {
        self.functions.get(index)
    }

    /// Finds a function by name and optional owner type (for method resolution).
    pub fn find_function(&self, name: &str, owner_type: Option<&SoulType>) -> Option<FunctionId> {
        self.find_function_with_module(name, owner_type)
            .map(|(id, _)| id)
    }

    /// try Inserts a enum into the store.
    pub fn try_insert_enum(&mut self, index: NodeId, obj: &Enum, module: ModuleId) {
        if self.custom_types.contains(index) {
            return;
        }

        self.custom_types
            .insert(index, (CustomType::Enum(obj.clone()), module));
    }

    /// try Inserts a trait into the store.
    pub fn try_insert_trait(&mut self, index: NodeId, obj: &Trait, module: ModuleId) {
        if self.custom_types.contains(index) {
            return;
        }

        self.custom_types
            .insert(index, (CustomType::Trait(obj.clone()), module));
    }

    /// try Inserts a struct into the store.
    pub fn try_insert_struct(&mut self, index: NodeId, obj: &Struct, module: ModuleId) {
        if self.custom_types.contains(index) {
            return;
        }

        self.custom_types
            .insert(index, (CustomType::Struct(obj.clone()), module));
    }

    pub fn insert_variable_resolve(&mut self, node_id: NodeId, resolved: NodeId) -> Option<NodeId> {
        self.variable_resolves.insert(node_id, resolved)
    }

    pub fn get_variable_resolve(&self, node_id: NodeId) -> Option<NodeId> {
        self.variable_resolves.get(node_id).copied()
    } 

    pub fn insert_function_resolve(&mut self, node_id: NodeId, function: FunctionResolve) -> Option<FunctionResolve> {
        self.function_resolves.insert(node_id, function)
    }

    pub fn get_function_resolve(&self, node_id: NodeId) -> Option<FunctionResolve> {
        self.function_resolves.get(node_id).copied()
    } 

    pub fn find_function_with_module(
        &self,
        name: &str,
        owner_kind: Option<&SoulType>,
    ) -> Option<(FunctionId, ModuleId)> {
        let functions = self.function_names.get(name)?;
        for id in functions {
            let (signature, module) = &self.functions[*id];
            match owner_kind {
                Some(owner) if &signature.method_type == owner => return Some((*id, *module)),
                None if matches!(signature.method_type, SoulType::None) => {
                    return Some((*id, *module));
                }
                _ => continue,
            }
        }

        None
    }

    pub fn find_function_in_module(&self, name: &str, module: ModuleId) -> Option<FunctionId> {
        let functions = self.function_names.get(name)?;
        for id in functions {
            let (_, module_id) = self.functions[*id];
            if module_id == module {
                return Some(*id);
            }
        }

        None
    }

    pub fn functions(&self) -> &VecMap<FunctionId, (FunctionSignature, ModuleId)> {
        &self.functions
    }

    /// Gets the type of a variable by its node ID.
    pub fn get_variable_type(
        &self,
        index: NodeId,
    ) -> Option<&(TypeModifier, Option<SoulType>, ModuleId)> {
        self.variable_type.get(index)
    }

    /// Sets the type of a variable.
    pub fn insert_variable_type(
        &mut self,
        index: NodeId,
        modifier: TypeModifier,
        ty: Option<SoulType>,
        module: ModuleId,
    ) {
        self.variable_type.insert(index, (modifier, ty, module));
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct FunctionResolve {
    pub id: FunctionId,
    pub is_defer: bool,
    pub ignore_callee: bool,
}

