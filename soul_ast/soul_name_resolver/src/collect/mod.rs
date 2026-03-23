use ast::{
    Block, FunctionSignature, NamedTupleElement, NamedTupleType, VarTypeKind,
    scope::{NodeId, Scope, ScopeId, ScopeValue, ScopeValueKind},
};
use soul_utils::{ids::FunctionId, span::Spanned};

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
        self.info.scopes.push_scope();
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

            self.insert_value(name.as_str(), id, ScopeValue::Variable)
        }
    }

    fn declare_value(&mut self, mut value: ScopeValueKind) -> NodeId {
        let id = self.alloc_node();
        *value.get_id_mut() = Some(id);

        self.insert_value(value.get_name().as_str(), id, value.to_entry_kind());
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

    fn insert_function<S: Into<String>>(&mut self, name: S, id: FunctionId) {
        self.current_scope_mut().functions.insert(name.into(), id);
    }

    fn insert_value<S: Into<String>>(&mut self, name: S, id: NodeId, kind: ScopeValue) {
        self.current_scope_mut()
            .values
            .entry(name.into())
            .or_default()
            .insert(kind, id);
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.info
            .scopes
            .current_scope_mut()
            .expect("resolver has no scope")
    }
}
