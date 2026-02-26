use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub fn lower_block(&mut self, block: hir::BlockId, entry_block: mir::BlockId) {
        self.current_block = Some(entry_block);

        let block = &self.hir.blocks[block];

        for stmt in &block.statements {
            self.lower_statement(stmt);
        }

        match block.terminator {
            Some(expr) => {
                let value = self.lower_operand(expr);
                self.insert_terminator(entry_block, mir::Terminator::Return(Some(value)));
            }
            None => {
                self.insert_terminator(entry_block, mir::Terminator::Return(None));
            }
        }
    }
}
