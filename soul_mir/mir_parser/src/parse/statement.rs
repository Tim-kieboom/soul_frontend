use soul_utils::soul_error_internal;

use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub(crate) fn lower_statement(&mut self, statement: &hir::Statement) {
        let statement_id = statement.get_id();

        match statement {
            hir::Statement::Variable(variable, _) => {
                debug_assert!(self.local_remap.get(variable.local) == None);
                let local = self.new_local(variable.local, variable.ty);

                if let Some(value) = variable.value {
                    let operand = self.lower_operand(value);

                    let place = self.new_place(mir::Place::Local(local));
                    let statement = mir::Statement::new(mir::StatementKind::Assign {
                        place,
                        value: mir::Rvalue::new(mir::RvalueKind::Use(operand)),
                    });

                    self.push_statement(statement);
                }
            }
            hir::Statement::Assign(assign, _) => {
                let place = self.lower_place(&assign.place);
                let value = self.lower_operand(assign.value);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place,
                    value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
                });

                self.push_statement(statement);
            }
            hir::Statement::Expression { value, .. } => {
                let operand = self.lower_operand(*value);

                let statement = mir::Statement::new(mir::StatementKind::Eval(operand));
                self.push_statement(statement);
            }
            hir::Statement::Return(value, _) => {
                let operand = value.map(|val| self.lower_operand(val));
                let block = self.expect_current_block();

                self.insert_terminator(block, mir::Terminator::Return(operand));
                self.current_block = None;
            }
            hir::Statement::Continue(_)
            | hir::Statement::Fall(_, _)
            | hir::Statement::Break(_, _) => {
                let span = self.hir.spans.statements[statement_id];
                self.log_error(soul_error_internal!("statement not yet impl", Some(span)));
            }
        }
    }
}
