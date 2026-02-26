use hir::IdAlloc;
use soul_utils::soul_error_internal;

use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub(crate) fn lower_operand(&mut self, value_id: hir::ExpressionId) -> mir::Operand {
        let value = &self.hir.expressions[value_id];
        let span = self.hir.spans.expressions[value_id];

        match &value.kind {
            hir::ExpressionKind::Literal(literal) => {
                mir::Operand::new(mir::OperandKind::Comptime(literal.clone()))
            }
            hir::ExpressionKind::Local(local_id) => {
                let id = match self.local_remap.get(*local_id) {
                    Some(val) => *val,
                    None => {
                        self.log_error(soul_error_internal!(
                            format!("local_remap could not find {:?}", local_id),
                            Some(span)
                        ));
                        mir::LocalId::error()
                    }
                };
                mir::Operand::new(mir::OperandKind::Local(id))
            }
            hir::ExpressionKind::Unary {
                operator,
                expression,
            } => {
                let ty = self.hir.expressions[*expression].ty;
                let value = self.lower_operand(*expression);

                let temp = self.new_temp(ty);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::Temp(temp)),
                    value: mir::Rvalue::new(mir::RvalueKind::Unary {
                        operator: operator.clone(),
                        value,
                    }),
                });
                self.push_statement(statement);
                mir::Operand::new(mir::OperandKind::Temp(temp))
            }
            hir::ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.lower_operand(*left);
                let right = self.lower_operand(*right);

                let temp = self.new_temp(value.ty);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::Temp(temp)),
                    value: mir::Rvalue::new(mir::RvalueKind::Binary {
                        left,
                        operator: operator.clone(),
                        right,
                    }),
                });
                self.push_statement(statement);
                mir::Operand::new(mir::OperandKind::Temp(temp))
            }
            hir::ExpressionKind::Call {
                function,
                callee,
                arguments,
            } => {
                if callee.is_some() {
                    self.log_error(soul_error_internal!(
                        "function call callee not yet impl",
                        Some(span)
                    ));
                }

                let arguments: Vec<_> = arguments
                    .iter()
                    .map(|arg| self.lower_operand(*arg))
                    .collect();

                let temp = if self.get_type(value.ty).is_none() {
                    None
                } else {
                    Some(self.new_temp(value.ty))
                };

                let current = self.expect_current_block();
                let next_block = self.new_block();

                let return_place = temp.map(|val| self.new_place(mir::Place::Temp(val)));
                self.insert_terminator(
                    current,
                    mir::Terminator::Call {
                        id: *function,
                        arguments,
                        return_place,
                        next: next_block,
                    },
                );

                self.current_block = Some(next_block);

                match temp {
                    Some(val) => mir::Operand::new(mir::OperandKind::Temp(val)),
                    None => mir::Operand::new(mir::OperandKind::None),
                }
            }
            hir::ExpressionKind::Block(block_id) => {
                let parent = self.expect_current_block();
                let new_block = self.new_block();

                self.insert_terminator(parent, mir::Terminator::Goto(new_block));
                self.current_block = Some(new_block);

                self.lower_block(*block_id, new_block);

                match self.hir.blocks[*block_id].terminator {
                    Some(terminator) => {
                        let value = self.lower_operand(terminator);
                        let ty = self.hir.expressions[terminator].ty;
                        let temp = self.new_temp(ty);

                        let place = self.new_place(mir::Place::Temp(temp));
                        self.push_statement(mir::Statement::new(mir::StatementKind::Assign {
                            place,
                            value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
                        }));

                        mir::Operand::new(mir::OperandKind::Temp(temp))
                    }
                    None => mir::Operand::new(mir::OperandKind::None),
                }
            }

            hir::ExpressionKind::Null
            | hir::ExpressionKind::Load(_)
            | hir::ExpressionKind::DeRef(_)
            | hir::ExpressionKind::If { .. }
            | hir::ExpressionKind::Ref { .. }
            | hir::ExpressionKind::Cast { .. }
            | hir::ExpressionKind::While { .. }
            | hir::ExpressionKind::Function(_)
            | hir::ExpressionKind::InnerRawStackArray { .. } => {
                todo!("expression kind is not yet impl")
            }
        }
    }
}
