use ast_model::NodeId;
use soul_utils::fault::Fault;

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    
    pub(crate) fn log_fault(&mut self, fault: Fault) {
        self.context.faults.push(fault);
    }

    pub(crate) fn alloc_node(&mut self) -> NodeId {
        self.node_generator.alloc()
    }
}