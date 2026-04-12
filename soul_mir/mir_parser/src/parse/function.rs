use soul_utils::{Ident, ids::FunctionId, soul_error_internal, span::Span};

use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub(crate) fn build_init_global_function(&mut self) {
        let entry_block = self.new_function_block();
        let init_globals = mir::Function {
            id: self.tree.init_global_function,
            name: Ident::new(
                "_init_globals".to_string(),
                Span::default(self.context.module_store.get_root_id()),
            ),
            body: mir::FunctionBody::Internal {
                entry_block,
                locals: vec![],
                blocks: vec![entry_block],
            },
            generics: vec![],
            parameters: vec![],
            owner_type: self.hir_response.typed.types_table.none_type,
            return_type: self.hir_response.typed.types_table.none_type,
        };

        self.tree.blocks.insert(
            entry_block,
            mir::Block {
                id: entry_block,
                returnable: false,
                terminator: mir::Terminator::Return(None),
                statements: vec![],
            },
        );
        self.tree
            .functions
            .insert(self.tree.init_global_function, init_globals);
    }

    pub(crate) fn lower_function(&mut self, function_id: FunctionId) {
        self.inner_function(function_id, false);
    }

    pub(crate) fn lower_main_function(&mut self) {
        self.inner_function(self.main, true);
    }

    fn inner_function(&mut self, function_id: FunctionId, is_main: bool) {
        self.current.function = function_id;
        let function = &self.hir_response.hir.nodes.functions[function_id];
        let span = self.function_span(function_id);

        let entry_block = self.new_function_block();
        self.current.block = Some(entry_block);

        let body = match function.body {
            hir::FunctionBody::Internal(_) => mir::FunctionBody::Internal {
                entry_block,
                locals: vec![],
                blocks: vec![entry_block],
            },
            hir::FunctionBody::External(extern_language) => {
                if is_main {
                    self.log_error(soul_error_internal!(
                        "main can not be external function",
                        Some(span)
                    ));
                    return;
                }

                mir::FunctionBody::External(extern_language)
            }
        };

        let mir_function = mir::Function {
            body,
            id: function_id,
            parameters: vec![],
            generics: function.generics.clone(),
            owner_type: function.owner_type,
            name: function.name.clone(),
            return_type: self.function_type(function_id),
        };
        self.tree.functions.insert(function_id, mir_function);

        for parameter in &function.parameters {
            let ty = self.local_type(parameter.local);
            let local_id = self.new_parameter(parameter.local, ty);

            let parameters = &mut self.tree.functions[function_id].parameters;
            parameters.push(local_id);
        }

        let body = match function.body {
            hir::FunctionBody::Internal(block_id) => block_id,
            hir::FunctionBody::External(_) => return,
        };

        if is_main {
            let statement = mir::Statement::new(mir::StatementKind::Call {
                id: self.hir_response.hir.init_globals,
                type_args: vec![],
                arguments: vec![],
                return_place: None,
            });
            self.push_statement_from(statement, entry_block);
        }
        let _endblock = self.lower_block(body, entry_block);
    }
}
