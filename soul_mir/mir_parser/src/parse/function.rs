use soul_utils::ids::FunctionId;

use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub fn lower_function(&mut self, function_id: FunctionId) {
        self.current.function = function_id;
        let function = &self.hir.functions[function_id];

        let entry_block = self.new_function_block();
        self.current.block = Some(entry_block);

        let body = match function.body {
            hir::FunctionBody::Internal(_) => {
                mir::FunctionBody::Internal { 
                    entry_block, 
                    locals: vec![],
                    blocks: vec![entry_block], 
                }
            }
            hir::FunctionBody::External(extern_language) => {
                mir::FunctionBody::External(extern_language)
            }
        };

        let mir_function = mir::Function {
            body,
            id: function_id,
            parameters: vec![],
            name: function.name.clone(),
            return_type: self.types.functions[function_id],
        };
        self.tree.functions.insert(function_id, mir_function);

        for parameter in &function.parameters {
            let ty = self.types.locals[parameter.local];
            let local_id = self.new_local(parameter.local, ty);

            let parameters = &mut self.tree.functions[function_id].parameters;
            parameters.push(local_id);
        }

        let body = match function.body {
            hir::FunctionBody::Internal(block_id) => block_id,
            hir::FunctionBody::External(_) => return,
        };

        let _endblock = self.lower_block(body, entry_block);
    }
}
