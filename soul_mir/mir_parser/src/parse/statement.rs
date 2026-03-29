use hir::LocalKind;
use soul_utils::soul_error_internal;

use crate::{
    EndBlock, MirContext,
    mir::{self, OperandKind},
};

pub(crate) struct StatementResponse {
    /// insert terminator in block
    pub(crate) terminator: Option<mir::Terminator>,

    /// operand is Some() when statement is expressionStatement
    pub(crate) expression_operand: Option<mir::Operand>,
}

impl<'a> MirContext<'a> {
    pub(crate) fn lower_statement(
        &mut self,
        statement: &hir::Statement,
    ) -> EndBlock<StatementResponse> {
        let statement_id = statement.id;
        let is_end = &mut false;

        let mut last_operand = None;

        let span = self.statement_span(statement_id);
        let terminator = match &statement.kind {
            hir::StatementKind::Variable(variable) => {
                self.lower_variable(variable, is_end);
                None
            }
            hir::StatementKind::Assign(assign) => {
                self.lower_assign(assign, is_end);
                None
            }
            hir::StatementKind::Expression { value, .. } => {
                let operand = self.lower_operand(*value).pass(is_end);
                let kind = &self.hir_response.hir.nodes.expressions[*value].kind;

                if is_valid_statement_expression(kind)
                    && !matches!(operand.kind, mir::OperandKind::None)
                {
                    let statement = mir::Statement::new(mir::StatementKind::Eval(operand.clone()));
                    self.push_statement(statement);
                }

                last_operand = Some(operand);
                None
            }
            hir::StatementKind::Return(value) => {
                let operand = match value {
                    Some(val) => {
                        let operand = self.lower_operand(*val).pass(is_end);
                        if matches!(operand.kind, OperandKind::None) {
                            None
                        } else {
                            Some(operand)
                        }
                    }
                    None => None,
                };

                *is_end = true;
                Some(mir::Terminator::Return(operand))
            }
            hir::StatementKind::Continue => {
                let current = self.expect_current_block();
                match self.current.loop_continue {
                    Some(block_id) => {
                        *is_end = true;
                        self.insert_terminator(current, mir::Terminator::Goto(block_id));
                    }
                    _ => {
                        self.log_error(soul_error_internal!(
                            "`self.current.loop_continue` is None",
                            Some(span)
                        ));
                    }
                }

                None
            }
            hir::StatementKind::Break => {

                let current = self.expect_current_block();
                match self.current.loop_finish {
                    Some(bock_id) => {
                        *is_end = true;
                        self.insert_terminator(current, mir::Terminator::Goto(bock_id));
                    }
                    _ => {
                        self.log_error(soul_error_internal!(
                            "`self.current.loop_finish` is None",
                            Some(span)
                        ));
                    }
                }

                None
            }
            hir::StatementKind::Fall(_) => {
                let span = self.statement_span(statement_id);
                self.log_error(soul_error_internal!("statement not yet impl", Some(span)));
                None
            }
        };

        let response = StatementResponse {
            terminator,
            expression_operand: last_operand,
        };
        EndBlock::new(response, is_end)
    }

    pub(crate) fn lower_variable(&mut self, variable: &hir::Variable, is_end: &mut bool) {
        
        let should_assign;
        let local_info = &self.hir_response.hir.nodes.locals[variable.local];
        let place_kind = if local_info.is_temp() {
            should_assign = true;
            mir::Place::Temp(match self.temp_remap.get(variable.local) {
                Some(val) => *val,
                None => self.new_temp(self.local_type(variable.local)),
            })
        } else {

            let literal = match local_info.kind {
                LocalKind::Variable(Some(value)) => self.get_expression_literal(value).cloned(),
                _ => None,
            };

            let local = match self.local_remap.get(variable.local) {
                Some(val) => *val,
                None => self.new_local(variable.local, self.local_type(variable.local), literal),
            };

            should_assign = self.tree.locals[local].is_runtime();
            mir::Place::Local(local)
        };

        if !should_assign {
            return
        }

        if let LocalKind::Variable(Some(value)) = local_info.kind {
            let operand = self.lower_operand(value).pass(is_end);
            let place = self.new_place(place_kind);

            let statement = mir::Statement::new(mir::StatementKind::Assign {
                place,
                value: mir::Rvalue::new(mir::RvalueKind::Use(operand)),
            });

            self.push_statement(statement);
        }
    }

    pub(crate) fn lower_assign(&mut self, assign: &hir::Assign, is_end: &mut bool) {
        let place = self.lower_place(assign.place).pass(is_end);
        let value = self.lower_operand(assign.value).pass(is_end);
        if matches!(value.kind, mir::OperandKind::None) {
            return;
        }

        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place,
            value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
        });

        self.push_statement(statement);
    }
}

fn is_valid_statement_expression(kind: &hir::ExpressionKind) -> bool {
    match kind {
        hir::ExpressionKind::Null
        | hir::ExpressionKind::Error
        | hir::ExpressionKind::Load(_)
        | hir::ExpressionKind::Local(_)
        | hir::ExpressionKind::DeRef(_)
        | hir::ExpressionKind::Literal(_)
        | hir::ExpressionKind::Ref { .. }
        | hir::ExpressionKind::Cast { .. }
        | hir::ExpressionKind::Unary { .. }
        | hir::ExpressionKind::Binary { .. }
        | hir::ExpressionKind::StructConstructor { .. }
        | hir::ExpressionKind::InnerRawStackArray { .. } => false,

        hir::ExpressionKind::Block(_)
        | hir::ExpressionKind::Function(_)
        | hir::ExpressionKind::If { .. }
        | hir::ExpressionKind::Call { .. }
        | hir::ExpressionKind::While { .. } => true,
    }
}
