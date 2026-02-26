use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(crate) fn lower_function(&mut self, function: &ast::Function) -> hir::FunctionId {
        let id = self.id_generator.alloc_function();
        let signature = &function.signature.node;
        self.insert_function(&signature.name, id);

        self.push_scope();

        let parameters = signature
            .parameters
            .iter()
            .map(|(name, ty, _node_id)| {
                let ty = self.lower_type(ty);
                let local = self.id_generator.alloc_local();
                self.insert_local(name, local, ty);
                hir::Parameter { local, ty }
            })
            .collect();

        let body = self.lower_block(&function.block);

        self.pop_scope();

        let hir_function = hir::Function {
            id,
            body,
            parameters,
            name: signature.name.clone(),
            kind: signature.function_kind,
            return_type: self.lower_type(&signature.return_type),
        };
        self.hir.functions.insert(id, hir_function);
        id
    }
}
