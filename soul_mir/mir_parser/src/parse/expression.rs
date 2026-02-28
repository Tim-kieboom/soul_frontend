use hir::IdAlloc;
use soul_utils::soul_error_internal;

use crate::{EndBlock, MirContext, mir};

impl<'a> MirContext<'a> {
    pub(crate) fn lower_operand(&mut self, value_id: hir::ExpressionId) -> EndBlock<mir::Operand> {
        let value = &self.hir.expressions[value_id];
        let span = self.hir.spans.expressions[value_id];
        let is_end = &mut false;
        let ty = self.expression_ty(value_id);

        let operand = match &value.kind {
            hir::ExpressionKind::Literal(literal) => {
                mir::Operand::new(ty, mir::OperandKind::Comptime(literal.clone()))
            }
            hir::ExpressionKind::Local(local_id) => {
                let local_type = self.types.locals[*local_id];
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
                mir::Operand::new(local_type, mir::OperandKind::Local(id))
            }
            hir::ExpressionKind::Unary {
                operator,
                expression,
            } => {
                let inner = self.lower_operand(*expression).pass(is_end);

                let temp = self.new_temp(ty);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::Temp(temp)),
                    value: mir::Rvalue::new(mir::RvalueKind::Unary {
                        operator: operator.clone(),
                        value: inner,
                    }),
                });
                self.push_statement(statement);
                mir::Operand::new(ty, mir::OperandKind::Temp(temp))
            }
            hir::ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.lower_operand(*left).pass(is_end);
                let right = self.lower_operand(*right).pass(is_end);

                let temp = self.new_temp(ty);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::Temp(temp)),
                    value: mir::Rvalue::new(mir::RvalueKind::Binary {
                        left,
                        operator: operator.clone(),
                        right,
                    }),
                });
                self.push_statement(statement);
                mir::Operand::new(ty, mir::OperandKind::Temp(temp))
            }
            hir::ExpressionKind::Call {
                function,
                callee,
                arguments: hir_arguments,
            } => {
                if callee.is_some() {
                    self.log_error(soul_error_internal!(
                        "function call callee not yet impl",
                        Some(span)
                    ));
                }

                let mut arguments = vec![];
                for arg in hir_arguments {
                    arguments.push(
                        self.lower_operand(*arg).pass(is_end)
                    );
                }

                let temp = if self.get_type(ty).is_none() {
                    None
                } else {
                    Some(self.new_temp(ty))
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

                self.current.block = Some(next_block);

                match temp {
                    Some(val) => mir::Operand::new(ty, mir::OperandKind::Temp(val)),
                    None => mir::Operand::new(ty, mir::OperandKind::None),
                }
            }
            hir::ExpressionKind::Block(block_id) => {
                let main_body = self.expect_current_block();
                self.lower_block(*block_id, main_body).pass(is_end);

                let operand = match self.hir.blocks[*block_id].terminator {
                    Some(terminator) => {
                        let inner = self.lower_operand(terminator).pass(is_end);
                        let terminator_type = self.expression_ty(terminator);
                        let temp = self.new_temp(terminator_type);

                        let place = self.new_place(mir::Place::Temp(temp));
                        self.push_statement(mir::Statement::new(mir::StatementKind::Assign {
                            place,
                            value: mir::Rvalue::new(mir::RvalueKind::Use(inner)),
                        }));

                        mir::Operand::new(ty, mir::OperandKind::Temp(temp))
                    }
                    None => mir::Operand::new(ty, mir::OperandKind::None),
                };

                operand
            }

            hir::ExpressionKind::Null => todo!("ExpressionKind::Null  not yet impl in mir"),
            hir::ExpressionKind::Function(_) => {
                todo!("ExpressionKind::Function not yet impl in mir")
            }

            hir::ExpressionKind::Load(place) => {

                let place_id = self.lower_place(place).pass(is_end);
                let operand = match &self.tree.places[place_id] {
                    mir::Place::Local(local) => {
                        mir::Operand::new(ty, mir::OperandKind::Local(*local))
                    }
                    _ => mir::Operand::new(
                        ty,
                        mir::OperandKind::Temp(self.place_to_temp(place_id, ty)),
                    ),
                };

                let temp = self.new_temp(ty);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::Temp(temp)),
                    value: mir::Rvalue::new(mir::RvalueKind::Use(operand)),
                });

                self.push_statement(statement);
                mir::Operand::new(ty, mir::OperandKind::Temp(temp))
            }

            hir::ExpressionKind::DeRef(inner) => {
                let ptr = self.lower_operand(*inner).pass(is_end);
                let temp = self.new_temp(ty);

                let stmt = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::Temp(temp)),
                    value: mir::Rvalue::new(mir::RvalueKind::Use(ptr)),
                });

                self.push_statement(stmt);
                mir::Operand::new(ty, mir::OperandKind::Temp(temp))
            }

            hir::ExpressionKind::Ref { place, mutable: _ } => {
                let ty = self.types.places[place.node.get_id()];

                let place_id = self.lower_place(place).pass(is_end);
                let temp = self.new_temp(ty);

                let stmt = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::Temp(temp)),
                    value: mir::Rvalue::new(mir::RvalueKind::Use(mir::Operand::new(
                        ty,
                        mir::OperandKind::Temp(self.place_to_temp(place_id, ty)),
                    ))),
                });

                self.push_statement(stmt);
                mir::Operand::new(ty, mir::OperandKind::Temp(temp))
            }

            hir::ExpressionKind::Cast { value: inner, .. } => self.lower_operand(*inner).pass(is_end),

            hir::ExpressionKind::InnerRawStackArray { ty:_, len } => {
                let len = self.lower_operand(*len).pass(is_end);
                let temp = self.new_temp(ty);

                let stmt = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::Temp(temp)),
                    value: mir::Rvalue::new(mir::RvalueKind::StackAlloc { ty, len }),
                });

                self.push_statement(stmt);
                mir::Operand::new(ty, mir::OperandKind::Temp(temp))
            }

            _ => todo!(),
            // hir::ExpressionKind::If {
            //     condition,
            //     then_block,
            //     else_block,
            // } => {
            //     self.lower_if(*condition, *then_block, *else_block, value.ty)
            // }

            // hir::ExpressionKind::While { condition, body } => {
            //     self.lower_while(*condition, *body);
            //     mir::Operand::new(mir::OperandKind::None)
            // }
        };

        EndBlock::new(operand, is_end)
    }
}
