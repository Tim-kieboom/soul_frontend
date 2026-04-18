use ast::{
    AstContext, AstModuleStore, DeclareStore, EntryKind, Function, Struct, Variable,
    meta_data::AstMetadata,
    scope::{NodeId, ScopeValue},
};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdGenerator},
    sementic_level::{CompilerContext, SementicFault},
    soul_error_internal,
    span::{ModuleId, Span},
};

mod check_name;
mod collect;
mod resolve;

pub fn name_resolve(
    module_id: ModuleId,
    context: &mut CompilerContext,
    ast_context: &mut AstContext,
) {
    let mut resolver = NameResolver::new(module_id, context, ast_context);

    resolver.collect_module(module_id);
    resolver.resolve_modules(module_id);
}

struct Current {
    in_global: bool,
    module: ModuleId,
    function: Option<FunctionId>,
}

struct NameResolver<'a> {
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

    fn header_insert_function(&mut self, function: &Function) -> Option<EntryKind<FunctionId>> {
        
        let signature = &function.signature.node;
        let is_public = self.is_name_public(signature.name.as_str());
        let header = &mut self.modules[self.current.module].header;
        let entry = match header.get_mut(signature.name.as_str()) {
            Some(val) => val,
            None => header.entry(signature.name.to_string()).or_default(),
        };

        entry.function.replace(EntryKind {
            value: signature.id?,
            is_public,
        })
    }

    fn header_insert_variable(&mut self, variable: &Variable) -> Option<EntryKind<NodeId>> {

        let is_public = self.is_name_public(variable.name.as_str());
        let header = &mut self.modules[self.current.module].header;
        let entry = match header.get_mut(variable.name.as_str()) {
            Some(val) => val,
            None => header.entry(variable.name.to_string()).or_default(),
        };

        entry.variable.replace(EntryKind {
            value: variable.node_id?,
            is_public,
        })
    }

    fn header_insert_struct(&mut self, obj: Struct) -> Option<EntryKind<Struct>> {

        let is_public = self.is_name_public(obj.name.as_str());
        let header = &mut self.modules[self.current.module].header;
        let entry = match header.get_mut(obj.name.as_str()) {
            Some(val) => val,
            None => header.entry(obj.name.to_string()).or_default(),
        };

        entry.struct_type.replace(EntryKind {
            value: obj,
            is_public,
        })
    }

    fn resolve_struct(
        context: &mut CompilerContext,
        store: &mut DeclareStore,
        current: &Current,
        obj: &Struct,
    ) {
        let id = match obj.id {
            Some(val) => val,
            None => {
                Self::static_log_error(
                    context,
                    soul_error_internal!(
                        format!("Struct: '{}' node_id is None", obj.name.as_str()),
                        None
                    ),
                );
                return;
            }
        };

        store.try_insert_struct(id, obj, current.module);
    }

    fn log_error(&mut self, error: SoulError) {
        self.context.faults.push(SementicFault::error(error));
    }

    fn static_log_error(context: &mut CompilerContext, error: SoulError) {
        context.faults.push(SementicFault::error(error));
    }

    fn check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValue::Variable)
    }

    fn lookup_module(&mut self, name: &str) -> Option<ast::scope::ScopeModuleEntry> {
        self.info.scopes.lookup_module(name)
    }

    fn lookup_module_function(
        &mut self,
        module_name: &str,
        function_name: &str,
        span: Span,
    ) -> Option<FunctionId> {
        let module_entry = self.info.scopes.lookup_module(module_name)?;
        let module_id = module_entry.module_id;

        if let Some(resolved_name) = self.resolve_alias(module_name, function_name) {
            return self.info.scopes.lookup_function(&resolved_name);
        }

        debug_assert!(self.modules.contains(module_id));

        let header = &self.modules.get(module_id)?.header;
        let entry = header.get(function_name)?.function?;
        if !entry.is_public {
            self.log_error(SoulError::new(
                format!("'{function_name}' is private"),
                SoulErrorKind::InvalidModuleAccess,
                Some(span),
            ));
        }

        Some(entry.value)
    }

    fn lookup_module_variable(
        &mut self,
        module_name: &str,
        variable_name: &str,
        span: Span,
    ) -> Option<NodeId> {
        let module_entry = self.info.scopes.lookup_module(module_name)?;
        let module_id = module_entry.module_id;

        if let Some(resolved_name) = self.resolve_alias(module_name, variable_name) {
            return self.flat_check_variable(&Ident::new(resolved_name, span));
        }

        debug_assert!(self.modules.contains(module_id));

        let header = &self.modules.get(module_id)?.header;
        let entry = header.get(variable_name)?.variable?;
        if !entry.is_public {
            self.log_error(SoulError::new(
                format!("'{variable_name}' is private"),
                SoulErrorKind::InvalidModuleAccess,
                Some(span),
            ));
        }

        Some(entry.value)
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

    fn flat_check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValue::Variable)
    }

    fn is_name_public(&self, name: &str) -> bool {
        name
            .chars()
            .next()
            .map(|ch| ch.is_uppercase())
            .unwrap_or(false)
    }
}
