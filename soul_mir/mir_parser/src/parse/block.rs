use soul_utils::soul_error_internal;

use crate::{EndBlock, MirContext, mir};

impl<'a> MirContext<'a> {
    pub(crate) fn lower_block(&mut self, block_id: hir::BlockId, entry_block: mir::BlockId) -> EndBlock<()> {

        let (live_i, parent_scope) = self.start_scope(entry_block);
        let block = &self.hir.blocks[block_id];

        let mut terminator = None;
        let mut block_operand = None;
        let is_end = &mut false;

        for statement in &block.statements {
            
            let response = self.lower_statement(statement).pass(is_end);
            match response.terminator {
                Some(val) => terminator = Some(val),
                None => (),
            }          
            match response.block_operand {
                Some(val) => block_operand = Some(val),
                None => (),
            }

            if *is_end {
                break
            }
        }
        
        
        if !self.tree.blocks[entry_block].returnable {
            if let Some(terminator) = terminator {
                self.insert_terminator(entry_block, terminator);
            }

            self.end_scope(live_i, entry_block, parent_scope);
            return EndBlock::new((), is_end);
        }

        match (terminator, block.terminator) {
            (Some(terminator), _) => {
                self.insert_terminator(entry_block, terminator)
            }
            (_, Some(expression)) => {
                let value = match block_operand {
                    Some(val) => val,
                    None => {
                        self.log_error(soul_error_internal!("block_operand should be Some(_)", None));
                        self.lower_operand(expression).pass(is_end)
                    }
                };

                self.insert_terminator(entry_block, mir::Terminator::Return(Some(value)))
            }
            _ => {
                self.insert_terminator(entry_block, mir::Terminator::Return(None))
            }
        }

        self.end_scope(live_i, entry_block, parent_scope);
        EndBlock::new((), is_end)
    }

    fn start_scope(&mut self, entry_block: mir::BlockId) -> (usize, Vec<mir::LocalId>) {
        self.current.block = Some(entry_block);
        let parent_scope = self.push_scope();
        
        let i = self.tree.blocks[entry_block].statements.len();
        self.push_statement(mir::Statement::new(mir::StatementKind::StorageStart(vec![])));
        (i, parent_scope)
    }

    fn end_scope(&mut self, i_live: usize, entry_block: mir::BlockId, parent_scope: Vec<mir::LocalId>) {
        let this_scope = self.pop_scope(parent_scope);
        for local in &this_scope {
            self.push_statement(mir::Statement::new(mir::StatementKind::StorageDead(*local)));
        }

        let statement_id = self.tree.blocks[entry_block].statements[i_live];
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
