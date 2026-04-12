use ast::{
    Block, FunctionSignature, NamedTupleElement, NamedTupleType, Struct, VarTypeKind,
    scope::{
        NodeId, Scope, ScopeId, ScopeTypeEntry, ScopeTypeEntryKind, ScopeValue, ScopeValueKind,
    },
};
use ast_parser::parse;
use soul_tokenizer::to_token_stream;
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    ids::FunctionId,
    sementic_level::SementicLevel,
    span::{ModuleId, Spanned},
};

use crate::NameResolver;
mod collect_expression;
mod collect_statement;
mod collect_type;

impl<'a> NameResolver<'a> {
    pub(crate) fn collect_declarations(&mut self, block: &mut Block) {
        block.scope_id = Some(self.info.scopes.current_scope_id());
        self.collect_scopeless_block(block);
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
                .insert_variable_type(id, VarTypeKind::NonInveredType(ty.clone()));

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

    fn declare_module(&mut self, name: &str, module_name: &str, import_kind: ast::ImportKind) {
        let entry = ast::scope::ScopeModuleEntry {
            module_name: module_name.to_string(),
            import_kind,
        };
        self.current_scope_mut().insert_module(name, entry);
    }

    fn find_module_file(&self, module_name: &str) -> Option<std::path::PathBuf> {
        let current_file = self.source_file.as_ref()?;
        let current_dir = current_file.parent()?;

        let module_path = current_dir.join(format!("{}.soul", module_name));
        if module_path.exists() {
            return Some(module_path);
        }

        let relative_path = current_dir.join(module_name);
        if relative_path.exists() {
            return Some(relative_path);
        }

        None
    }

    fn parse_module(&mut self, source: &str, module: ModuleId) -> Option<ast::Block> {
        let tokens = to_token_stream(source, module);
        let response = parse(tokens, self.context, None);

        if self
            .context
            .faults
            .iter()
            .any(|f| f.get_level() == SementicLevel::Error)
        {
            return None;
        }

        Some(response.tree.root)
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
