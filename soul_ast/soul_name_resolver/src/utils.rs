use ast_model::{
    NodeId,
    block::{Block, BlockId},
    expression::{Binding, Expression, ExpressionId},
    scope::{Scope, ScopeBuilder, ScopeValue},
    statements::{FunctionSignature, Statement, StatementId},
};
use soul_utils::{
    CrateContext, FunctionId,
    fault::Fault,
    span::{ModuleId, Span},
};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(crate) fn log_fault(&mut self, fault: Fault) {
        self.context.faults.push(fault);
    }

    pub(crate) fn static_log_fault(context: &mut CrateContext, fault: Fault) {
        context.faults.push(fault);
    }

    pub(crate) fn alloc_node(&mut self) -> NodeId {
        self.node_generator.alloc()
    }

    pub(crate) fn declare_function(
        &mut self,
        function_signature: &FunctionSignature,
    ) -> FunctionId {
        let id = function_signature.value.id;
        let name = function_signature.value.name.as_str();
        self.current_scope_mut().insert_function(name, id);
        id
    }

    pub(crate) fn static_current_scope_mut(
        scopes: &mut ScopeBuilder,
        module: ModuleId,
    ) -> &mut Scope {
        scopes
            .current_scope_mut(module)
            .expect("resolver has no scope")
    }

    pub(crate) fn insert_binding(&mut self, binding: &Binding) {
        self.insert_value(
            binding.ident.as_str(),
            binding.id,
            binding.ident.span(),
            ScopeValue::Variable,
        )
    }

    pub(crate) fn insert_value(&mut self, name: &str, id: NodeId, span: Span, kind: ScopeValue) {
        if self
            .current_scope_mut()
            .insert_value(name, kind, id)
            .is_some()
        {
            self.log_fault(Fault::error(
                format!("`{name}` already exists in scope"),
                Some(span),
            ));
        }
    }

    pub(crate) fn get_block(&self, id: BlockId) -> Option<&Block> {
        self.store.blocks.get(id)
    }

    pub(crate) fn get_statement(&self, id: StatementId) -> Option<&Statement> {
        self.store.statements.get(id)
    }

    pub(crate) fn get_expression(&self, id: ExpressionId) -> Option<&Expression> {
        self.store.expressions.get(id)
    }
}
