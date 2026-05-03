use std::path::PathBuf;

use ast::{
    Block, Enum, FunctionSignature, NamedTupleElement, NamedTupleType, Struct, VarTypeKind, scope::{
        NodeId, Scope, ScopeBuilder, ScopeId, ScopeTypeEntry, ScopeTypeEntryKind, ScopeValue,
        ScopeValueKind,
    }
};
use ast_parser::parse_module;
use soul_tokenizer::to_token_stream;
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::FunctionId,
    soul_error_internal,
    span::{ModuleId, Span, Spanned},
};

use crate::NameResolver;
mod collect_expression;
mod collect_import;
mod collect_statement;
mod collect_type;

impl<'a> NameResolver<'a> {
    pub(crate) fn collect_module(&mut self, module_id: ModuleId) {
        use std::mem::swap;

        let mut global = Block::dummy();
        match self.modules.get_mut(module_id) {
            Some(module) => swap(&mut module.global, &mut global),
            None => {
                self.log_error(soul_error_internal!(
                    format!("{:?} not found", module_id),
                    None
                ));
                return;
            }
        }

        let prev = self.current.module;
        self.current.module = module_id;
        self.current.in_global = true;
        self.collect_block(&mut global);
        self.current.module = prev;

        match self.modules.get_mut(module_id) {
            Some(module) => swap(&mut global, &mut module.global),
            None => {
                self.log_error(soul_error_internal!(
                    format!("{:?} not found", module_id),
                    None
                ));
                return;
            }
        }
    }

