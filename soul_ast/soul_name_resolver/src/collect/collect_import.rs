use std::path::{Path, PathBuf};

use ast::{EntryKind, ImportItem, ImportPath};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    ids::IdAlloc,
    soul_error_internal,
    span::{ModuleId, Span},
};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(crate) fn collect_import_path(&mut self, path: &ImportPath, span: Span) {

        if let Some(name) = &path.lib_name {
            self.collect_external_lib(path, name, span);
        } else {
            self.collect_internal_module(path, span);
        }
    }

    fn collect_external_lib(&mut self, _path: &ImportPath, _lib_name: &String, _span: Span) {
        todo!()
        // let Some(lib_path) = self.context.libarys.get(lib_name) else {
        //     self.log_error(SoulError::new(format!("lib '{lib_name}' not found"), SoulErrorKind::PathNotFound, Some(span)));
        //     return 
        // };
        
        // let module_name = match lib_path.module.get_module_name() {
        //     Some(val) => val,
        //     None => {
        //         self.log_error(soul_error_internal!("could not get module name", None));
        //         return;
        //     }
        // };

        // let alias = match &lib_path.kind {
        //     ast::ImportKind::Alias(ident) => Some(ident.as_str()),
        //     _ => None,
        // };

        // let imported_items = match &lib_path.kind {
        //     ast::ImportKind::Items { items, .. } => items,
        //     _ => &vec![],
        // };

        // lib_path.module
    }

    fn collect_internal_module(&mut self, path: &ImportPath, span: Span) {
        let module_name = match path.module.get_module_name() {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!("could not get module name", None));
                return;
            }
        };

        let alias = match &path.kind {
            ast::ImportKind::Alias(ident) => Some(ident.as_str()),
            _ => None,
        };

        let imported_items = match &path.kind {
            ast::ImportKind::Items { items, .. } => items,
            _ => &vec![],
        };

        let parent_module = self.current.module;
        self.check_if_module_private(path, span);

        let Some(module_file_path) = self.find_module_file(path.module.to_pathbuf(), span) else {
            return;
        };

        self.insure_parents_are_loaded(&module_file_path, span);

        let module_id = self.import_module(&module_file_path, module_name, parent_module, span);

        let import_name = alias.unwrap_or(module_name);
        self.declare_module(
            import_name,
            &module_name,
            module_id,
            path.kind.clone(),
            imported_items.clone(),
        );

        self.collect_items(module_id, module_name, &imported_items, span)
    }

    fn is_module_internal(&mut self, path: &ImportPath) -> bool {
        let Some(parent) = path.module.as_pathbuf().parent() else {
            return false;
        };

        let Some(parent_name) = parent.file_name() else {
            return false;
        };

        let current = self.context.current_path();
        if current.file_name() == Some(parent_name) {
            return true;
        }

        let Some(current_parent) = current.parent() else {
            return false;
        };

        current_parent.file_name() == Some(parent_name)
    }

    fn check_if_module_private(&mut self, path: &ImportPath, span: Span) {
        if self.is_module_internal(path) {
            return;
        }

        let path = path.module.as_path();
        let Some(name) = path.file_name().and_then(|f| f.to_str()) else {
            return;
        };

        let is_public = self.is_name_public(name);
        if !is_public {
            self.log_error(SoulError::new(
                format!("module '{}' is private", name),
                SoulErrorKind::NotFoundInScope,
                Some(span),
            ));
            return;
        }
    }

    fn insure_parents_are_loaded(&mut self, module_file_path: &PathBuf, span: Span) {
        fn get_module_name(current: &PathBuf) -> Option<String> {
            let osstr = current.file_name()?;
            osstr
                .to_str()?
                .split('.')
                .next()
                .map(|name| name.to_string())
        }

        let mut current = self.context.source_folder.clone();
        let relative_path = match module_file_path.strip_prefix(&current) {
            Ok(val) => val,
            Err(err) => {
                self.log_error(soul_error_internal!(format!("{}", err.to_string()), None));
                return;
            }
        };

        let mut parent = self.current.module;
        for component in relative_path.components() {
            current.push(component);
            let name = match get_module_name(&current) {
                Some(val) => val,
                None => {
                    self.log_error(soul_error_internal!(
                        format!("file_name of '{:?}' not found", current),
                        None
                    ));
                    return;
                }
            };

            let is_dir = current.is_dir();
            if is_dir {
                current.push("mod.soul");
            }
            parent = self.import_module(&current, &name, parent, span);
            if is_dir {
                current.pop();
            }
        }
    }

    fn collect_items(
        &mut self,
        module_id: ModuleId,
        module_name: &str,
        imported_items: &Vec<ImportItem>,
        span: Span,
    ) {
        for item in imported_items {
            let (name, alias_name) = match &item {
                ast::ImportItem::Alias { name, alias } => (name.as_str(), alias),
                ast::ImportItem::Normal(name) => (name.as_str(), name),
            };

            let Some(module) = self.modules.get(module_id) else {
                self.log_error(soul_error_internal!(
                    format!("module {:?} not found", module_id),
                    Some(span)
                ));
                continue;
            };

            let Some(entry) = module.header.get(name) else {
                self.log_error(SoulError::new(
                    format!("module '{}' does not export '{}'", module_name, name),
                    SoulErrorKind::NotFoundInScope,
                    Some(span),
                ));
                continue;
            };

            let entry_variable = entry.variable;
            let entry_function = entry.function;
            if let Some(EntryKind {
                value: obj,
                is_public,
            }) = &entry.struct_type
            {
                if !is_public {
                    Self::static_log_error(
                        self.context,
                        SoulError::new(
                            format!("struct {} is private", alias_name.as_str()),
                            SoulErrorKind::AlreadyFoundInScope,
                            Some(alias_name.span),
                        ),
                    );
                }

                let id = match obj.id {
                    Some(val) => val,
                    None => {
                        self.log_error(soul_error_internal!(
                            format!("Struct: '{}' node_id is None", obj.name.as_str()),
                            None
                        ));
                        return;
                    }
                };

                if !Self::insert_struct_alias(
                    &mut self.info.scopes,
                    alias_name,
                    span,
                    id,
                    self.current.module,
                ) {
                    Self::static_log_error(
                        self.context,
                        SoulError::new(
                            format!("struct {} already exists", alias_name.as_str()),
                            SoulErrorKind::AlreadyFoundInScope,
                            Some(alias_name.span),
                        ),
                    );
                }

                Self::resolve_struct(self.context, self.store, &self.current, obj);
            }

            if let Some(EntryKind {
                value: id,
                is_public,
            }) = entry_variable
            {
                if !is_public {
                    self.log_error(SoulError::new(
                        format!("variable '{}' is private", alias_name.as_str()),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(alias_name.span),
                    ));
                }

                if !self.insert_variable_alias(alias_name, id) {
                    self.log_error(SoulError::new(
                        format!("variable '{}' already exists", alias_name.as_str()),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(alias_name.span),
                    ));
                }
            }

            if let Some(EntryKind {
                value: id,
                is_public,
            }) = entry_function
            {
                if !is_public {
                    self.log_error(SoulError::new(
                        format!("function '{}' is private", alias_name.as_str()),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(alias_name.span),
                    ));
                }

                if !self.insert_function_alias(alias_name, id) {
                    self.log_error(SoulError::new(
                        format!("function '{}' already exists", alias_name.as_str()),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(alias_name.span),
                    ));
                }
            }
        }
    }

    fn import_module(
        &mut self,
        module_file_path: &PathBuf,
        module_name: &str,
        parent: ModuleId,
        span: Span,
    ) -> ModuleId {
        let dir = module_file_path.parent().unwrap_or(module_file_path);
        let module_id = self.context.module_store.get_or_insert(module_file_path);
        if self.modules.get(module_id).is_some() {
            return module_id;
        }

        let Some(module_source) = self.read_module(module_file_path, module_name, span) else {
            return ModuleId::error();
        };

        self.context.push_current_path(dir.to_path_buf());
        self.parse_module(&module_source, module_id, parent, module_name.to_string());
        self.context.pop_current_path();
        module_id
    }

    fn read_module(&mut self, path: &Path, module_name: &str, span: Span) -> Option<String> {
        match std::fs::read_to_string(path) {
            Ok(val) => Some(val),
            Err(err) => {
                self.log_error(soul_error_internal!(
                    format!(
                        "import '{}': could not read module file '{}': {}",
                        module_name,
                        path.display(),
                        err,
                    ),
                    Some(span)
                ));
                None
            }
        }
    }
}
