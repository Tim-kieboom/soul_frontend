use soul_utils::soul_error_internal;

use crate::{EndBlock, MirContext, mir};

pub(crate) struct StatementResponse {
    pub(crate) terminator: Option<mir::Terminator>,
    pub(crate) block_operand: Option<mir::Operand>,
}

impl<'a> MirContext<'a> {
    pub(crate) fn lower_statement(&mut self, statement: &hir::Statement) -> EndBlock<StatementResponse> {
        let statement_id = statement.get_id();
        let is_end = &mut false; 

        let mut block_operand = None;

        let terminator = match statement {
            hir::Statement::Variable(variable, _) => {
                let local = match self.local_remap.get(variable.local) {
                    Some(val) => *val,
                    None => self.new_local(variable.local, variable.ty),
                };
                
                if let Some(value) = variable.value {
                    let operand = self.lower_operand(value).pass(is_end);

                    let place = self.new_place(mir::Place::Local(local));
                    let statement = mir::Statement::new(mir::StatementKind::Assign {
                        place,
                        value: mir::Rvalue::new(mir::RvalueKind::Use(operand)),
                    });

                    self.push_statement(statement);
                }
                None
            }
            hir::Statement::Assign(assign, _) => {
                let place = self.lower_place(&assign.place).pass(is_end);
                let value = self.lower_operand(assign.value).pass(is_end);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place,
                    value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
                });

                self.push_statement(statement);
                None
            }
            hir::Statement::Expression { value, .. } => {
                let operand = self.lower_operand(*value).pass(is_end);

                if !matches!(operand.kind, mir::OperandKind::None) {
                    let statement = mir::Statement::new(mir::StatementKind::Eval(operand));
                    self.push_statement(statement);
                } else {
                    block_operand = Some(operand);
                }
                
                None
            }
            hir::Statement::Return(value, _) => {
                let operand = value.map(|val| self.lower_operand(val).pass(is_end));

                *is_end = true;
                Some(mir::Terminator::Return(operand))
            }
            hir::Statement::Continue(_)
            | hir::Statement::Fall(_, _)
            | hir::Statement::Break(_, _) => {
                let span = self.hir.spans.statements[statement_id];
                self.log_error(soul_error_internal!("statement not yet impl", Some(span)));
                None
            }
        };

        let response = StatementResponse {
            terminator,
            block_operand,
        };
        EndBlock::new(response, is_end)
    }
}
