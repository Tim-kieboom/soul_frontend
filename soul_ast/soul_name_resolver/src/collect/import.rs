use std::path::PathBuf;

use ast_model::{
    EntryKind, NodeId,
    scope::ScopeModuleEntry,
    statements::{ImportItem, ImportKind, ImportPath},
};
use soul_utils::{
    FunctionId, Ident, fault::Fault, ids::IdAlloc, soul_error_internal, span::{ModuleId, Span},
};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_import_path(&mut self, path: &ImportPath, span: Span) {
        let module_name = match path.module.get_module_name() {
            Some(val) => val,
            None => {
                self.log_fault(soul_error_internal!("could not get module name", None));
                return;
            }
        };

        let alias = match &path.kind {
            ImportKind::Alias(ident) => Some(ident.as_str()),
            _ => None,
        };

        let imported_items = match &path.kind {
            ImportKind::Items { items, .. } => items,
            _ => &vec![],
        };

        let module_id = if path.module.is_external() {
            self.collect_external_module(path, span)
        } else {
            self.collect_internal_module(path, span)
        };

        if module_id == ModuleId::error() {
            return;
        }

        self.collect_module(module_id);
        self.resolve_module(module_id);

        self.current_scope_mut().insert_module(
            alias.unwrap_or(module_name),
            ScopeModuleEntry {
                module_id,
                import_kind: path.kind.clone(),
                crate_name: path.lib_name.clone(),
                module_name: module_name.to_string(),
                imported_items: imported_items.clone(),
            },
        );

        for imported_item in imported_items {
            self.collect_items(module_id, module_name, imported_item, span)
        }
    }

    fn collect_internal_module(&mut self, path: &ImportPath, span: Span) -> ModuleId {
        let pathbuf = path.module.as_pathbuf();
        let result = self
            .modules
            .get_id(pathbuf)
            .or_else(|| self.modules.get_id(&pathbuf.with_extension("soul")))
            .or_else(|| self.modules.get_id(&pathbuf.join("mod.soul")));

        match result {
            Some(val) => val,
            None => {
                self.log_fault(soul_error_internal!(
                    format!("module: {:?} not found in ModuleStore", pathbuf),
                    Some(span)
                ));
                return ModuleId::error();
            }
        }
    }

    fn collect_external_module(&mut self, path: &ImportPath, span: Span) -> ModuleId {
        let lib_name = match &path.lib_name {
            Some(name) => name,
            None => {
                self.log_fault(Fault::error(
                    "external import missing crate name".to_string(),
                    Some(span),
                ));
                return ModuleId::error();
            }
        };

        let Some(crate_entry) = self.crate_store.get(lib_name) else {
            self.log_fault(Fault::error(
                format!(
                    "external crate '{}' not found in Soul.toml dependencies",
                    lib_name
                ),
                Some(span),
            ));
            return ModuleId::error();
        };

        let module_path = to_crate_name(path.module.as_pathbuf());

        let result = if module_path.as_os_str().is_empty() {
            let root = &crate_entry.source_root;
            self.modules.get_id(&root.join("lib.soul"))
                .or_else(|| self.modules.get_id(&root.join("main.soul")))
                .or_else(|| self.modules.get_id(&root.join("mod.soul")))
        } else {
            let full_path = crate_entry.source_root.join(&module_path);
            self.modules
                .get_id(&full_path)
                .or_else(|| self.modules.get_id(&full_path.with_extension("soul")))
                .or_else(|| self.modules.get_id(&full_path.join("mod.soul")))
        };

        match result {
            Some(val) => val,
            None => {
                self.log_fault(soul_error_internal!(
                    format!(
                        "module '{:?}' not found in crate '{}'",
                        module_path, lib_name
                    ),
                    Some(span)
                ));
                return ModuleId::error();
            }
        }
    }

    fn collect_items(
        &mut self,
        module_id: ModuleId,
        module_name: &str,
        item: &ImportItem,
        span: Span,
    ) {
        let (name, alias_name) = match item {
            ImportItem::Alias { name, alias } => (name.as_str(), alias),
            ImportItem::Normal(name) => (name.as_str(), name),
        };

        let Some(module) = self.ast_modules.get(module_id) else {
            self.log_fault(soul_error_internal!(
                format!("module {:?} not found", module_id),
                Some(span)
            ));
            return;
        };

        let Some(entry) = module.header.get(name) else {
            self.log_fault(Fault::error(
                format!("module `{}` does not export `{}`", module_name, name),
                Some(span),
            ));
            return;
        };

        let entry_variable = entry.variable;
        let entry_function = entry.function;
        if let Some(entry) = &entry.custom_type {
            if !entry.is_public {
                Self::static_log_fault(
                    self.context,
                    Fault::error(
                        format!(
                            "{} {} is private",
                            entry.value.variant_name(),
                            alias_name.as_str()
                        ),
                        Some(alias_name.span()),
                    ),
                );
            }

            let id = entry.value.id();
            if !Self::insert_struct_alias(
                &mut self.scope_info.scopes,
                alias_name,
                span,
                id,
                module_id,
            ) {
                Self::static_log_fault(
                    self.context,
                    Fault::error(
                        format!(
                            "{} {} already exists",
                            entry.value.variant_name(),
                            alias_name.as_str()
                        ),
                        Some(alias_name.span()),
                    ),
                );
            }
        }

        if let Some(entry) = entry_variable {
            self.collect_entry_variable(entry, alias_name);
        }

        if let Some(entry) = entry_function {
            self.collect_entry_function(entry, alias_name);
        }
    }

    fn collect_entry_variable(&mut self, entry: EntryKind<NodeId>, alias_name: &Ident) {
        if !entry.is_public {
            self.log_fault(Fault::error(
                format!("variable '{}' is private", alias_name.as_str()),
                Some(alias_name.span()),
            ));
        }

        if !self.insert_variable_alias(alias_name, entry.value) {
            self.log_fault(Fault::error(
                format!("variable '{}' already exists", alias_name.as_str()),
                Some(alias_name.span()),
            ));
        }
    }

    fn collect_entry_function(&mut self, entry: EntryKind<FunctionId>, alias_name: &Ident) {
        if !entry.is_public {
            self.log_fault(Fault::error(
                format!("function '{}' is private", alias_name.as_str()),
                Some(alias_name.span()),
            ));
        }

        if !self.insert_function_alias(alias_name, entry.value) {
            self.log_fault(Fault::error(
                format!("function '{}' already exists", alias_name.as_str()),
                Some(alias_name.span()),
            ));
        }
    }
}

fn to_crate_name(path: &PathBuf) -> PathBuf {
    path
        .components()
        .skip(1)
        .collect::<PathBuf>()
}
