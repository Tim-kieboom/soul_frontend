use hir::{ExpressionId, StatementId};
use soul_utils::{
    Ident,
    span::{ItemMetaData, Span},
};

use crate::{HirContext, create_local_name};

impl<'a> HirContext<'a> {
    pub(crate) fn alloc_expression(&mut self, span: Span) -> ExpressionId {
        let id = self.id_generator.alloc_expression();
        self.store_expression_data(id, span);
        id
    }

    pub(crate) fn alloc_statement(&mut self, meta_data: &ItemMetaData, span: Span) -> StatementId {
        let id = self.id_generator.alloc_statement();
        self.store_statement_data(id, meta_data, span);
        id
    }

    pub(crate) fn insert_expression(
        &mut self,
        id: ExpressionId,
        expression: hir::Expression,
    ) -> ExpressionId {
        self.hir.expressions.insert(id, expression);
        id
    }

    pub(crate) fn insert_global(&mut self, global: hir::Global) -> StatementId {
        let id = global.get_id();
        self.hir.root.globals.push(global);
        id
    }

    pub(crate) fn insert_desugar_variable(&mut self, variable: hir::Variable, span: Span) {
        let name = Ident::new(create_local_name(variable.local), span);
        self.insert_local(&name, variable.local, variable.ty);

        match self.current_body {
            crate::CurrentBody::Global => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let global = hir::Global::InternalVariable(variable, id);
                self.hir.root.globals.push(global);
            }
            crate::CurrentBody::Block(block_id) => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let statement = hir::Statement::Variable(variable, id);
                self.insert_in_block(block_id, statement);
            }
        }
    }

    pub(crate) fn insert_desugar_assignment(&mut self, assign: hir::Assign, span: Span) {
        match self.current_body {
            crate::CurrentBody::Global => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let global = hir::Global::InternalAssign(assign, id);
                self.hir.root.globals.push(global);
            }
            crate::CurrentBody::Block(block_id) => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let statement = hir::Statement::Assign(assign, id);
                self.insert_in_block(block_id, statement);
            }
        }
    }

    fn store_statement_data(&mut self, id: StatementId, meta_data: &ItemMetaData, span: Span) {
        self.hir.spans.statements.insert(id, span);
        self.hir.meta_data.statements.insert(id, meta_data.clone());
    }

    fn store_expression_data(&mut self, id: ExpressionId, span: Span) {
        self.hir.spans.expressions.insert(id, span);
    }
}
