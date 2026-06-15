use std::collections::HashMap;

use crate::{NodeId, soul_type::SoulType, statements::FunctionSignature};
use soul_utils::{FunctionId, TypeModifier, collections::vec_map::VecMap, span::ModuleId};

/// A store of all declarations in a module.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DeclareStore {
    /// The main function (entry point), if defined.
    main_function: Option<FunctionId>,
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
            functions: VecMap::const_default(),
            function_names: HashMap::default(),
            variable_type: VecMap::const_default(),
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
    pub fn find_function(&self, name: &str, owner_kind: Option<&SoulType>) -> Option<FunctionId> {
        self.find_function_with_module(name, owner_kind)
            .map(|(id, _)| id)
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
