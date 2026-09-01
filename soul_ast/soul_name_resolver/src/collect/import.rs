use std::path::{Path, PathBuf};

use ast_model::{
    EntryKind, HeaderEntry, NodeId,
    scope::ScopeModuleEntry,
    statements::{ImportItem, ImportKind, ImportPath},
};
use soul_utils::{
    FunctionId, Ident,
    fault::Fault,
    ids::IdAlloc,
    soul_error_internal,
    span::{ModuleId, Span},
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

        let insert_name = alias.unwrap_or(module_name);
        self.current_scope_mut().insert_module(
            insert_name,
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

        if matches!(&path.kind, ImportKind::Module) {
            self.re_export_module_items(module_id);
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
                ModuleId::ERROR
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
                return ModuleId::ERROR;
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
            return ModuleId::ERROR;
        };

        let module_path = to_crate_name(path.module.as_pathbuf());

        let result = if module_path.as_os_str().is_empty() {
            let root = &crate_entry.source_root;
            self.modules
                .get_id(&root.join("lib.soul"))
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
                ModuleId::ERROR
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
                self.current.module,
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

    fn re_export_module_items(&mut self, module_id: ModuleId) {
        fn is_entry_public(entry: &HeaderEntry) -> bool {
            let var_public = entry.variable.map(|var| var.is_public).unwrap_or(false);
            let func_public = entry.function.map(|func| func.is_public).unwrap_or(false);
            let ty_public = entry
                .custom_type
                .as_ref()
                .map(|ty| ty.is_public)
                .unwrap_or(false);
            func_public || ty_public || var_public
        }

        let mut to_re_export: Vec<(String, ast_model::HeaderEntry)> = vec![];
        let target = match self.ast_modules.get(module_id) {
            Some(module) => &module.header,
            None => return,
        };

        for (name, entry) in target.iter() {
            if is_entry_public(entry) {
                to_re_export.push((name.clone(), entry.clone()));
            }
        }

        let current = match self.ast_modules.get_mut(self.current.module) {
            Some(module) => &mut module.header,
            None => return,
        };

        for (name, entry) in to_re_export {
            let h = current.entry(name).or_default();
            if let Some(func) = entry.function
                && func.is_public
            {
                h.function.get_or_insert(func);
            }

            if let Some(var) = entry.variable
                && var.is_public
            {
                h.variable.get_or_insert(var);
            }
            if let Some(ty) = entry.custom_type
                && ty.is_public
            {
                h.custom_type.get_or_insert(ty);
            }
        }
    }
}

fn to_crate_name(path: &Path) -> PathBuf {
    path.components().skip(1).collect::<PathBuf>()
}
