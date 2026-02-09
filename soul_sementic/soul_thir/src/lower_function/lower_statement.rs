use crate::lower_function::LowerFunctionContext;
use hir_model as hir;
use soul_utils::{soul_error_internal, span::Spanned};
use thir_model::{self as thir, LocalKind};

impl<'a> LowerFunctionContext<'a> {

    pub(super) fn lower_statement(&mut self, statement: &hir::Statement) -> thir::Statement {

        let meta_data = statement.get_meta_data().clone();
        match &statement.node {
            hir_model::StatementKind::Import(_) => {
                self.log_error(soul_error_internal!(
                    "import should not be reachable in thir",
                    Some(meta_data.span)
                ));
                thir::Statement::with_meta_data(thir::StatementKind::Continue, meta_data)
            }
            hir_model::StatementKind::Assign(assign) => {
                let place = self.lower_place(assign.left);
                let value = self.lower_expression(assign.right);

                thir::Statement::with_meta_data(
                    thir::StatementKind::Assign { place, value },
                    meta_data,
                )
            }
            hir_model::StatementKind::Variable(variable) => {
                let local = self.id_generator.alloc_local();
                let ty = self.typed_context.types[variable.id].clone();

                let local_value = Spanned::new(
                    thir::InnerLocal {
                        ty,
                        kind: LocalKind::Variable,
                    },
                    meta_data.span,
                );
                self.locals.insert(local, local_value);

                let value = variable.value.map(|id| self.lower_expression(id));
                thir::Statement::with_meta_data(
                    thir::StatementKind::Variable { local, value },
                    meta_data,
                )
            }
            hir_model::StatementKind::Function(_) => todo!("nested functions not yet impl"),
            hir_model::StatementKind::Expression(expression) => {
                let thir_id = self.lower_expression(expression.expression);
                thir::Statement::with_meta_data(
                    thir::StatementKind::Expression(thir_id), 
                    meta_data,
                )
            }
        }
    }
}
