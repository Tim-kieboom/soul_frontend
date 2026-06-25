use ast_model::{
    AstModuleStore, AstStore, AstTree, CustomType, EntryKind, NodeId, ScopeInfo,
    block::BlockId,
    declare_store::DeclareStore,
    scope::{ScopeId, ScopeValue},
    statements::{StatementId, Variable},
};
use soul_utils::{
    CrateContext, FunctionId, Ident,
    collections::{module_store::ModuleStore, vec_map::VecMap},
    ids::IdGenerator,
    span::ModuleId,
};

mod collect;
mod resolve;
mod utils;

pub fn name_resolve<'a>(module_store: &mut ModuleStore, ast: &mut AstTree) {
    let root = ast.root;
    let mut resolver = NameResolver::new(ast.root, module_store, ast);
    resolver.collect_module(root);
    resolver.resolve_module(root);
}

struct NameResolver<'a> {
    store: &'a AstStore,
    modules: &'a mut ModuleStore,
    context: &'a mut CrateContext,
    scope_info: &'a mut ScopeInfo,
    ast_modules: &'a mut AstModuleStore,

    current: Current,
    declares: DeclareStore,
    scope_ids: VecMap<BlockId, ScopeId>,
    node_generator: IdGenerator<NodeId>,
}

struct Current {
    in_global: bool,
    module: ModuleId,
    function: Option<FunctionId>,
}

impl<'a> NameResolver<'a> {
    pub fn new(module: ModuleId, modules: &'a mut ModuleStore, ast: &'a mut AstTree) -> Self {
        Self {
            modules,
            store: &ast.store,
            context: &mut ast.context,
            ast_modules: &mut ast.modules,
            declares: DeclareStore::new(),
            scope_info: &mut ast.scope_info,
            current: Current {
                module,
                in_global: true,
                function: None,
            },
            scope_ids: VecMap::new(),
            node_generator: ast.store.clone_node_generator(),
        }
    }

    fn header_insert_custom_type(
        &mut self,
        id: StatementId,
        custom: CustomType,
    ) -> Option<EntryKind<CustomType>> {
        let is_public = self
            .store
            .statements
            .get(id)
            .map(|s| s.is_public())
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
        let header = &mut self.ast_modules.get_mut(self.current.module)?.header;
        let entry = match header.get_mut(variable.name.as_str()) {
            Some(val) => val,
            None => header.entry(variable.name.to_string()).or_default(),
        };

        entry.variable.replace(EntryKind {
            value: variable.id,
            is_public,
        })
    }

    fn header_insert_function_id(
        &mut self,
        function_id: FunctionId,
    ) -> Option<EntryKind<FunctionId>> {
        let function = self.store.functions.get(function_id)?;
        let signature = &function.signature().value;
        let is_public = signature.is_public;
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
            .flat_lookup_value(name, ScopeValue::Variable, self.current.module)
            .is_some()
        {
            return false;
        }

        self.current_scope_mut()
            .insert_value(name.as_str(), ScopeValue::Variable, id);

        true
    }
}
