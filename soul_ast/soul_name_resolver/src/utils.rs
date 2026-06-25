use ast_model::{
    NodeId,
    expression::Binding,
    scope::{Scope, ScopeBuilder, ScopeValue, ScopeValueKind},
    statements::FunctionSignature,
};
use soul_utils::{
    CrateContext, FunctionId,
    fault::Fault,
    span::{ModuleId, Span, Spanned},
};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(crate) fn log_fault(&mut self, fault: Fault) {
        self.context.faults.push(fault);
    }

    pub(crate) fn static_log_fault(context: &mut CrateContext, fault: Fault) {
        context.faults.push(fault);
    }

    pub(crate) fn declare_value(&mut self, value: ScopeValueKind) {
        let name = value.get_ident();
        self.insert_value(
            name.as_str(),
            value.get_id(),
            name.span(),
            value.to_entry_kind(),
        );
    }

    pub(crate) fn declare_function(
        &mut self,
        function_signature: &Spanned<FunctionSignature>,
    ) -> FunctionId {
        let id = function_signature.value.id;
        let name = function_signature.value.name.as_str();
        self.current_scope_mut().insert_function(name, id);
        id
    }

    pub(crate) fn static_current_scope_mut<'b>(
        scopes: &'b mut ScopeBuilder,
        module: ModuleId,
    ) -> &'b mut Scope {
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
}
