use ast::NamedTupleElement;
use soul_utils::{
    ids::{FunctionId, IdAlloc},
    soul_error_internal,
};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(crate) fn lower_function(&mut self, function: &ast::Function) -> FunctionId {
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
            let id = self.id_generator.alloc_generic();
            self.insert_generic(&generic.name, id);
            generics.push(id);
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

        let return_type = self.lower_type(&signature.return_type);
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
        self.hir.functions.insert(id, hir_function);
        id
    }
}