    fn push_scope(&mut self, set_scope_id: &mut Option<ScopeId>) {
        let parent = match self.info.scopes.current_scope_id(self.current.module) {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    format!(
                        "push_scope current_scope_id is None {:?}",
                        self.current.module
                    ),
                    None
                ));
                return;
            }
        };
        self.info
            .scopes
            .push_scope(parent, self.current.module)
            .expect("no err");
        *set_scope_id = self.info.scopes.current_scope_id(self.current.module)
    }

    fn pop_scope(&mut self) {
        self.info
            .scopes
            .pop_scope(self.current.module)
            .expect("no err");
    }

    fn alloc_node(&mut self) -> NodeId {
        self.node_generator.alloc()
    }

    fn alloc_function(&mut self) -> FunctionId {
        self.function_generator.alloc()
    }

    fn declare_parameters(&mut self, types: &mut NamedTupleType) {
        for NamedTupleElement {
            name,
            ty,
            node_id,
            default: _,
        } in types
        {
            let id = self.alloc_node();
            *node_id = Some(id);

            self.store.insert_variable_type(
                id,
                VarTypeKind::NonInveredType(ty.clone()),
                self.current.module,
            );

            self.insert_value(name.as_str(), id, ScopeValue::Variable);
        }
    }

    fn declare_struct(&mut self, obj: &mut Struct) -> NodeId {
        let id = self.alloc_node();
        obj.id = Some(id);
        obj.defined_in = Some(self.current.module);

        let name = &obj.name;
        let scope_type = ScopeTypeEntry {
            node_id: id,
            trait_parent: None,
            span: name.span,
            kind: ScopeTypeEntryKind::Struct,
        };

        let old_entry = self
            .current_scope_mut()
            .insert_types(name.as_str(), scope_type);

        if old_entry.is_some() {
            self.log_error(SoulError::new(
                format!("type of name {} already exists in scope", name.as_str()),
                SoulErrorKind::AlreadyFoundInScope,
                Some(name.span),
            ));
        }

        id
    }

    fn declare_enum(&mut self, obj: &mut Enum) -> NodeId {
        let id = self.alloc_node();
        obj.id = Some(id);

        let name = &obj.name;
        let scope_type = ScopeTypeEntry {
            node_id: id,
            trait_parent: None,
            span: name.span,
            kind: ScopeTypeEntryKind::Enum,
        };

        let old_entry = self
            .current_scope_mut()
            .insert_types(name.as_str(), scope_type);

        if old_entry.is_some() {
            self.log_error(SoulError::new(
                format!("type of name {} already exists in scope", name.as_str()),
                SoulErrorKind::AlreadyFoundInScope,
                Some(name.span),
            ));
        }

        id
    }

    fn declare_value(&mut self, mut value: ScopeValueKind) -> NodeId {
        let id = self.alloc_node();
        *value.get_id_mut() = Some(id);

        let name = value.get_ident();
        let old_entry = self.insert_value(name.as_str(), id, value.to_entry_kind());
        if old_entry.is_some() {
            self.log_error(SoulError::new(
                format!("name {} already exists in scope", name.as_str()),
                SoulErrorKind::AlreadyFoundInScope,
                Some(name.span),
            ));
        }
        id
    }

    fn declare_function(
        &mut self,
        function_signature: &mut Spanned<FunctionSignature>,
    ) -> FunctionId {
        let id = self.alloc_function();
        function_signature.node.id = Some(id);
        let name = function_signature.node.name.as_str();
        self.insert_function(name, id);
        id
    }

    fn insert_function(&mut self, name: &str, id: FunctionId) {
        self.current_scope_mut().insert_function(name, id);
    }

    fn insert_function_alias(&mut self, name: &Ident, id: FunctionId) -> bool {
        if self
            .info
            .scopes
            .flat_lookup_function(name.as_str(), self.current.module)
            .is_some()
        {
            return false;
        }

        self.current_scope_mut().insert_function(name.as_str(), id);
        true
    }

    fn insert_variable_alias(&mut self, name: &Ident, id: NodeId) -> bool {
        if self
            .info
            .scopes
            .flat_lookup_value(name, ScopeValue::Variable, self.current.module)
            .is_some()
        {
            return false;
        }

        self.current_scope_mut()
            .insert_value(name.as_str(), ScopeValue::Variable, id);
        true
    }

    fn insert_struct_alias(
        scopes: &mut ScopeBuilder,
        name: &Ident,
        span: Span,
        id: NodeId,
        module: ModuleId,
    ) -> bool {
        if scopes.flat_lookup_type(name, module).is_some() {
            return false;
        }

        Self::static_current_scope_mut(scopes, module).insert_types(
            name.as_str(),
            ScopeTypeEntry {
                span,
                node_id: id,
                trait_parent: None,
                kind: ScopeTypeEntryKind::Struct,
            },
        );

        true
    }

    fn declare_module(
        &mut self,
        name: &str,
        module_name: &str,
        module_id: ModuleId,
        import_kind: ast::ImportKind,
        imported_items: Vec<ast::ImportItem>,
        crate_name: Option<String>,
    ) {
        let entry = ast::scope::ScopeModuleEntry {
            module_name: module_name.to_string(),
            module_id,
            crate_name,
            import_kind,
            imported_items,
        };
        self.current_scope_mut().insert_module(name, entry);
    }

    fn find_module_file(
        &mut self,
        mut module_path: PathBuf,
        span: Span,
    ) -> Option<std::path::PathBuf> {
        if module_path.is_dir() {
            module_path.push("mod.soul");
            if !module_path.is_file() {
                self.log_error(SoulError::new(
                    format!("no 'mod.soul' found in folder '{:?}'", module_path),
                    SoulErrorKind::FieldNotFound,
                    Some(span),
                ));
                return None;
            }

            return Some(module_path);
        }

        module_path.add_extension("soul");
        if !module_path.is_file() {
            self.log_error(SoulError::new(
                format!("file '{:?}' not found", module_path),
                SoulErrorKind::FieldNotFound,
                Some(span),
            ));
        }

        Some(module_path)
    }

    fn parse_module(&mut self, source: &str, path: PathBuf, module_id: ModuleId, parent: ModuleId, name: String) {
        let tokens = to_token_stream(source, module_id);
        let module = parse_module(
            tokens,
            module_id,
            name,
            Some(parent),
            self.context,
            path,
        );

        if let Some(module) = self.modules.get_mut(parent) {
            module.modules.insert(module_id);
        }
        self.modules.insert(module_id, module);
        let _res = self.info.scopes.add_module(module_id);
        debug_assert!(_res.is_none());

        self.collect_module(module_id);
        self.resolve_modules(module_id);
    }

    fn insert_value(&mut self, name: &str, id: NodeId, kind: ScopeValue) -> Option<NodeId> {
        self.current_scope_mut().insert_value(name, kind, id)
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.info
            .scopes
            .current_scope_mut(self.current.module)
            .expect("resolver has no scope")
    }

    fn static_current_scope_mut<'b>(
        scopes: &'b mut ScopeBuilder,
        module: ModuleId,
    ) -> &'b mut Scope {
        scopes
            .current_scope_mut(module)
            .expect("resolver has no scope")
    }
}
