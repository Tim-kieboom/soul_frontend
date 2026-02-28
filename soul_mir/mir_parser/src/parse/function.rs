use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub fn lower_function(&mut self, function_id: hir::FunctionId) {
        self.current.function = function_id;
        let function = &self.hir.functions[function_id];

        let entry_block = self.new_function_block();
        self.current.block = Some(entry_block);

        let mir_function = mir::Function {
            locals: vec![],
            id: function_id,
            parameters: vec![],
            blocks: vec![entry_block],
            name: function.name.clone(),
            return_type: function.return_type,
        };
        self.tree.functions.insert(function_id, mir_function);

        for parameter in &function.parameters {
            let ty = self.types.locals[parameter.local];
            let local_id = self.new_local(parameter.local, ty);
            
            let parameters = &mut self.tree.functions[function_id].parameters;
            parameters.push(local_id);
        }

        let _endblock = self.lower_block(function.body, entry_block);
    }
}
