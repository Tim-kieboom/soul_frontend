use hir::IdAlloc;
use soul_utils::{soul_error_internal, span::Span};

use crate::{EndBlock, MirContext, mir};

impl<'a> MirContext<'a> {
    pub(crate) fn lower_operand(&mut self, value_id: hir::ExpressionId) -> EndBlock<mir::Operand> {
        let value = &self.hir.expressions[value_id];
        let span = self.hir.spans.expressions[value_id];
        let ty = self.expression_ty(value_id);
        let is_end = &mut false;

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
                self.lower_call(*function, callee, hir_arguments, ty, span).pass(is_end)
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

            hir::ExpressionKind::Ref { place, mutable } => {
                let ty = self.types.places[place.node.get_id()];

                let place_id = self.lower_place(place).pass(is_end);

                mir::Operand::new(ty, mir::OperandKind::Ref{place: place_id, mutable: *mutable})
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

            hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.lower_if(*condition, *then_block, *else_block, value.ty, is_end)
            }

            // hir::ExpressionKind::While { condition, body } => {
            //     self.lower_while(*condition, *body);
            //     mir::Operand::new(mir::OperandKind::None)
            // }
            _ => todo!(),
        };

        EndBlock::new(operand, is_end)
    }

    pub(crate) fn lower_call(
        &mut self, 
        function: hir::FunctionId, 
        callee: &Option<hir::ExpressionId>, 
        hir_arguments: &Vec<hir::ExpressionId>, 
        ty: hir::TypeId, 
        span: Span,
    ) -> EndBlock<mir::Operand> {
        let is_end = &mut false;
        
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
        self.tree.blocks[next_block].returnable = self.tree.blocks[current].returnable;

        let return_place = temp.map(|val| self.new_place(mir::Place::Temp(val)));
        self.insert_terminator(
            current,
            mir::Terminator::Call {
                id: function,
                arguments,
                return_place,
                next: next_block,
            },
        );

        self.current.block = Some(next_block);

        let operand = match temp {
            Some(val) => mir::Operand::new(ty, mir::OperandKind::Temp(val)),
            None => mir::Operand::new(ty, mir::OperandKind::None),
        };

        EndBlock::new(operand, is_end)
    }

    fn lower_if(
        &mut self, 
        hir_condition: hir::ExpressionId, 
        then_block: hir::BlockId, 
        else_block: Option<hir::BlockId>, 
        ty: hir::TypeId, 
        is_end: &mut bool,
    ) -> mir::Operand {
        let parent = self.expect_current_block();
        let returnable = self.tree.blocks[parent].returnable;

        let temp = &mut None;

        let after_if = self.new_block();
        self.tree.blocks[after_if].returnable = returnable;

        let then = self.new_block();
        let condition = self.lower_operand(hir_condition).pass(is_end);
        self.lower_arm(parent, then_block, then, after_if, ty, temp, is_end);

        let arm = match else_block {
            Some(arm_block) => {
                let arm = self.new_block();
                self.lower_arm(parent, arm_block, arm, after_if, ty, temp, is_end);
                arm
            }
            None => after_if,
        };

        self.insert_terminator(parent, mir::Terminator::If { condition, then, arm });
        self.current.block = Some(after_if);
        mir::Operand::new(
            ty, 
            match temp {
                Some(temp_id) => mir::OperandKind::Temp(*temp_id),
                None => mir::OperandKind::None,
            }
        )
    } 

    fn lower_arm(
        &mut self, 
        parent: mir::BlockId, 
        hir_block: hir::BlockId, 
        arm: mir::BlockId, 
        join: mir::BlockId, 
        ty: hir::TypeId, 
        temp: &mut Option<mir::TempId>,
        is_end: &mut bool,
    ) {
        match self.lower_block(hir_block, arm).pass(is_end) {
            Some(value) => {
                let temp_id = match temp {
                    Some(val) => *val,
                    None => {
                        let id = self.new_temp(ty);
                        *temp = Some(id);
                        id
                    }
                };

                let place = self.new_place(mir::Place::Temp(temp_id));
                self.push_statement_from(
                    mir::Statement::new(mir::StatementKind::Assign { 
                        place, 
                        value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
                    }), 
                    parent,
                );
            }
            None => (),
        }

        self.insert_terminator(arm, mir::Terminator::Goto(join));
    }
}
