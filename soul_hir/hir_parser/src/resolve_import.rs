use crate::HirContext;

impl<'a> HirContext<'a> {
    pub fn resolve_import(&mut self, import: &ast::Import) {
        for path in &import.paths {
            let module_id = self
                .hir
                .imports
                .insert(&mut self.id_generator.module, path.module.clone());

            let imports = match self.current_body {
                crate::CurrentBody::Global => &mut self.hir.root.imports,
                crate::CurrentBody::Block(block_id) => &mut self.hir.blocks[block_id].imports,
            };

            imports.push(hir::Import {
                module: module_id,
                kind: path.kind.clone(),
            });
        }
    }
}
