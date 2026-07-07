use ast_model::{
    declare_store::FunctionResolve,
    expression::{
        ExpressionKind, FunctionCall, FunctionCallee, FunctionCalleeKind, VariableExpression,
    },
    scope::{ScopeModuleEntry, ScopeTypeEntryKind},
    soul_type::{SoulType, Stub},
    statements::{ImportItem, ImportKind},
};
use soul_utils::{
    FunctionId,
    fault::Fault,
    soul_error_internal,
    span::{ModuleId, Span},
};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_function_call(&mut self, call: &FunctionCall) {
        self.resolve_call(call);
    }

    fn resolve_call(&mut self, call: &FunctionCall) {
        let module_entry = match &self.try_get_callee_string(call) {
            Some(string) => self.lookup_module(string),
            None => None,
        };

        if let Some(module) = module_entry {
            self.resolve_module_call(module, call);
            return;
        }

        if !self.is_function_imported(call) {
            let id = self
                .lookup_function(call.name.as_str())
                .unwrap_or(FunctionId::ERROR);

            self.declares.insert_function_resolve(
                call.id,
                FunctionResolve {
                    id,
                    is_defer: false,
                    ignore_callee: false,
                },
            );
            return;
        }

        let type_qualifier = self.parse_owner_type(call.callee.as_ref());
        let ignore_callee = type_qualifier.is_some();
        if let Some(callee) = &call.callee {
            let is_union = self.is_callee_union(callee).unwrap_or(false);
            if type_qualifier.is_some() && is_union {
                self.declares.insert_function_resolve(
                    call.id,
                    FunctionResolve {
                        id: FunctionId::ERROR,
                        is_defer: false,
                        ignore_callee,
                    },
                );
                return;
            }

            match &callee.kind {
                FunctionCalleeKind::Type(_) => (),
                FunctionCalleeKind::Expression(id) => self.resolve_expression(*id),
            }
        }

        let name = call.name.as_str();
        let owner_type = self.get_owner_kind(type_qualifier.as_ref(), call);
        let resolved = if owner_type.is_some() {
            self.declares.find_function(name, owner_type)
        } else {
            self.lookup_function(name)
        };

        let Some(id) = resolved else { return };
        self.declares.insert_function_resolve(
            call.id,
            FunctionResolve {
                id,
                is_defer: false,
                ignore_callee: false,
            },
        );
    }

    fn get_owner_kind(
        &'a self,
        type_qualifier: Option<&'a SoulType>,
        call: &'a FunctionCall,
    ) -> Option<&'a SoulType> {
        if let Some(ty) = type_qualifier {
            return Some(ty);
        }

        let callee = call.callee.as_ref()?;
        let value = match &callee.kind {
            FunctionCalleeKind::Type(soul_type) => return Some(soul_type),
            FunctionCalleeKind::Expression(id) => self.store.expressions.get(*id)?,
        };

        match &value.node {
            ExpressionKind::Variable(VariableExpression { id, .. }) => {
                let resolved = self.declares.get_variable_resolve(*id)?;
                match self.declares.get_variable_type(resolved)? {
                    (_, Some(ty), _) => Some(ty),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn is_callee_union(&self, callee: &FunctionCallee) -> Option<bool> {
        let ident = match &callee.kind {
            FunctionCalleeKind::Expression(id) => match &self.store.expressions.get(*id)?.node {
                ExpressionKind::Variable(VariableExpression { name, .. }) => name.as_str(),
                _ => return None,
            },
            FunctionCalleeKind::Type(soul_type) => match soul_type {
                SoulType::Stub(stub) => &stub.name,
                SoulType::Res { .. } => return Some(true),
                _ => return None,
            },
        };

        let entry = self
            .scope_info
            .scopes
            .lookup_type(ident, self.current.module)?;
        Some(matches!(entry.kind, ScopeTypeEntryKind::Union))
    }

    fn is_function_imported(&mut self, call: &FunctionCall) -> bool {
        let function_name = call.name.as_str();
        let mut has_module_with_this = false;
        let mut matches_imported_item = false;

        let Some(modules) = self.scope_info.scopes.iter_modules(self.current.module) else {
            self.log_fault(soul_error_internal!(
                format!("{:?} not found", self.current.module),
                Some(call.name.span())
            ));
            return false;
        };

        for (_name, entry) in modules {
            match &entry.import_kind {
                ImportKind::Module => continue,
                ImportKind::This => has_module_with_this = true,
                ImportKind::Items { has_this, .. } if *has_this => {
                    has_module_with_this = true;
                }
                _ => (),
            }

            for item in &entry.imported_items {
                match item {
                    ImportItem::Normal(ident) => {
                        if ident.as_str() == function_name {
                            matches_imported_item = true;
                        }
                    }
                    ImportItem::Alias { alias, .. } => {
                        if alias.as_str() == function_name {
                            matches_imported_item = true;
                        }
                    }
                }
            }
        }

        !has_module_with_this || matches_imported_item
    }

    fn parse_owner_type(&mut self, callee: Option<&FunctionCallee>) -> Option<SoulType> {
        let callee = callee?;
        let value = match &callee.kind {
            FunctionCalleeKind::Type(_) => return None,
            FunctionCalleeKind::Expression(val) => *val,
        };

        let ident = match self.store.expressions.get(value).map(|value| &value.node) {
            Some(ExpressionKind::Variable(VariableExpression { name, .. })) => name,
            _ => return None,
        };

        if self.contains_type(ident.as_str()) {
            return Some(SoulType::Stub(Stub::new(ident.as_str())));
        }

        None
    }

    fn resolve_module_call(&mut self, module_entry: ScopeModuleEntry, call: &FunctionCall) {
        let resolve =
            self.lookup_module_function(&module_entry, call.name.as_str(), call.name.span());
        if resolve == Some(FunctionId::ERROR) {
            todo!("impl resolve_external_function")
        }

        let id = resolve.unwrap_or(FunctionId::ERROR);
        let ignore_callee = id != FunctionId::ERROR;
        self.declares.insert_function_resolve(
            call.id,
            FunctionResolve {
                id,
                ignore_callee,
                is_defer: false,
            },
        );
        return;
    }

    fn lookup_module_function(
        &mut self,
        module_entry: &ScopeModuleEntry,
        function_name: &str,
        span: Span,
    ) -> Option<FunctionId> {
        if let Some(function_name) = self.lookup_function_import(module_entry, function_name) {
            return self.lookup_function(&function_name);
        }

        if let Some(_) = &module_entry.crate_name {
            todo!("impl external crates")
        }

        let module_id = module_entry.module_id;
        debug_assert!(module_id != ModuleId::ERROR);
        debug_assert!(self.ast_modules.contains(module_id));

        let header = &self.ast_modules.get(module_id)?.header;
        let entry = header.get(function_name)?.function?;
        if !entry.is_public {
            self.log_fault(Fault::error(
                format!("'{function_name}' is private"),
                Some(span),
            ));
        }

        Some(entry.value)
    }

    fn lookup_function_import(
        &mut self,
        module_entry: &ScopeModuleEntry,
        function_name: &str,
    ) -> Option<String> {
        for item in &module_entry.imported_items {
            match item {
                ast_model::statements::ImportItem::Normal(name) => {
                    if name.as_str() == function_name {
                        return Some(name.to_string());
                    }
                }
                ast_model::statements::ImportItem::Alias { alias, name } => {
                    if alias.as_str() == function_name {
                        return Some(name.to_string());
                    }
                }
            }
        }

        None
    }

    fn try_get_callee_string(&mut self, call: &FunctionCall) -> Option<String> {
        let callee = call.callee.as_ref()?;
        let value = match &callee.kind {
            FunctionCalleeKind::Type(_) => return None,
            FunctionCalleeKind::Expression(val) => *val,
        };

        let Some(value) = self.store.expressions.get(value) else {
            self.log_fault(soul_error_internal!(
                format!("{value:?} not found"),
                Some(call.name.span())
            ));
            return None;
        };

        match &value.node {
            ExpressionKind::Variable(var) => Some(var.name.to_string()),
            _ => None,
        }
    }

    pub(super) fn contains_type(&mut self, ident: &str) -> bool {
        self.scope_info
            .scopes
            .lookup_type(ident, self.current.module)
            .is_some()
    }

    fn lookup_module(&mut self, string: &str) -> Option<ScopeModuleEntry> {
        self.scope_info
            .scopes
            .lookup_module(string, self.current.module)
    }

    fn lookup_function(&mut self, string: &str) -> Option<FunctionId> {
        self.scope_info
            .scopes
            .lookup_function(string, self.current.module)
    }
}
