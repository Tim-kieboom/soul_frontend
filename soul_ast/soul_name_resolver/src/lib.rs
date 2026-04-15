use ast::{
    AstContext, AstModuleStore, DeclareStore, meta_data::AstMetadata, scope::{NodeId, ScopeValue}
};
use soul_utils::{
    Ident,
    error::SoulError,
    ids::{FunctionId, IdGenerator},
    sementic_level::{CompilerContext, SementicFault},
    span::ModuleId, vec_map::VecMapIndex,
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
        if let Some(resolved_name) = self.resolve_alias(module_name, function_name) {
            let entry_key = format!("{}::{}", module_id.index(), resolved_name);
            self.info
                .scopes
                .lookup_function(&entry_key)
        } else {
            let entry_key = format!("{}::{}", module_id.index(), function_name);
            self.info
                .scopes
                .lookup_function(&entry_key)
        }
    }

    fn resolve_alias(&self, module_name: &str, function_name: &str) -> Option<String> {
        let module_entry = match self.info.scopes.lookup_module(module_name) {
            Some(entry) => entry,
            None => return None,
        };

        for item in &module_entry.imported_items {
            match item {
                ast::ImportItem::Alias { name, alias } => {
                    if alias.as_str() == function_name {
                        return Some(name.to_string());
                    }
                }
                ast::ImportItem::Normal(ident) => {
                    if ident.as_str() == function_name {
                        return Some(ident.to_string());
                    }
                }
            }
        }
        None
    }

    fn is_item_imported(
        &self,
        module_name: &str,
        function_name: &str,
    ) -> bool {
        let module_entry = match self.info.scopes.lookup_module(module_name) {
            Some(entry) => entry,
            None => return true,
        };

        match &module_entry.import_kind {
            ast::ImportKind::Glob | ast::ImportKind::Alias(_) => {
                return true;
            }
            ast::ImportKind::This | ast::ImportKind::Items{..} => {}
        }

        if module_entry.imported_items.is_empty() {
            match &module_entry.import_kind {
                ast::ImportKind::This => return false,
                _ => return true,
            }
        }

        for item in &module_entry.imported_items {
            match item {
                ast::ImportItem::Normal(ident) => {
                    if ident.as_str() == function_name {
                        return true;
                    }
                }
                ast::ImportItem::Alias { name: _, alias } => {
                    if alias.as_str() == function_name {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn flat_check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValue::Variable)
    }

}
