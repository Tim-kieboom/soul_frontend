use mir_parser::mir::{Function, FunctionBody, LocalId};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    vec_map::VecMapIndex,
};

use crate::{LlvmBackend, build_error};

impl<'a> LlvmBackend<'a> {
    pub(crate) fn allocate_function_locals(&mut self, function: &Function) {
        let (entry_block, locals) = match &function.body {
            FunctionBody::External(_) => {
                panic!("can not call allocate_function_locals in external function")
            }
            FunctionBody::Internal {
                entry_block,
                locals,
                ..
            } => (entry_block, locals),
        };

        let entry = self.blocks[*entry_block];
        self.builder.position_at_end(entry);

        for (i, local_id) in function.parameters.iter().enumerate() {
            let local = &self.mir.tree.locals[*local_id];
            let name = self.local_name(*local_id);

            let ty = match self.lower_type(local.ty) {
                Ok(Some(ty)) => ty,
                Err(err) => {
                    self.log_error(err);
                    self.context.i8_type().into()
                }
                Ok(None) => self.context.i8_type().into(),
            };

            let ptr = match self.builder.build_alloca(ty, &name) {
                Ok(val) => val,
                Err(err) => {
                    self.log_error(build_error(err));
                    return;
                }
            };
            self.locals.insert(*local_id, ptr);

            let param = self.functions[function.id]
                .get_nth_param(i as u32)
                .expect("should have parameter");
            if let Err(err) = self.builder.build_store(ptr, param) {
                self.log_error(build_error(err));
            }
        }

        for local_id in locals {
            let local = &self.mir.tree.locals[*local_id];
            let name = self.local_name(*local_id);

            let ty = match self.lower_type(local.ty) {
                Ok(Some(ty)) => ty,
                Err(err) => {
                    self.log_error(err);
                    self.context.i8_type().into()
                }
                Ok(None) => self.context.i8_type().into(),
            };

            let ptr = match self.builder.build_alloca(ty, &name) {
                Ok(val) => val,
                Err(err) => {
                    self.log_error(build_error(err));
                    return;
                }
            };
            self.locals.insert(*local_id, ptr);
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
                    continue;
                }
                Err(err) => {
                    self.insert_dummy_global(local);
                    self.log_error(err);
                    continue;
                }
            };

            let ir_global = self.module.add_global(ty, None, &self.local_name(local));
            self.locals.insert(local, ir_global.as_pointer_value());
            if let Some(comptime) = &global.literal {
                let ir_operand = match self.lower_literal(comptime, global.ty) {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        return;
                    }
                };
                ir_global.set_constant(true);
                ir_global.set_initializer(&ir_operand.value);
            } else {
                ir_global.set_constant(false);
                ir_global.set_linkage(inkwell::module::Linkage::External);
                ir_global.set_initializer(&ty.const_zero());
            }
        }
    }

    fn insert_dummy_global(&mut self, local: LocalId) {
        let dummy_type = self.context.i8_type();
        let value = self
            .module
            .add_global(dummy_type, None, "global_error")
            .as_pointer_value();
        self.locals.insert(local, value);
    }

    fn local_name(&self, id: LocalId) -> String {
        format!("_{}", id.index())
    }
}
