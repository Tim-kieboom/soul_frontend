use hir::{ExpressionId, LazyTypeId};
use soul_utils::{
    Ident, span::{ItemMetaData, Span}
};

use crate::{HirContext, create_local_name};

impl<'a> HirContext<'a> {
    pub(super) fn lower_array(
        &mut self,
        id: hir::ExpressionId,
        array: &ast::Array,
        span: Span,
    ) -> hir::Expression {

        let collection_type = array.collection_type
            .as_ref()
            .map(|ty| self.lower_type(ty, span));
        
        let element_type = array.element_type
            .as_ref()
            .map(|ty| self.lower_type(ty, span));

        let values = array.values
        .iter()
        .map(|value| self.lower_expression(value))
        .collect();
    
        let hir_array = hir::Array {
            collection_type,
            element_type,
            values,
        };

        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, span),
            kind: hir::ExpressionKind::Array(hir_array),
        }
    }

    pub(super) fn insert_desugar_variable(
        &mut self,
        variable: hir::Variable,
        ty: LazyTypeId,
        value: ExpressionId,
        span: Span,
    ) {
        let name = Ident::new(create_local_name(variable.local), span);

        self.insert_temp(&name, variable.local, ty, value);

        match self.current.body {
            crate::CurrentBody::Global => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let kind = hir::GlobalKind::InternalVariable(variable);
                let root_id = self.current.module;
                self.tree.nodes.modules[root_id]
                    .globals
                    .push(hir::Global::new(kind, id));
            }
            crate::CurrentBody::Block(block_id) => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let kind = hir::StatementKind::Variable(variable);
                self.insert_in_block(block_id, hir::Statement::new(kind, id));
            }
        }
    }
}