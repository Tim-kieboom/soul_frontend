use ast::{
    Block, FunctionSignature, NamedTupleElement, NamedTupleType, Struct, VarTypeKind, scope::{
        NodeId, Scope, ScopeId, ScopeTypeEntry, ScopeTypeEntryKind, ScopeValue, ScopeValueKind,
    }
};
use ast_parser::parse;
use soul_tokenizer::to_token_stream;
use soul_utils::{
    error::{SoulError, SoulErrorKind}, ids::{FunctionId}, soul_error_internal,
    span::{ModuleId, Span, Spanned}
};

use crate::NameResolver;
mod collect_expression;
mod collect_statement;
mod collect_type;

impl<'a> NameResolver<'a> {
    pub(crate) fn collect_module(&mut self, module_id: ModuleId) {
        use std::mem::swap;
        
        let mut global = Block::dummy();
        match self.modules.get_mut(module_id) {
            Some(module) => swap(&mut module.global, &mut global),
            None => {
                self.log_error(soul_error_internal!(format!("{:?} not found", module_id), None));
                return
            }
        }
        
        let prev = self.current_module;
        self.current_module = module_id;
        self.collect_block(&mut global);
        self.current_module = prev;
        
        swap(
            &mut global,
            &mut self.modules.get_mut(module_id).expect("just checked").global,
        );
    }

    fn push_scope(&mut self, set_scope_id: &mut Option<ScopeId>) {
        let parent = self.info.scopes.current_scope_id();
        self.info.scopes.push_scope(parent);
        *set_scope_id = Some(self.info.scopes.current_scope_id())
    }

    fn pop_scope(&mut self) {
        self.info.scopes.pop_scope();
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

            self.store
                .insert_variable_type(id, VarTypeKind::NonInveredType(ty.clone()), self.current_module);

            self.insert_value(name.as_str(), id, ScopeValue::Variable);
        }
    }

    fn declare_struct(&mut self, obj: &mut Struct) -> NodeId {
        let id = self.alloc_node();
        obj.id = Some(id);

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

    fn insert_function_alias(&mut self, name: &str, id: FunctionId) {
        self.current_scope_mut().insert_function(name, id);
    }

    fn declare_module(&mut self, name: &str, module_name: &str, module_id: ModuleId, import_kind: ast::ImportKind, imported_items: Vec<ast::ImportItem>) {
        let entry = ast::scope::ScopeModuleEntry {
            module_name: module_name.to_string(),
            module_id,
            import_kind,
            imported_items,
        };
        self.current_scope_mut().insert_module(name, entry);
    }

    fn find_module_file(&mut self, module_name: &str, span: Span) -> Option<std::path::PathBuf> {
        let current_dir = self.context.current_path();

        let mut module_path = current_dir.join(module_name);
        if module_path.is_dir() {

            module_path.push("mod.soul");
            if !module_path.is_file() {
                self.log_error(SoulError::new(format!("no 'mod.soul' found in folder '{:?}'", module_path), SoulErrorKind::FieldNotFound, Some(span)));
                return None
            }

            return Some(module_path)
        }
        
        module_path.add_extension("soul");
        if !module_path.is_file() {
            self.log_error(SoulError::new(format!("file '{:?}' not found", module_path), SoulErrorKind::FieldNotFound, Some(span)));
        }

        Some(module_path)
    }

    fn parse_module(&mut self, source: &str, module_id: ModuleId, name: String) {
        let tokens = to_token_stream(source, module_id);
        let module = parse(tokens, module_id, name, self.context);
        if let Some(module) = self.modules.get_mut(self.current_module) {
            module.modules.push(module_id);
        }
        self.modules.insert(module_id, module);

        self.collect_module(module_id);
    }

    fn insert_value(&mut self, name: &str, id: NodeId, kind: ScopeValue) -> Option<NodeId> {
        self.current_scope_mut().insert_value(name, kind, id)
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.info
            .scopes
            .current_scope_mut()
            .expect("resolver has no scope")
    }
}
