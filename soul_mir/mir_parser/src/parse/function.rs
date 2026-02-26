use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub fn lower_function(&mut self, function_id: hir::FunctionId) {
        self.current_function = function_id;
        let function = &self.hir.functions[function_id];

        let entry_block = self.new_block();

        let mut locals = vec![];

        for parameter in &function.parameters {
            let local_id = self.new_local(parameter.local, parameter.ty);
            locals.push(local_id);
        }

        let parameters = locals.clone();
        let mir_function = mir::Function {
            locals,
            parameters,
            id: function_id,
            blocks: vec![entry_block],
            name: function.name.clone(),
            return_type: function.return_type,
        };

        self.tree.functions.insert(function_id, mir_function);

        self.lower_block(function.body, entry_block);
    }
}
