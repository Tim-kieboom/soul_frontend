use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub(super) fn lower_while(
        &mut self,
        hir_condition: Option<hir::ExpressionId>,
        body_id: hir::BlockId,
        is_end: &mut bool,
    ) -> mir::Operand {
        let prev_finish = self.current.loop_finish;
        let prev_continue = self.current.loop_continue;

        let parent_bb = self.expect_current_block();

        let returnable = self.tree.blocks[parent_bb].returnable;

        let join_bb = self.new_block();
        self.current.loop_finish = Some(join_bb);
        self.tree.blocks[join_bb].returnable = returnable;

        let loop_bb = self.new_block();

        let condition_bb = self.new_block();
        self.current.block = Some(condition_bb);
        self.current.loop_continue = Some(condition_bb);
        self.insert_terminator(parent_bb, mir::Terminator::Goto(condition_bb));

        match hir_condition {
            Some(hir_condition) => {
                let condition = self.lower_operand(hir_condition).pass(is_end);
                self.insert_terminator(
                    condition_bb,
                    mir::Terminator::If {
                        condition,
                        then: loop_bb,
                        arm: join_bb,
                    },
                );
            }
            None => self.insert_terminator(condition_bb, mir::Terminator::Goto(loop_bb)),
        }

        self.insert_terminator(loop_bb, mir::Terminator::Goto(condition_bb));
        self.lower_block(body_id, loop_bb);

        self.current.block = Some(join_bb);
        self.current.loop_finish = prev_finish;
        self.current.loop_continue = prev_continue;
        mir::Operand::new(
            self.hir_response.typed.types_table.none_type,
            mir::OperandKind::None,
        )
    }

    pub(super) fn lower_if(
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
        self.lower_arm(then_block, then, after_if, ty, temp, is_end);

        let arm = match else_block {
            Some(arm_block) => {
                let arm = self.new_block();
                self.lower_arm(arm_block, arm, after_if, ty, temp, is_end);
                arm
            }
            None => after_if,
        };

        self.insert_terminator(
            parent,
            mir::Terminator::If {
                condition,
                then,
                arm,
            },
        );
        self.current.block = Some(after_if);
        mir::Operand::new(
            ty,
            match temp {
                Some(temp_id) => mir::OperandKind::Temp(*temp_id),
                None => mir::OperandKind::None,
            },
        )
    }

    fn lower_arm(
        &mut self,
        hir_block: hir::BlockId,
        arm: mir::BlockId,
        join: mir::BlockId,
        ty: hir::TypeId,
        temp: &mut Option<mir::TempId>,
        is_end: &mut bool,
    ) {
        self.current.block = Some(arm);
        let value = self.lower_block(hir_block, arm).pass(is_end);
        let end_block = self.expect_current_block();

        if let Some(value) = value {
            let temp_id = match temp {
                Some(id) => *id,
                None => {
                    let id = self.new_temp(ty);
                    *temp = Some(id);
                    id
                }
            };

            let place = self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp_id), ty));

            self.push_statement_from(
                mir::Statement::new(mir::StatementKind::Assign {
                    place,
                    value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
                }),
                end_block,
            );
        }

        if matches!(
            self.tree.blocks[end_block].terminator,
            mir::Terminator::Unreachable
        ) {
            self.insert_terminator(end_block, mir::Terminator::Goto(join));
        }
    }
}
