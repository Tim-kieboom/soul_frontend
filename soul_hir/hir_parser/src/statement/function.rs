use ast::NamedTupleElement;
use hir::TypeId;
use soul_utils::{
    Ident, error::{SoulError, SoulErrorKind}, ids::{FunctionId, IdAlloc}, print_breakpoint, soul_error_internal
};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_function(&mut self, function: &ast::Function) -> FunctionId {
        let id = match function.signature.node.id {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "function.id should be Some(_)",
                    Some(function.signature.span)
                ));
                FunctionId::error()
            }
        };

        let signature = &function.signature.node;
        self.insert_function(&signature.name, id);

        self.push_scope();
        let mut generics = vec![];
        for generic in &function.signature.node.generics {
            generics.push(self.insert_generic(generic.name.to_string()));
        }

        if function.signature.node.name.as_str() == "gen" {
            print_breakpoint!();
        }

        let parameters = signature
            .parameters
            .iter()
            .map(
                |NamedTupleElement {
                     name,
                     ty,
                     default,
                     node_id: _,
                 }| {
                    let ty = self.lower_type(ty);
                    let local = self.id_generator.alloc_local();
                    self.insert_parameter(name, local, ty);

                    let default = default.as_ref().map(|value| self.lower_expression(value));
                    hir::Parameter { local, ty, default }
                },
            )
            .collect();

        let body = match function.signature.node.external {
            Some(language) => hir::FunctionBody::External(language),
            None => hir::FunctionBody::Internal(self.lower_block(&function.block)),
        };

        let return_type = match self.lower_type(&signature.return_type) {
            hir::LazyTypeId::Known(type_id) => type_id,
            hir::LazyTypeId::Infer(_) => {
                self.log_error(SoulError::new(
                    "function return type should be known",
                    SoulErrorKind::TypeInferenceError,
                    Some(function.signature.span),
                ));
                TypeId::error()
            }
        };

        self.pop_scope();

        let hir_function = hir::Function {
            id,
            body,
            generics,
            parameters,
            return_type,
            name: signature.name.clone(),
            kind: signature.function_kind,
        };
        self.tree.nodes.functions.insert(id, hir_function);
        id
    }

    fn insert_function(&mut self, name: &Ident, function: FunctionId) {
        let scope = match self.scopes.last_mut() {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "tryed to insert_local in global scope",
                    Some(name.span)
                ));
                return;
            }
        };

        self.tree.info.spans.functions.insert(function, name.span);
        scope.functions.insert(name.to_string(), function);
    }
}
