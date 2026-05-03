use std::path::PathBuf;

use ast::{
    AbtractSyntaxTree, AstModuleStore, CustomType, DeclareStore, EntryKind, Enum, Function, Struct, Variable, meta_data::AstMetadata, scope::{NodeId, ScopeValue}
};
use soul_utils::{
    CrateStore, Ident, crate_store::CrateContext, error::{SoulError, SoulErrorKind}, ids::{FunctionId, IdAlloc, IdGenerator}, sementic_level::{ModuleStore, SementicFault}, soul_error_internal, span::{ModuleId, Span}
};

mod check_name;
mod collect;
mod resolve;

pub fn name_resolve(
    module_id: ModuleId,
    module_store: &mut ModuleStore,
    context: &mut CrateContext,
    ast_context: &mut AbtractSyntaxTree,
    crates: &CrateStore,
    source_folder: PathBuf,
) {
    let mut resolver =
        NameResolver::new(module_id, module_store, context, ast_context, crates, source_folder);

    resolver.collect_module(module_id);
    resolver.resolve_modules(module_id);
}

struct Current {
    in_global: bool,
    module: ModuleId,
    function: Option<FunctionId>,
    source_folder: PathBuf,
    path_stack: Vec<PathBuf>,
}

impl Current {
    fn current_path(&self) -> PathBuf {
        let mut result = self.source_folder.clone();
        for component in &self.path_stack {
            result.push(component);
        }
        result
    }

    fn push_current_path(&mut self, path: PathBuf) {
        self.path_stack.push(path);
    }

    fn pop_current_path(&mut self) {
        self.path_stack.pop();
    }
}

struct NameResolver<'a> {
    current: Current,
    info: &'a mut AstMetadata,
    store: &'a mut DeclareStore,
    modules: &'a mut AstModuleStore,
    module_store: &'a mut ModuleStore,
    context: &'a mut CrateContext,
    node_generator: IdGenerator<NodeId>,
    function_generator: &'a mut IdGenerator<FunctionId>,
    crates: &'a CrateStore,
}
impl<'a> NameResolver<'a> {
    fn new(
        module: ModuleId,
        module_store: &'a mut ModuleStore,
        context: &'a mut CrateContext,
        ast_context: &'a mut AbtractSyntaxTree,
        crates: &'a CrateStore,
        source_folder: PathBuf,
    ) -> Self {
        Self {
            crates,
            context,
            module_store,
            current: Current {
                in_global: true,
                module,
                function: None,
                source_folder,
                path_stack: Vec::new(),
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

    fn header_insert_enum(&mut self, obj: Enum) -> Option<EntryKind<CustomType>> {
        let is_public = self.is_name_public(obj.name.as_str());
        let header = &mut self.modules[self.current.module].header;
        let entry = match header.get_mut(obj.name.as_str()) {
            Some(val) => val,
            None => header.entry(obj.name.to_string()).or_default(),
        };

        entry.struct_type.replace(EntryKind {
            value: ast::CustomType::Enum(obj),
            is_public,
        })
    }

    fn header_insert_struct(&mut self, obj: Struct) -> Option<EntryKind<CustomType>> {
        let is_public = self.is_name_public(obj.name.as_str());
        let header = &mut self.modules[self.current.module].header;
        let entry = match header.get_mut(obj.name.as_str()) {
            Some(val) => val,
            None => header.entry(obj.name.to_string()).or_default(),
        };

        entry.struct_type.replace(EntryKind {
            value: ast::CustomType::Struct(obj),
            is_public,
        })
    }

    fn resolve_enum(
        faults: &mut CrateContext,
        store: &mut DeclareStore,
        current: &Current,
        obj: &Enum,
    ) {
        let id = match obj.id {
            Some(val) => val,
            None => {
                Self::static_log_error(
                    faults,
                    soul_error_internal!(
                        format!("Enum: '{}' node_id is None", obj.name.as_str()),
                        None
                    ),
                );
                return;
            }
        };

        store.try_insert_enum(id, obj, current.module);
    }

    fn resolve_struct(
        faults: &mut CrateContext,
        store: &mut DeclareStore,
        current: &Current,
        obj: &Struct,
    ) {
        let id = match obj.id {
            Some(val) => val,
            None => {
                Self::static_log_error(
                    faults,
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

    fn static_log_error(context: &mut CrateContext, error: SoulError) {
        context.faults.push(SementicFault::error(error));
    }

    fn check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info
            .scopes
            .lookup_value(name, ScopeValue::Variable, self.current.module)
    }

    fn lookup_module(&mut self, name: &str) -> Option<ast::scope::ScopeModuleEntry> {
        self.info.scopes.lookup_module(name, self.current.module)
    }

    fn lookup_module_function(
        &mut self,
        module_name: &str,
        function_name: &str,
        span: Span,
    ) -> Option<FunctionId> {
        let module_entry = self
            .info
            .scopes
            .lookup_module(module_name, self.current.module)?;

        if let Some(resolved_name) = self.resolve_alias(module_name, function_name) {
            return self
                .info
                .scopes
                .lookup_function(&resolved_name, self.current.module);
        }

        if let Some(crate_name) = &module_entry.crate_name {
            
            let full_name = format!("{}.{}", module_name, function_name);
            if self.crates.resolve_function(crate_name, &full_name).is_some() {
                // Return error FunctionId but set external_ref in caller
                // The caller will set external_ref on the FunctionCall
                return Some(FunctionId::error());
            }

            self.log_error(SoulError::new(
                format!("function '{function_name}' not found in external module '{module_name}'"),
                SoulErrorKind::NotFoundInScope,
                Some(span),
            ));
            return None;
        }

        let module_id = module_entry.module_id;
        debug_assert_ne!(module_id, ModuleId::error());
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
        let module_entry = self
            .info
            .scopes
            .lookup_module(module_name, self.current.module)?;
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
        let module_entry = match self
            .info
            .scopes
            .lookup_module(module_name, self.current.module)
        {
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
        self.info
            .scopes
            .lookup_value(name, ScopeValue::Variable, self.current.module)
    }

    fn is_name_public(&self, name: &str) -> bool {
        name.chars()
            .next()
            .map(|ch| ch.is_uppercase())
            .unwrap_or(false)
    }
}
