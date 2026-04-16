use ast::{
    AstContext, AstModuleStore, DeclareStore, Function, Struct, Variable, meta_data::AstMetadata, scope::{NodeId, ScopeValue}
};
use soul_utils::{
    Ident,
    error::SoulError,
    ids::{FunctionId, IdGenerator},
    sementic_level::{CompilerContext, SementicFault},
    span::ModuleId,
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

struct Current {
    in_global: bool,
    module: ModuleId,
    function: Option<FunctionId>,
}

struct NameResolver<'a> {
    root: ModuleId,
    current: Current,
    info: &'a mut AstMetadata,
    store: &'a mut DeclareStore,
    modules: &'a mut AstModuleStore,
    context: &'a mut CompilerContext,
    node_generator: IdGenerator<NodeId>,
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
            context,
            current: Current {
                in_global: true,
                module,
                function: None,
            },
            node_generator: IdGenerator::new(),
            store: &mut ast_context.store,
            info: &mut ast_context.meta_data,
            modules: &mut ast_context.modules,
            function_generator: &mut ast_context.function_generators,
        }
    }

    fn header_insert_function(&mut self, function: &Function) -> Option<FunctionId> {
        
        let signature = &function.signature.node;
        let header = &mut self.modules[self.current.module].header;

        let entry = match header.get_mut(signature.name.as_str()) {
            Some(val) => val,
            None => header.entry(signature.name.to_string()).or_default(),
        };

        entry.function.replace(signature.id?)
    }

    fn header_insert_variable(&mut self, variable: &Variable) -> Option<NodeId> {
        
        let header = &mut self.modules[self.current.module].header;

        let entry = match header.get_mut(variable.name.as_str()) {
            Some(val) => val,
            None => header.entry(variable.name.to_string()).or_default(),
        };

        entry.variable.replace(variable.node_id?)
    }

    fn header_insert_struct(&mut self, obj: &Struct) -> Option<NodeId> {
        
        let header = &mut self.modules[self.current.module].header;

        let entry = match header.get_mut(obj.name.as_str()) {
            Some(val) => val,
            None => header.entry(obj.name.to_string()).or_default(),
        };

        entry.variable.replace(obj.id?)
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
        _module_id: ModuleId,
        function_name: &str,
    ) -> Option<FunctionId> {
        let module_entry = self.info.scopes.lookup_module(module_name)?;
        let module_id = module_entry.module_id;
        
        if let Some(resolved_name) = self.resolve_alias(module_name, function_name) {
            self.info.scopes.lookup_function(&resolved_name)
        } else {
            self.store.find_function_in_module(function_name, module_id)
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
            ast::ImportKind::This => {
                return false;
            }
            ast::ImportKind::Items { this, this_alias: _, items: _ } => {
                if *this {
                    return true;
                }
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
