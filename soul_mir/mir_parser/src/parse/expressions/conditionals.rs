use ast::{ArrayKind, BinaryOperator, BinaryOperatorKind, Literal};
use hir::{ComplexLiteral, MatchPatternHir, TypeId, UnionId};
use typed_hir::ThirTypeKind;

use soul_utils::ids::IdAlloc;

use crate::{
    MirContext,
    mir::{self, BlockId},
};

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

        let end_block = self.expect_current_block();
        if matches!(
            self.tree.blocks[end_block].terminator,
            mir::Terminator::Unreachable
        ) {
            self.insert_terminator(end_block, mir::Terminator::Goto(condition_bb));
        }

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
        if let Some(target_place) = self.current.target_place {
            let parent = self.expect_current_block();
            let returnable = self.tree.blocks[parent].returnable;

            let then = self.new_block();
            let after_if = self.new_block();
            self.tree.blocks[after_if].returnable = returnable;

            let condition = self.lower_operand(hir_condition).pass(is_end);

            self.lower_arm_with_target(then_block, then, after_if, ty, target_place, is_end);

            let arm = match else_block {
                Some(arm_block) => {
                    let arm = self.new_block();
                    self.lower_arm_with_target(arm_block, arm, after_if, ty, target_place, is_end);
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
            return mir::Operand::new(ty, mir::OperandKind::None);
        }

        let parent = self.expect_current_block();
        let returnable = self.tree.blocks[parent].returnable;

        let then = self.new_block();
        let after_if = self.new_block();
        self.tree.blocks[after_if].returnable = returnable;

        let condition = self.lower_operand(hir_condition).pass(is_end);

        let temp = &mut None;
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

    pub(crate) fn lower_if_assignment(
        &mut self,
        hir_condition: hir::ExpressionId,
        then_block: hir::BlockId,
        else_block: Option<hir::BlockId>,
        ty: hir::TypeId,
        is_end: &mut bool,
        target_place: mir::PlaceId,
    ) {
        let parent = self.expect_current_block();
        let returnable = self.tree.blocks[parent].returnable;

        let then = self.new_block();
        let after_if = self.new_block();
        self.tree.blocks[after_if].returnable = returnable;

        let condition = self.lower_operand(hir_condition).pass(is_end);

        self.current.target_place = Some(target_place);

        self.lower_arm_with_target(then_block, then, after_if, ty, target_place, is_end);

        let arm = match else_block {
            Some(arm_block) => {
                let arm = self.new_block();
                self.lower_arm_with_target(arm_block, arm, after_if, ty, target_place, is_end);
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
        self.current.target_place = None;
    }

    fn lower_arm(
        &mut self,
        hir_block: hir::BlockId,
        arm: mir::BlockId,
        join: mir::BlockId,
        ty: hir::TypeId,
        temp: &mut Option<mir::TempId>,
        _is_end: &mut bool,
    ) {
        self.current.block = Some(arm);
        let arm_end = &mut false;
        let value = self.lower_block(hir_block, arm).pass(arm_end);
        let end_block = self.expect_current_block();

        if !*arm_end {
            if let Some(value) = value.filter(|value| !matches!(value.kind, mir::OperandKind::None))
            {
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
                        value: mir::Rvalue::new(mir::RvalueKind::Operand(value)),
                    }),
                    end_block,
                );
            }
        }

        if matches!(
            self.tree.blocks[end_block].terminator,
            mir::Terminator::Unreachable
        ) {
            self.insert_terminator(end_block, mir::Terminator::Goto(join));
        }
    }

    fn lower_arm_with_target(
        &mut self,
        hir_block: hir::BlockId,
        arm: mir::BlockId,
        join: mir::BlockId,
        _ty: hir::TypeId,
        target_place: mir::PlaceId,
        _is_end: &mut bool,
    ) {
        self.current.block = Some(arm);
        let arm_end = &mut false;
        let value = self.lower_block(hir_block, arm).pass(arm_end);
        let end_block = self.expect_current_block();

        if !*arm_end {
            if let Some(value) = value.filter(|value| !matches!(value.kind, mir::OperandKind::None))
            {
                self.push_statement_from(
                    mir::Statement::new(mir::StatementKind::Assign {
                        place: target_place,
                        value: mir::Rvalue::new(mir::RvalueKind::Operand(value)),
                    }),
                    end_block,
                );
            }
        }

        if matches!(
            self.tree.blocks[end_block].terminator,
            mir::Terminator::Unreachable
        ) {
            self.insert_terminator(end_block, mir::Terminator::Goto(join));
        }
    }

    pub(super) fn lower_match(
        &mut self,
        scrutinee: hir::ExpressionId,
        arms: &[hir::MatchArm],
        ty: hir::TypeId,
        is_end: &mut bool,
    ) -> mir::Operand {
        let scrutinee_op = self.lower_operand(scrutinee).pass(is_end);
        let scrutinee_ty = self.expression_type(scrutinee);

        let parent_bb = self.expect_current_block();
        let returnable = self.tree.blocks[parent_bb].returnable;

        let mut targets = Vec::new();
        let mut wildcard_arm: Option<(BlockId, hir::BlockId)> = None;
        let mut lit_arm_blocks = Vec::new();
        let mut array_arm_blocks = Vec::new();
        let mut string_arm_blocks = Vec::new();
        let mut constructor_arms: Vec<(BlockId, hir::BlockId, Option<hir::LocalId>, UnionId, usize)> =
            Vec::new();
        let mut binding_arms: Vec<(BlockId, hir::LocalId)> = Vec::new();
        let mut has_constructor = false;

        for arm in arms {
            let arm_bb = self.new_block();

            match &arm.pattern {
                hir::MatchPatternHir::Literal(lit) => {
                    match lit {
                        Literal::Int(val) => {
                            lit_arm_blocks.push((arm_bb, arm.body));
                            targets.push((*val, arm_bb));
                        }
                        Literal::Uint(val) => {
                            lit_arm_blocks.push((arm_bb, arm.body));
                            targets.push((*val as i128, arm_bb));
                        }
                        Literal::Char(val) => {
                            lit_arm_blocks.push((arm_bb, arm.body));
                            targets.push((*val as i128, arm_bb));
                        }
                        Literal::Str(_) => {
                            string_arm_blocks.push((arm_bb, arm.body, lit.clone()));
                        }
                        _ => {} // Float, Bool, Cstr not supported as match patterns
                    }
                }
                hir::MatchPatternHir::Wildcard => {
                    wildcard_arm = Some((arm_bb, arm.body));
                    lit_arm_blocks.push((arm_bb, arm.body));
                }
                hir::MatchPatternHir::Binding(local_id) => {
                    wildcard_arm = Some((arm_bb, arm.body));
                    binding_arms.push((arm_bb, *local_id));
                    lit_arm_blocks.push((arm_bb, arm.body));
                }
                hir::MatchPatternHir::Array(elements) => {
                    array_arm_blocks.push((arm_bb, arm.body, elements.clone()));
                }
                hir::MatchPatternHir::Constructor {
                    union_id,
                    variant_index,
                    binding,
                } => {
                    has_constructor = true;
                    targets.push((*variant_index as i128, arm_bb));
                    constructor_arms.push((arm_bb, arm.body, *binding, *union_id, *variant_index));
                    lit_arm_blocks.push((arm_bb, arm.body));
                }
            }
        }

        let discriminant = if has_constructor {
            let tag_type = self.hir_response.typed.types_table.index_type;
            let tag_temp = self.new_temp(tag_type);
            let tag_stmt = mir::Statement::new(mir::StatementKind::Assign {
                place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(tag_temp), tag_type)),
                value: mir::Rvalue::new(mir::RvalueKind::UnionTag {
                    value: scrutinee_op.clone(),
                }),
            });
            self.push_statement(tag_stmt);
            mir::Operand::new(tag_type, mir::OperandKind::Temp(tag_temp))
        } else {
            scrutinee_op.clone()
        };

        let join_bb = self.new_block();

        if wildcard_arm.is_none() {
            if has_constructor {
                wildcard_arm = Some((join_bb, arms.last().map(|a| a.body).unwrap()));
            } else {
                let else_bb = self.new_block();
                targets.push((0, else_bb));
                wildcard_arm = Some((else_bb, arms.last().map(|a| a.body).unwrap()));
            }
        }

        let otherwise = wildcard_arm.as_ref().unwrap().0;
        self.tree.blocks[join_bb].returnable = returnable;

        let switch_parent = if array_arm_blocks.is_empty() && string_arm_blocks.is_empty() {
            parent_bb
        } else {
            let ultimate_fallthrough = self.new_block();
            self.tree.blocks[ultimate_fallthrough].returnable = returnable;

            let string_entry = if !string_arm_blocks.is_empty() {
                self.lower_string_arm_chain(
                    scrutinee_op.clone(),
                    scrutinee_ty,
                    &string_arm_blocks,
                    ultimate_fallthrough,
                    returnable,
                )
            } else {
                ultimate_fallthrough
            };

            let chain_entry = if !array_arm_blocks.is_empty() {
                self.lower_array_arm_chain(
                    scrutinee_op.clone(),
                    scrutinee_ty,
                    &array_arm_blocks,
                    string_entry,
                    returnable,
                )
            } else {
                string_entry
            };

            self.insert_terminator(parent_bb, mir::Terminator::Goto(chain_entry));
            ultimate_fallthrough
        };

        if !targets.is_empty() {
            self.insert_terminator(
                switch_parent,
                mir::Terminator::SwitchInt {
                    discriminant,
                    targets,
                    otherwise,
                },
            );
        } else {
            self.insert_terminator(switch_parent, mir::Terminator::Goto(otherwise));
        }

        for (arm_bb, _hir_body, binding_local, union_id, variant_index) in &constructor_arms {
            if let Some(local_id) = binding_local {
                self.current.block = Some(*arm_bb);
                let variant_type = self
                    .hir_response
                    .typed
                    .types_map
                    .id_to_union(*union_id)
                    .map(|u| u.variant_types[*variant_index])
                    .unwrap_or(scrutinee_ty);
                let mir_local = self.new_local(*local_id, variant_type, None);
                let extract_temp = self.new_temp(variant_type);
                let extract_stmt = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(
                        mir::Place::new(mir::PlaceKind::Temp(extract_temp), variant_type),
                    ),
                    value: mir::Rvalue::new(mir::RvalueKind::UnionExtract {
                        value: scrutinee_op.clone(),
                    }),
                });
                self.push_statement(extract_stmt);
                let local_place = self.new_place(
                    mir::Place::new(mir::PlaceKind::Local(mir_local), variant_type),
                );
                let assign_stmt = mir::Statement::new(mir::StatementKind::Assign {
                    place: local_place,
                    value: mir::Rvalue::new(mir::RvalueKind::Operand(mir::Operand::new(
                        variant_type,
                        mir::OperandKind::Temp(extract_temp),
                    ))),
                });
                self.push_statement(assign_stmt);
            }
        }

        for (arm_bb, local_id) in &binding_arms {
            let local_type = scrutinee_op.ty;
            self.current.block = Some(*arm_bb);
            let mir_local = self.new_local(*local_id, local_type, None);
            let local_place = self.new_place(
                mir::Place::new(mir::PlaceKind::Local(mir_local), local_type),
            );
            let assign_stmt = mir::Statement::new(mir::StatementKind::Assign {
                place: local_place,
                value: mir::Rvalue::new(mir::RvalueKind::Operand(scrutinee_op.clone())),
            });
            self.push_statement(assign_stmt);
        }

        let all_arm_blocks: Vec<(BlockId, hir::BlockId)> = lit_arm_blocks
            .into_iter()
            .chain(array_arm_blocks.into_iter().map(|(bb, body, _)| (bb, body)))
            .chain(
                string_arm_blocks
                    .into_iter()
                    .map(|(bb, body, _)| (bb, body)),
            )
            .collect();

        let has_target_place = self.current.target_place.is_some();

        if has_target_place {
            let target_place = self.current.target_place.unwrap();
            for (arm_bb, hir_body) in all_arm_blocks {
                self.current.block = Some(arm_bb);
                let arm_end = &mut false;
                let value = self.lower_block(hir_body, arm_bb).pass(arm_end);
                let end_block = self.expect_current_block();

                if !*arm_end {
                    if let Some(value) = value.filter(|v| !matches!(v.kind, mir::OperandKind::None))
                    {
                        self.push_statement_from(
                            mir::Statement::new(mir::StatementKind::Assign {
                                place: target_place,
                                value: mir::Rvalue::new(mir::RvalueKind::Operand(value)),
                            }),
                            end_block,
                        );
                    }
                }

                if matches!(
                    self.tree.blocks[end_block].terminator,
                    mir::Terminator::Unreachable
                ) {
                    self.insert_terminator(end_block, mir::Terminator::Goto(join_bb));
                }
            }

            self.current.block = Some(join_bb);
            return mir::Operand::new(ty, mir::OperandKind::None);
        }

        let mut temp: Option<mir::TempId> = None;
        for (arm_bb, hir_body) in all_arm_blocks {
            self.current.block = Some(arm_bb);
            let arm_end = &mut false;
            let value = self.lower_block(hir_body, arm_bb).pass(arm_end);
            let end_block = self.expect_current_block();

            if !*arm_end {
                if let Some(value) = value.filter(|v| !matches!(v.kind, mir::OperandKind::None)) {
                    let temp_id = match temp {
                        Some(id) => id,
                        None => {
                            let id = self.new_temp(ty);
                            temp = Some(id);
                            id
                        }
                    };

                    let place = self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp_id), ty));

                    self.push_statement_from(
                        mir::Statement::new(mir::StatementKind::Assign {
                            place,
                            value: mir::Rvalue::new(mir::RvalueKind::Operand(value)),
                        }),
                        end_block,
                    );
                }
            }

            if matches!(
                self.tree.blocks[end_block].terminator,
                mir::Terminator::Unreachable
            ) {
                self.insert_terminator(end_block, mir::Terminator::Goto(join_bb));
            }
        }

        self.current.block = Some(join_bb);

        mir::Operand::new(
            ty,
            match temp {
                Some(temp_id) => mir::OperandKind::Temp(temp_id),
                None => mir::OperandKind::None,
            },
        )
    }

    fn lower_array_arm_chain(
        &mut self,
        scrutinee_op: mir::Operand,
        scrutinee_ty: hir::TypeId,
        array_arms: &[(BlockId, hir::BlockId, Vec<MatchPatternHir>)],
        fallthrough: BlockId,
        returnable: bool,
    ) -> BlockId {
        let bool_ty = self.hir_response.typed.types_table.bool_type;

        let thir_type = self.id_to_type(scrutinee_ty);
        let (elem_ty, _array_len) = match &thir_type.kind {
            ThirTypeKind::Array {
                element,
                kind: ArrayKind::StackArray(len),
            } => (*element, *len),
            _ => return fallthrough,
        };

        let mut next_bb = fallthrough;
        for (arm_bb, _hir_body, elements) in array_arms.iter().rev() {
            let chain_entry = self.new_block();
            self.tree.blocks[chain_entry].returnable = returnable;

            let has_wildcard = elements
                .iter()
                .any(|e| matches!(e, MatchPatternHir::Wildcard | MatchPatternHir::Binding(_) | MatchPatternHir::Constructor { .. }));

            if !has_wildcard {
                self.lower_array_arm_wildcard(
                    elements,
                    scrutinee_ty,
                    scrutinee_op.clone(),
                    chain_entry,
                    bool_ty,
                    next_bb,
                    *arm_bb,
                );
            } else {
                self.lower_array_arm(
                    elements,
                    scrutinee_ty,
                    &scrutinee_op,
                    chain_entry,
                    returnable,
                    elem_ty,
                    bool_ty,
                    next_bb,
                    *arm_bb,
                );
            }

            next_bb = chain_entry;
        }

        next_bb
    }

    fn lower_array_arm(
        &mut self,
        elements: &Vec<MatchPatternHir>,
        scrutinee_ty: hir::TypeId,
        scrutinee_op: &mir::Operand,
        chain_entry: BlockId,
        returnable: bool,
        elem_ty: TypeId,
        bool_ty: TypeId,
        next_bb: BlockId,
        arm_bb: BlockId,
    ) {
        let mut current_bb = chain_entry;
        for (i, pattern_elem) in elements.iter().enumerate() {
            match pattern_elem {
                MatchPatternHir::Wildcard => continue,
                MatchPatternHir::Binding(_) => continue,
                MatchPatternHir::Constructor { .. } => continue,
                MatchPatternHir::Literal(lit) => {
                    let lit_value = ComplexLiteral::Basic(lit.clone());

                    let ptr_temp = self.new_temp(scrutinee_ty);
                    let index_op = mir::Operand::new(
                        self.hir_response.typed.types_table.index_type,
                        mir::OperandKind::Comptime(ComplexLiteral::Basic(Literal::Int(i as i128))),
                    );
                    let ptr_place = self.new_place(mir::Place::new(
                        mir::PlaceKind::Temp(ptr_temp),
                        scrutinee_ty,
                    ));
                    self.push_statement_from(
                        mir::Statement::new(mir::StatementKind::Assign {
                            place: ptr_place,
                            value: mir::Rvalue::new(mir::RvalueKind::StackArrayIndex {
                                array: scrutinee_op.clone(),
                                index: index_op,
                            }),
                        }),
                        current_bb,
                    );

                    let ptr_op = mir::Operand::new(scrutinee_ty, mir::OperandKind::Temp(ptr_temp));
                    let elem_temp = self.new_temp(elem_ty);
                    let elem_place =
                        self.new_place(mir::Place::new(mir::PlaceKind::Temp(elem_temp), elem_ty));
                    self.push_statement_from(
                        mir::Statement::new(mir::StatementKind::Assign {
                            place: elem_place,
                            value: mir::Rvalue::new(mir::RvalueKind::Place(mir::Place::new(
                                mir::PlaceKind::Deref(ptr_op),
                                elem_ty,
                            ))),
                        }),
                        current_bb,
                    );

                    let elem_op = mir::Operand::new(elem_ty, mir::OperandKind::Temp(elem_temp));
                    let lit_op = mir::Operand::new(elem_ty, mir::OperandKind::Comptime(lit_value));
                    let eq_temp = self.new_temp(bool_ty);
                    let eq_place =
                        self.new_place(mir::Place::new(mir::PlaceKind::Temp(eq_temp), bool_ty));
                    self.push_statement_from(
                        mir::Statement::new(mir::StatementKind::Assign {
                            place: eq_place,
                            value: mir::Rvalue::new(mir::RvalueKind::Binary {
                                left: elem_op,
                                operator: BinaryOperator::new(
                                    BinaryOperatorKind::Eq,
                                    self.hir_response.hir.info.spans.expressions
                                        [hir::ExpressionId::error()],
                                ),
                                right: lit_op,
                            }),
                        }),
                        current_bb,
                    );

                    let continue_bb = if i + 1 < elements.len() {
                        let bb = self.new_block();
                        self.tree.blocks[bb].returnable = returnable;
                        bb
                    } else {
                        arm_bb
                    };

                    let eq_condition = mir::Operand::new(bool_ty, mir::OperandKind::Temp(eq_temp));
                    self.insert_terminator(
                        current_bb,
                        mir::Terminator::If {
                            condition: eq_condition,
                            then: continue_bb,
                            arm: next_bb,
                        },
                    );

                    current_bb = continue_bb;
                }
                MatchPatternHir::Array(_) => {}
            }
        }
    }

    fn lower_array_arm_wildcard(
        &mut self,
        elements: &Vec<MatchPatternHir>,
        scrutinee_ty: hir::TypeId,
        scrutinee_op: mir::Operand,
        chain_entry: BlockId,
        bool_ty: TypeId,
        next_bb: BlockId,
        arm_bb: BlockId,
    ) {
        let pattern_values = elements
            .iter()
            .map(|e| match e {
                MatchPatternHir::Literal(lit) => ComplexLiteral::Basic(lit.clone()),
                _ => ComplexLiteral::Basic(Literal::Int(0)),
            })
            .collect();

        let pattern_array = mir::Operand::new(
            scrutinee_ty,
            mir::OperandKind::Comptime(ComplexLiteral::Array {
                array_type: scrutinee_ty,
                values: pattern_values,
            }),
        );

        let eq_temp = self.new_temp(bool_ty);
        let eq_place = self.new_place(mir::Place::new(mir::PlaceKind::Temp(eq_temp), bool_ty));
        self.push_statement_from(
            mir::Statement::new(mir::StatementKind::Assign {
                place: eq_place,
                value: mir::Rvalue::new(mir::RvalueKind::Binary {
                    left: scrutinee_op.clone(),
                    operator: BinaryOperator::new(
                        BinaryOperatorKind::Eq,
                        self.hir_response.hir.info.spans.expressions[hir::ExpressionId::error()],
                    ),
                    right: pattern_array,
                }),
            }),
            chain_entry,
        );

        let eq_condition = mir::Operand::new(bool_ty, mir::OperandKind::Temp(eq_temp));
        self.insert_terminator(
            chain_entry,
            mir::Terminator::If {
                condition: eq_condition,
                then: arm_bb,
                arm: next_bb,
            },
        );
    }

    fn lower_string_arm_chain(
        &mut self,
        scrutinee_op: mir::Operand,
        scrutinee_ty: hir::TypeId,
        string_arms: &[(BlockId, hir::BlockId, Literal)],
        fallthrough: BlockId,
        returnable: bool,
    ) -> BlockId {
        let bool_ty = self.hir_response.typed.types_table.bool_type;

        let mut next_bb = fallthrough;
        for (arm_bb, _hir_body, lit) in string_arms.iter().rev() {
            let chain_entry = self.new_block();
            self.tree.blocks[chain_entry].returnable = returnable;

            let pattern_str = mir::Operand::new(
                scrutinee_ty,
                mir::OperandKind::Comptime(ComplexLiteral::Basic(lit.clone())),
            );

            let eq_temp = self.new_temp(bool_ty);
            let eq_place = self.new_place(mir::Place::new(mir::PlaceKind::Temp(eq_temp), bool_ty));
            self.push_statement_from(
                mir::Statement::new(mir::StatementKind::Assign {
                    place: eq_place,
                    value: mir::Rvalue::new(mir::RvalueKind::Binary {
                        left: scrutinee_op.clone(),
                        operator: BinaryOperator::new(
                            BinaryOperatorKind::Eq,
                            self.hir_response.hir.info.spans.expressions
                                [hir::ExpressionId::error()],
                        ),
                        right: pattern_str,
                    }),
                }),
                chain_entry,
            );

            let eq_condition = mir::Operand::new(bool_ty, mir::OperandKind::Temp(eq_temp));
            self.insert_terminator(
                chain_entry,
                mir::Terminator::If {
                    condition: eq_condition,
                    then: *arm_bb,
                    arm: next_bb,
                },
            );

            next_bb = chain_entry;
        }

        next_bb
    }
}
