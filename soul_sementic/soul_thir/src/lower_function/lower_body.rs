use crate::lower_function::LowerFunctionContext;
use hir_model as hir;
use soul_utils::{soul_error_internal, span::Span};
use thir_model as thir;

impl<'a> LowerFunctionContext<'a> {
    pub(crate) fn lower_body(&mut self, body_id: hir::BodyId) -> thir::Body {
        let (block, span) = match self.hir.root.bodies.get(body_id) {
            Some(hir::Body::Block(block, span)) => (block, *span),
            Some(hir::Body::Expression(value, span)) => {
                return self.lower_expression_body(*value, *span);
            }
            None => {
                self.log_error(soul_error_internal!(
                    format!("BodyId({}) not found", body_id.display()),
                    None
                ));
                return thir::Body::new(
                    thir_model::BodyKind {
                        statements: vec![],
                        tail: None,
                    },
                    Span::default_const(),
                );
            }
        };

        let mut statements = vec![];
        let mut tail = None;

        for statement in block.statements.values() {
            let thir_statement = match &statement.node {
                hir::StatementKind::Expression(value) => {
                    let id = self.lower_expression(value.expression);
                    tail = Some(id);
                    thir::Statement::with_meta_data(
                        thir::StatementKind::Expression(id),
                        statement.get_meta_data().clone(),
                    )
                }
                _ => self.lower_statement(statement),
            };

            statements.push(thir_statement);
        }

        thir::Body::new(
            thir::BodyKind{
                statements,
                tail,
            }, 
            span,
        )
    }

    pub(crate) fn lower_expression_body(
        &mut self,
        value: hir::ExpressionId,
        span: Span,
    ) -> thir::Body {
        let expression = self.lower_expression(value);
        let statement = thir::Statement::new(thir::StatementKind::Expression(expression), span);
        let kind = thir::BodyKind {
            statements: vec![statement],
            tail: Some(expression),
        };
        thir::Body::new(kind, span)
    }
}
