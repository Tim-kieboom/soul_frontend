use mir_parser::mir::{Function, FunctionBody, LocalId};
use soul_utils::{error::{SoulError, SoulErrorKind}, vec_map::VecMapIndex};

use crate::{LlvmBackend, build_error};

impl<'a> LlvmBackend<'a> {
    pub(crate) fn allocate_function_locals(&mut self, function: &Function) {
        let (entry_block, locals) = match &function.body {
            FunctionBody::External(_) => panic!("can not call allocate_function_locals in external function"),
            FunctionBody::Internal { entry_block, locals, .. } => (entry_block, locals),
        };
        
        let entry = self.blocks[*entry_block];
        self.builder.position_at_end(entry);

        for local_id in locals {
            let local = &self.mir.tree.locals[*local_id];
            let name = self.local_name(*local_id);

            match self.lower_type(local.ty) {
                Ok(Some(ty)) => {
                    let ptr = match self.builder.build_alloca(ty, &name) {
                        Ok(val) => val,
                        Err(err) => {
                            self.log_error(build_error(err));
                            return;
                        }
                    };
                    self.locals.insert(*local_id, ptr);
                }
                Err(err) => self.log_error(err),
                Ok(None) => (),
            };
        }
    }

    pub(crate) fn allocate_globals(&mut self) {
        for global in self.mir.tree.globals.values() {
            let local = global.local;
            let ty = match self.lower_type(global.ty) {
                Ok(Some(val)) => val,
                Ok(None) => {
                    self.insert_dummy_global(local);
                    self.log_error(SoulError::new(
                        format!("global type of {:?} was None", local), 
                        SoulErrorKind::LlvmError, 
                        None,
                    ));
                    continue
                }
                Err(err) => {
                    self.insert_dummy_global(local);
                    self.log_error(err);
                    continue
                }
            };

            let ir_global = self.module.add_global(ty, None, &self.local_name(local));
            self.locals.insert(local, ir_global.as_pointer_value());

            if let Some(comptime) = &global.literal {
                let ir_operand = self.lower_literal(comptime);
                ir_global.set_initializer(&ir_operand.value);
            }
        }
    }

    fn insert_dummy_global(&mut self, local: LocalId) {
        let dummy_type = self.context.i8_type();
        let value = self.module.add_global(dummy_type, None, "global_error").as_pointer_value();
        self.locals.insert(local, value);
    }

    fn local_name(&self, id: LocalId) -> String {
        format!("_{}", id.index())
    }
}