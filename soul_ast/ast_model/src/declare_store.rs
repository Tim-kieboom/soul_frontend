use std::collections::HashMap;

use crate::{
    CustomType, NodeId,
    expression::ExpressionId,
    soul_type::SoulType,
    statements::{Enum, InnerFunctionSignature, Struct, Trait},
};
use soul_utils::{
    FunctionId, TypeModifier, collections::vec_map::VecMap, intrinsics::IntrinsicFunction,
    span::ModuleId,
};

/// A store of all declarations in a module.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DeclareStore {
    /// The main function (entry point), if defined.
    pub main_function: Option<FunctionId>,
    /// All variable resolutions, indexed by their ID.
    variable_resolves: VecMap<NodeId, NodeId>,
    /// All functionCall resolutions, indexed by their ID.
    function_resolves: VecMap<NodeId, FunctionResolve>,
    /// All `intrinsic.*` call resolutions, indexed by their ID.
    intrinsic_resolves: VecMap<NodeId, IntrinsicResolve>,
    /// All structs declarations, indexed by their ID.
    custom_types: VecMap<NodeId, (CustomType, ModuleId)>,
    /// All function declarations, indexed by their ID.
    functions: VecMap<FunctionId, (InnerFunctionSignature, ModuleId)>,
    /// All function declarations, indexed by their ID.
    function_names: HashMap<String, Vec<FunctionId>>,
    /// Variable type information, indexed by node ID.
    variable_type: VecMap<NodeId, (TypeModifier, Option<SoulType>, ModuleId)>,
    /// Resolved type of an expression, indexed by its ID.
    expression_types: VecMap<ExpressionId, SoulType>,
    /// Non-`distinct` `type X := Y` aliases, mapping `X`'s name to its
    /// underlying type `Y`. A `distinct` alias is deliberately not
    /// interchangeable with its underlying type, so it's never registered here.
    type_aliases: HashMap<String, SoulType>,
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
            intrinsic_resolves: VecMap::new(),
            expression_types: VecMap::new(),
            type_aliases: HashMap::new(),
        }
    }

    /// Inserts a function into the store.
    pub fn insert_functions(
        &mut self,
        index: FunctionId,
        function: InnerFunctionSignature,
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
    pub fn get_function(&self, index: FunctionId) -> Option<&(InnerFunctionSignature, ModuleId)> {
        self.functions.get(index)
    }

    /// Retrieves the resolved function-call info for a call-expression node.
    pub fn get_call_resolve(&self, id: NodeId) -> Option<&FunctionResolve> {
        self.function_resolves.get(id)
    }

    /// Retrieves the node ID a variable reference resolves to.
    pub fn get_variable_resolve(&self, id: NodeId) -> Option<NodeId> {
        self.variable_resolves.get(id).copied()
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

    /// Retrieves a struct/enum/trait declaration by its own declaration NodeId.
    pub fn get_custom_type(&self, index: NodeId) -> Option<&(CustomType, ModuleId)> {
        self.custom_types.get(index)
    }

    /// Records that a variable reference node resolves to the declaration
    /// node `resolved`. Returns the previously stored resolution, if any.
    pub fn insert_variable_resolve(&mut self, node_id: NodeId, resolved: NodeId) -> Option<NodeId> {
        self.variable_resolves.insert(node_id, resolved)
    }

    /// Records the resolved function-call info for a call-expression node.
    /// Returns the previously stored resolution, if any.
    pub fn insert_function_resolve(
        &mut self,
        node_id: NodeId,
        function: FunctionResolve,
    ) -> Option<FunctionResolve> {
        self.function_resolves.insert(node_id, function)
    }

    /// Retrieves the resolved function-call info for a call-expression node.
    pub fn get_function_resolve(&self, node_id: NodeId) -> Option<FunctionResolve> {
        self.function_resolves.get(node_id).copied()
    }

    /// Records the resolved intrinsic-call info for a call-expression node.
    /// Returns the previously stored resolution, if any.
    pub fn insert_intrinsic_resolve(
        &mut self,
        node_id: NodeId,
        intrinsic: IntrinsicResolve,
    ) -> Option<IntrinsicResolve> {
        self.intrinsic_resolves.insert(node_id, intrinsic)
    }

    /// Retrieves the resolved intrinsic-call info for a call-expression node.
    pub fn get_intrinsic_resolve(&self, node_id: NodeId) -> Option<IntrinsicResolve> {
        self.intrinsic_resolves.get(node_id).copied()
    }

    /// Finds a function by name and optional owner type (for method
    /// resolution), also returning the module it was declared in.
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

    /// Finds a function declared by `name` within a specific module.
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

    /// Returns all function declarations in the store, indexed by their ID.
    pub fn functions(&self) -> &VecMap<FunctionId, (InnerFunctionSignature, ModuleId)> {
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

    /// Gets the resolved type of an expression by its ID.
    pub fn get_expression_type(&self, index: ExpressionId) -> Option<&SoulType> {
        self.expression_types.get(index)
    }

    /// Sets the resolved type of an expression.
    pub fn insert_expression_type(&mut self, index: ExpressionId, ty: SoulType) {
        self.expression_types.insert(index, ty);
    }

    /// Registers a non-`distinct` `type X := Y` alias's underlying type.
    pub fn insert_type_alias(&mut self, name: impl Into<String>, underlying: SoulType) {
        self.type_aliases.insert(name.into(), underlying);
    }

    /// The underlying type of a non-`distinct` alias by name, if `name` is one.
    pub fn get_type_alias(&self, name: &str) -> Option<&SoulType> {
        self.type_aliases.get(name)
    }
}

/// The resolved target of a function call.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct FunctionResolve {
    /// The ID of the resolved function.
    pub id: FunctionId,
    /// Whether the call is deferred (executed at scope exit).
    pub is_defer: bool,
    /// Whether the callee expression should be ignored when generating code
    /// for the call (e.g. because it was only used for method resolution).
    pub ignore_callee: bool,
}

/// The resolved target of an `intrinsic.*` call.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct IntrinsicResolve {
    /// Which intrinsic function the call resolves to.
    pub kind: IntrinsicFunction,
}
