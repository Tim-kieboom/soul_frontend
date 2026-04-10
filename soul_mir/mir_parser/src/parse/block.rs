use soul_utils::soul_error_internal;

use crate::{
    EndBlock, MirContext,
    mir::{self, OperandKind},
};

struct LiveIndex {
    block: mir::BlockId,
    index: usize,
}

impl<'a> MirContext<'a> {
    pub(crate) fn lower_block(
        &mut self,
        hir_block: hir::BlockId,
        mir_block: mir::BlockId,
    ) -> EndBlock<Option<mir::Operand>> {
        let mut this_block = mir_block;

        self.current.block = Some(this_block);
        let (live_i, parent_scope) = self.start_scope(this_block);
        let block = &self.hir_response.hir.nodes.blocks[hir_block];

        let mut terminator = None;
        let mut block_operand = None;
        let mut block_operand_id = None;
        let is_end = &mut false;

        for statement in &block.statements {
            let response = self.lower_statement(statement).pass(is_end);
            match response.terminator {
                Some(val) => terminator = Some(val),
                None => (),
            }
            match response.expression_operand {
                Some(val) => block_operand = Some(val),
                None => (),
            }
            match response.expression_value_id {
                Some(val) => block_operand_id = Some(val),
                None => (),
            }

            if *is_end {
                break;
            }
        }

        this_block = self.expect_current_block();
        if !self.tree.blocks[this_block].returnable {
            let value = match terminator {
                Some(mir::Terminator::Return(operand)) => {
                    let return_value = operand.filter(|value| !matches!(value.kind, OperandKind::None));
                    self.insert_terminator(this_block, mir::Terminator::Return(return_value.clone()));
                    return_value
                }
                Some(mir::Terminator::Goto(target)) => {
                    self.insert_terminator(this_block, mir::Terminator::Goto(target));
                    block_operand.filter(|value| !matches!(value.kind, OperandKind::None))
                }
                None => block_operand.filter(|value| !matches!(value.kind, OperandKind::None)),
                _ => {
                    self.log_error(soul_error_internal!(
                        "should not have this terminator kind in block",
                        None
                    ));
                    None
                }
            };

            self.end_scope(live_i, parent_scope);
            return EndBlock::new(value, is_end);
        }

        match (terminator, block.terminator) {
            (Some(terminator), _) => self.insert_terminator(this_block, terminator),
            (_, Some(expression)) => {
                let expression_id = expression.get_expression_id();
                let value = if block_operand_id == Some(expression_id) {
                    match block_operand {
                        Some(val) => val,
                        None => self.lower_operand(expression_id).pass(is_end),
                    }
                } else {
                    self.lower_operand(expression_id).pass(is_end)
                };

                let return_value = if matches!(value.kind, OperandKind::None) {
                    None
                } else {
                    Some(value)
                };

                self.insert_terminator(this_block, mir::Terminator::Return(return_value))
            }
            _ => self.insert_terminator(this_block, mir::Terminator::Return(None)),
        }

        self.end_scope(live_i, parent_scope);
        EndBlock::new(None, is_end)
    }

    fn start_scope(&mut self, entry_block: mir::BlockId) -> (LiveIndex, Vec<mir::LocalId>) {
        let parent_scope = self.push_scope();

        let i = self.tree.blocks[entry_block].statements.len();
        self.push_statement(mir::Statement::new(mir::StatementKind::StorageStart(
            vec![],
        )));
        (
            LiveIndex {
                block: entry_block,
                index: i,
            },
            parent_scope,
        )
    }

    fn end_scope(&mut self, i_live: LiveIndex, parent_scope: Vec<mir::LocalId>) {
        let this_scope = self.pop_scope(parent_scope);
        for local in &this_scope {
            self.push_statement(mir::Statement::new(mir::StatementKind::StorageDead(*local)));
        }

        let statement_id = self.tree.blocks[i_live.block].statements[i_live.index];
        let statement = &mut self.tree.statements[statement_id];
        if let mir::StatementKind::StorageStart(scope) = &mut statement.kind {
            *scope = this_scope;
        }
    }

    /// makes new scope in `current.scope` and return parent scope
    fn push_scope(&mut self) -> Vec<mir::LocalId> {
        use std::mem::swap;

        let mut parent_scope = vec![];
        swap(&mut parent_scope, &mut self.current.scope);
        parent_scope
    }

    /// inserts parant_scope current scope and returns child scope
    fn pop_scope(&mut self, mut parent_scope: Vec<mir::LocalId>) -> Vec<mir::LocalId> {
        use std::mem::swap;

        swap(&mut parent_scope, &mut self.current.scope);
        parent_scope
    }
}
