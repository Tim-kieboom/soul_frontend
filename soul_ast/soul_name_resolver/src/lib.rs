use ast_model::{
    AstModuleStore, AstStore, AstTree, CustomType, EntryKind, Module, NodeId, ScopeInfo,
    block::BlockId,
    declare_store::DeclareStore,
    scope::{ScopeId, ScopeValue},
    statements::{FunctionModifier, StatementId, VarPattern, Variable},
};
use soul_utils::{
    CrateContext, FunctionId, Ident,
    collections::{crate_store::CrateStore, module_store::ModuleStore, vec_map::VecMap},
    ids::IdGenerator,
    span::ModuleId,
};

mod collect;
mod resolve;
mod utils;

pub fn name_resolve(module_store: &mut ModuleStore, ast: &mut AstTree, crate_store: &CrateStore) {
    let root = ast.root;
    let mut resolver = NameResolver::new(ast.root, module_store, ast, crate_store);
    resolver.collect_module(root);
    resolver.resolve_module(root);
}

struct NameResolver<'a> {
    store: &'a AstStore,
    crate_store: &'a CrateStore,
    modules: &'a mut ModuleStore,
    context: &'a mut CrateContext,
    scope_info: &'a mut ScopeInfo,
    declares: &'a mut DeclareStore,
    ast_modules: &'a mut AstModuleStore,

    current: Current,
    scope_ids: VecMap<BlockId, ScopeId>,
    node_generator: IdGenerator<NodeId>,
}

struct Current {
    in_global: bool,
    module: ModuleId,
    function: Option<FunctionId>,
}

impl<'a> NameResolver<'a> {
    pub fn new(
        module: ModuleId,
        modules: &'a mut ModuleStore,
        ast: &'a mut AstTree,
        crate_store: &'a CrateStore,
    ) -> Self {
        Self {
            modules,
            store: &ast.crates.store,
            context: &mut ast.context,
            declares: &mut ast.declares,
            ast_modules: &mut ast.crates.modules,
            scope_info: &mut ast.scope_info,
            crate_store,
            current: Current {
                module,
                in_global: true,
                function: None,
            },
            scope_ids: VecMap::new(),
            node_generator: ast.crates.store.clone_node_generator(),
        }
    }

    fn header_insert_custom_type(
        &mut self,
        id: StatementId,
        custom: CustomType,
    ) -> Option<EntryKind<CustomType>> {
        let is_public = self
            .get_statement(id)
            .map(|it| it.is_public())
            .unwrap_or(false);

        let header = &mut self.ast_modules.get_mut(self.current.module)?.header;
        let entry = match header.get_mut(custom.name().as_str()) {
            Some(val) => val,
            None => header.entry(custom.name().to_string()).or_default(),
        };

        entry.custom_type.replace(EntryKind {
            value: custom,
            is_public,
        })
    }

    fn header_insert_variable(&mut self, variable: &Variable) -> Option<EntryKind<NodeId>> {
        let is_public = variable.is_public;
        self.header_insert_var_pattern(&variable.pattern, is_public)
    }

    fn header_insert_var_pattern(
        &mut self,
        pattern: &VarPattern,
        is_public: bool,
    ) -> Option<EntryKind<NodeId>> {
        match pattern {
            VarPattern::Discard => {}
            VarPattern::Simple { binding, .. } => {
                let module = self.ast_modules.get_mut(self.current.module)?;
                Self::header_insert_binding(module, binding.ident.as_str(), binding.id, is_public);
            }
            VarPattern::Tuple(tuple) => {
                for element in &tuple.elements {
                    self.header_insert_var_pattern(element, is_public)?;
                }
            }
            VarPattern::NamedTuple(named) => {
                let module = self.ast_modules.get_mut(self.current.module)?;
                for field in &named.fields {
                    if let Some(binding) = &field.binding {
                        Self::header_insert_binding(
                            module,
                            binding.ident.as_str(),
                            binding.id,
                            is_public,
                        );
                    }
                }
            }
            VarPattern::Constructor(ctor) => {
                let module = self.ast_modules.get_mut(self.current.module)?;
                for field in &ctor.fields {
                    if let Some(binding) = &field.binding {
                        Self::header_insert_binding(
                            module,
                            binding.ident.as_str(),
                            binding.id,
                            is_public,
                        );
                    }
                }
            }
        }
        Some(EntryKind {
            value: NodeId::ERROR,
            is_public,
        })
    }

    fn header_insert_binding(module: &mut Module, name: &str, id: NodeId, is_public: bool) {
        let entry = match module.header.get_mut(name) {
            Some(val) => val,
            None => module.header.entry(name.to_string()).or_default(),
        };
        entry.variable.replace(EntryKind {
            value: id,
            is_public,
        });
    }

    fn header_insert_function_id(
        &mut self,
        function_id: FunctionId,
    ) -> Option<EntryKind<FunctionId>> {
        let function = self.store.functions.get(function_id)?;
        let signature = &function.signature();
        let is_public = signature.modifier.contains(FunctionModifier::PUBLIC);
        let header = &mut self.ast_modules.get_mut(self.current.module)?.header;
        let entry = match header.get_mut(signature.name.as_str()) {
            Some(val) => val,
            None => header.entry(signature.name.to_string()).or_default(),
        };

        entry.function.replace(EntryKind {
            value: signature.id,
            is_public,
        })
    }

    fn insert_function_alias(&mut self, name: &Ident, id: FunctionId) -> bool {
        if self
            .scope_info
            .scopes
            .flat_lookup_function(name.as_str(), self.current.module)
            .is_some()
        {
            return false;
        }

        self.current_scope_mut().insert_function(name.as_str(), id);
        true
    }

    fn insert_variable_alias(&mut self, name: &Ident, id: NodeId) -> bool {
        if self
            .scope_info
            .scopes
            .flat_lookup_value(name.as_str(), ScopeValue::Variable, self.current.module)
            .is_some()
        {
            return false;
        }

        self.current_scope_mut()
            .insert_value(name.as_str(), ScopeValue::Variable, id);

        true
    }
}
