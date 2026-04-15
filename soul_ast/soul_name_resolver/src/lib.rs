use ast::{
    AstContext, AstModuleStore, DeclareStore, meta_data::AstMetadata, scope::{NodeId, ScopeValue}
};
use soul_utils::{
    Ident,
    error::SoulError,
    ids::{FunctionId, IdGenerator},
    sementic_level::{CompilerContext, SementicFault},
    span::{ModuleId},
};

mod check_name;
mod collect;
mod resolve;

pub fn name_resolve(module_id: ModuleId, context: &mut CompilerContext, ast_context: &mut AstContext) {
    let mut resolver = NameResolver::new(
        module_id,
        context,
        ast_context,
    );

    resolver.collect_module(module_id);
    resolver.resolve_modules(module_id);
}

struct NameResolver<'a> {
    root: ModuleId,
    current_module: ModuleId,
    info: &'a mut AstMetadata,
    store: &'a mut DeclareStore,
    modules: &'a mut AstModuleStore,
    context: &'a mut CompilerContext,
    node_generator: IdGenerator<NodeId>,
    current_function: Option<FunctionId>,
    function_generator: &'a mut IdGenerator<FunctionId>,
}
impl<'a> NameResolver<'a> {
    fn new(
        module: ModuleId,
        context: &'a mut CompilerContext,
        ast_context: &'a mut AstContext,
    ) -> Self {
        Self {
            root: module,
            current_module: module,
            context,
            current_function: None,
            node_generator: IdGenerator::new(),
            store: &mut ast_context.store,
            info: &mut ast_context.meta_data,
            modules: &mut ast_context.modules,
            function_generator: &mut ast_context.function_generators,
        }
    }

    fn log_error(&mut self, error: SoulError) {
        self.context.faults.push(SementicFault::error(error));
    }

    fn check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValue::Variable)
    }

    fn lookup_module(&mut self, name: &str) -> Option<ast::scope::ScopeModuleEntry> {
        self.info.scopes.lookup_module(name)
    }

    fn lookup_module_function(
        &self,
        module_name: &str,
        module_id: ModuleId,
        function_name: &str,
    ) -> Option<FunctionId> {
        let entry_key = format!("{}::{}", module_name, function_name);
        self.info
            .scopes
            .lookup_function(&Ident::new_dummy(&entry_key, module_id))
    }

    fn flat_check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValue::Variable)
    }

}
