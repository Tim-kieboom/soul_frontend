use std::path::{Path, PathBuf};

use ast_model::statements::{ImportPath};
use soul_tokenizer::to_token_stream;
use soul_utils::{fault::Fault, ids::IdAlloc, soul_error_internal, span::{ModuleId, Span}};

use crate::parser::Parser;

impl<'a, 'f> Parser<'a, 'f> {

    pub(crate) fn parse_child_module(&mut self, path: &ImportPath, span: Span) {
        let Some(module_file_path) = self.find_module_file(path.module.to_pathbuf(), span) else {
            return;
        };

        let (starting_parent, base_path) = if path.module.is_absolute() {
            (self.get_crate_root(), self.crate_source_path.clone())
        } else {
            (self.get_directory_owner(&self.source_path), self.source_path.clone())
        };

        self.insure_parents_are_loaded(&module_file_path, starting_parent, &base_path, span);
    }

    fn get_directory_owner(&self, dir: &Path) -> ModuleId {
        let mod_path = dir.join("mod.soul");
        if let Some(owner_id) = self.modules.get_id(&mod_path) {
            return owner_id;
        }
        self.id
    }

    fn get_crate_root(&self) -> ModuleId {
        let mut current = self.id;
        while let Some(module) = self.ast_modules.get(current) {
            match module.parent {
                Some(parent) => current = parent,
                None => return current,
            }
        }
        current
    }

    fn import_module(
        &mut self,
        module_file_path: &PathBuf,
        module_name: &str,
        parent: ModuleId,
        span: Span,
    ) -> ModuleId {
        let module_id = self.modules.get_or_insert(module_file_path);
        if self.ast_modules.get(module_id).is_some() {
            return module_id
        }

        let Some(module_source) = self.read_module(module_file_path, module_name, span) else {
            return ModuleId::error()
        };

        let folder_path = module_file_path
            .parent()
            .expect("should have parent")
            .to_path_buf();
        
        self.parse_module(
            &module_source,
            folder_path,
            module_id,
            parent,
            module_name.to_string(),
        );

        module_id
    }

    fn parse_module(
        &mut self,
        source: &str,
        path: PathBuf,
        module_id: ModuleId,
        parent: ModuleId,
        name: String,
    ) {
        let tokens = match to_token_stream(source, module_id) {
            Ok(val) => val,
            Err(err) => {
                self.log_fault(err);
                return
            }
        };
        
        let info = crate::ParseInfo { 
            id: module_id, 
            parent: Some(parent), 
            source_folder: path, 
            crate_source_folder: self.crate_source_path.clone(),
            store: self.store, 
            context: self.context, 
            modules: self.modules, 
            ast_modules: self.ast_modules,
        };
        
        Parser::parse(tokens, name, info);
        if let Some(module) = self.ast_modules.get_mut(parent) {
            module.modules.insert(module_id);
        }
    }

    fn find_module_file(
        &mut self,
        mut module_path: PathBuf,
        span: Span,
    ) -> Option<std::path::PathBuf> {
        if module_path.is_dir() {
            module_path.push("mod.soul");
            if !module_path.is_file() {
                self.log_fault(Fault::error(
                    format!("no 'mod.soul' found in folder '{:?}'", module_path),
                    Some(span),
                ));
                return None;
            }

            return Some(module_path);
        }

        module_path.add_extension("soul");
        if !module_path.is_file() {
            self.log_fault(Fault::error(
                format!("file '{:?}' not found", module_path),
                Some(span),
            ));
        }

        Some(module_path)
    }
    
    fn insure_parents_are_loaded(&mut self, module_file_path: &PathBuf, starting_parent: ModuleId, base_path: &Path, span: Span) {
        fn get_module_name(current: &PathBuf) -> Option<String> {
            let osstr = current.file_name()?;
            osstr
                .to_str()?
                .split('.')
                .next()
                .map(|name| name.to_string())
        }

        let mut current = base_path.to_path_buf();
        let relative_path = match module_file_path.strip_prefix(base_path) {
            Ok(val) => val,
            Err(err) => {
                self.log_fault(soul_error_internal!(format!("{}", err.to_string()), None));
                return;
            }
        };

        let mut parent = starting_parent;
        for component in relative_path.components() {
            current.push(component);
            let name = match get_module_name(&current) {
                Some(val) => val,
                None => {
                    self.log_fault(soul_error_internal!(
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

    fn read_module(&mut self, path: &Path, module_name: &str, span: Span) -> Option<String> {
        match std::fs::read_to_string(path) {
            Ok(val) => Some(val),
            Err(err) => {
                self.log_fault(soul_error_internal!(
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
