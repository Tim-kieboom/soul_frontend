use hir::{ComplexLiteral, LocalInfo, LocalKind};
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
    pub(crate) expression_value_id: Option<hir::ExpressionId>,
}

impl<'a> MirContext<'a> {
    pub(crate) fn lower_statement(
        &mut self,
        statement: &hir::Statement,
    ) -> EndBlock<StatementResponse> {
        let statement_id = statement.id;
        let is_end = &mut false;

        let mut last_operand = None;
        let mut last_expression_id = None;

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
                last_expression_id = Some(*value);
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
            expression_value_id: last_expression_id,
        };
        EndBlock::new(response, is_end)
    }

    pub(crate) fn lower_variable(&mut self, variable: &hir::Variable, is_end: &mut bool) {
        let local_info = &self.hir_response.hir.nodes.locals[variable.local];
        let (should_assign, place_kind) = self.lower_variable_place(variable, local_info);

        if !should_assign {
            return;
        }

        if let LocalKind::Variable(Some(value)) = local_info.kind {
            let operand = self.lower_operand(value).pass(is_end);
            let place =
                self.new_place(mir::Place::new(place_kind, self.local_type(variable.local)));

            let statement = mir::Statement::new(mir::StatementKind::Assign {
                place,
                value: mir::Rvalue::new(mir::RvalueKind::Operand(operand)),
            });

            self.push_statement(statement);
        }
    }

    fn lower_variable_place(
        &mut self,
        variable: &hir::Variable,
        local_info: &LocalInfo,
    ) -> (bool, mir::PlaceKind) {
        const SHOULD_ASSIGN: bool = true;

        if local_info.is_temp() {
            return (
                SHOULD_ASSIGN,
                mir::PlaceKind::Temp(match self.temp_remap.get(variable.local) {
                    Some(val) => *val,
                    None => self.new_temp(self.local_type(variable.local)),
                }),
            );
        }

        let local = match self.local_remap.get(variable.local) {
            Some(val) => *val,
            None => {
                let literal = self.try_get_variable_literal(local_info);
                self.new_local(variable.local, self.local_type(variable.local), literal)
            }
        };

        let should_assign = self.tree.locals[local].is_runtime();
        (should_assign, mir::PlaceKind::Local(local))
    }

    fn try_get_variable_literal(&mut self, local_info: &LocalInfo) -> Option<ComplexLiteral> {
        match local_info.kind {
            LocalKind::Variable(Some(value)) => {
                let literal = self.get_expression_literal(value)?;
                if literal.is_mutable() {
                    None
                } else {
                    Some(literal)
                }
            }
            _ => None,
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
            value: mir::Rvalue::new(mir::RvalueKind::Operand(value)),
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
        | hir::ExpressionKind::Sizeof(_)
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
