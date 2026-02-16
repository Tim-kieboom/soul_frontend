use crate::HirContext;

impl<'a> HirContext<'a> {
    pub fn lower_variable(&mut self, variable: &ast::Variable) -> hir::Variable {
        let ty = match &variable.ty {
            ast::VarTypeKind::NonInveredType(soul_type) => self.lower_type(soul_type),
            ast::VarTypeKind::InveredType(type_modifier) => {
                self.new_infer_with_modifier(*type_modifier)
            }
        };

        let value = match &variable.initialize_value {
            Some(val) => Some(self.lower_expression(val)),
            None => None,
        };

        let local = self.id_generator.alloc_local();
        self.insert_local(&variable.name, local, ty);

        hir::Variable { ty, value, local }
    }
}
