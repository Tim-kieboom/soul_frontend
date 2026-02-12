use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(crate) fn lower_function(&mut self, function: &ast::Function) -> hir::Function {
        let id = self.id_generator.alloc_function();
        let signature = &function.signature.node;
        self.insert_function(&signature.name, id);

        self.push_scope();

        let parameters = signature
            .parameters
            .iter()
            .map(|(name, ty, _node_id)| {
                let local = self.id_generator.alloc_local();
                self.insert_local(name, local);
                hir::Parameter {
                    local,
                    ty: self.lower_type(ty),
                }
            })
            .collect();

        let body = self.lower_block(&function.block);

        self.pop_scope();

        hir::Function {
            id,
            body,
            parameters,
            name: signature.name.clone(),
            kind: signature.function_kind,
            return_type: self.lower_type(&signature.return_type),
        }
    }
}
