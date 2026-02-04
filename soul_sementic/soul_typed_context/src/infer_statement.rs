use hir_model::{Statement, StatementKind};

use crate::{TypedContext, model::InferType};

impl<'a> TypedContext<'a> {
    pub(crate) fn infer_statement(&mut self, statement: &Statement) -> Option<InferType> {
        match &statement.node {
            StatementKind::Import(_) => todo!("impl infer import"),
            StatementKind::Assign(assign) => {
                let lplace = self.infer_place(assign.left);
                let mut rtype = self.infer_rvalue(assign.right);
                self.unify(
                    assign.right,
                    &rtype,
                    lplace.get_type(),
                    statement.get_span(),
                );

                if matches!(rtype, InferType::Known(_)) {
                    self.try_resolve_untyped_number(&mut rtype, None, statement.get_span());
                    match &rtype {
                        InferType::Known(hir_type) => {
                            self.try_resolve_untyped_var(&lplace, hir_type)
                        }
                        _ => unreachable!(),
                    };
                }
                None
            }
            StatementKind::Variable(variable) => {
                self.infer_variable(variable);
                None
            }
            StatementKind::Function(function) => {
                self.infer_function(function);
                None
            }
            StatementKind::Expression(expression) => Some(self.infer_rvalue(expression.expression)),
        }
    }
}
