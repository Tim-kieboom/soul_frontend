use hir_model::{Statement, StatementKind};

use crate::{TypedContext, model::InferType};

impl<'a> TypedContext<'a> {
    pub(crate) fn infer_statement(&mut self, statement: &Statement) -> Option<InferType> {
        match &statement.node {
            StatementKind::Import(_) => todo!("impl infer import"),
            StatementKind::Assign(assign) => {
                let lplace = self.infer_place(assign.left);
                let rtype = self.infer_rvalue(assign.right);
                self.unify(lplace.get_type(), &rtype, statement.get_span());
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
            StatementKind::Expression(expression) => {
                Some(self.infer_rvalue(expression.expression))
            }
        }
    }
}