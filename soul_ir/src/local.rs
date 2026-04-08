use hir::{ComplexLiteral, TypeId};
use inkwell::{types::BasicTypeEnum, values::PointerValue};
use mir_parser::mir::{self, Function, FunctionBody, LocalId};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    vec_map::VecMapIndex,
};
use crate::{GenericSubstitute, LlvmBackend};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn allocate_function_locals(
        &mut self,
        function: &Function,
        type_args: &Vec<TypeId>,
        generics: &GenericSubstitute,
    ) {
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

        let entry = self.get_block(*entry_block);
        self.builder.position_at_end(entry);

        self.alloc_parameter(function, type_args, generics);
        for local_id in locals {
        
            if let Err(err) = self.alloc_local(*local_id, generics) {
                self.log_error(err);
            }
        }
    }

    pub(crate) fn allocate_globals(&mut self, generics: &GenericSubstitute) {
        let empty_generics = GenericSubstitute::new(&[], &[]);
        for global in self.mir.tree.globals.values() {
            let local = global.local;
            let ty = match self.lower_type(global.ty, &empty_generics) {
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
            self.push_global(local, ir_global.as_pointer_value());
            if let Some(comptime) = &global.literal {
                let ir_operand = match self.lower_literal(comptime, global.ty, generics) {
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

    fn alloc_parameter(
        &mut self,
        function: &Function,
        type_args: &Vec<TypeId>,
        generics: &GenericSubstitute,
    ) {
        for (i, local_id) in function.parameters.iter().enumerate() {
            let local = &self.mir.tree.locals[*local_id];
            let name = self.local_name(*local_id);

            let ty = match self.lower_type(local.ty(), generics) {
                Ok(Some(ty)) => ty,
                Err(err) => {
                    self.log_error(err);
                    self.context.i8_type().into()
                }
                Ok(None) => self.context.i8_type().into(),
            };

            if let mir::Local::Comptime { value, .. } = local {
                if let Err(err) =
                    self.build_comptime_local(ty, local.ty(), *local_id, value, generics)
                {
                    self.log_error(err);
                }

                continue;
            }

            let ptr = match self.build_runtime_local(ty, *local_id, &name) {
                Ok(val) => val,
                Err(err) => {
                    self.log_error(err);
                    continue;
                }
            };
            let function = self.get_or_create_function(function.id, type_args);
            let param = function
                .get_nth_param(i as u32)
                .expect("should have parameter");

            if let Err(err) = self.builder.store_parameter(ptr, param) {
                self.log_error(err);
            }
        }
    }

    fn alloc_local(&mut self, local_id: LocalId, generics: &GenericSubstitute) -> SoulResult<()> {
        let local = &self.mir.tree.locals[local_id];
        let name = self.local_name(local_id);

        let type_id = local.ty();

        let ty = match self.lower_type(type_id, generics) {
            Ok(Some(ty)) => ty,
            Err(err) => {
                self.log_error(err);
                self.context.i8_type().into()
            }
            Ok(None) => self.context.i8_type().into(),
        };

        if let mir::Local::Comptime { value, .. } = local {
            self.build_comptime_local(ty, type_id, local_id, value, generics)?;
            return Ok(());
        }

        _ = self.build_runtime_local(ty, local_id, &name)?;
        Ok(())
    }

    fn build_runtime_local(
        &mut self,
        ty: BasicTypeEnum<'a>,
        local_id: LocalId,
        name: &str,
    ) -> SoulResult<PointerValue<'a>> {
        let ptr = self.builder.build_alloca(ty, name)?;
        self.push_local(local_id, crate::Local::Runtime(ptr));
        Ok(ptr)
    }

    fn build_comptime_local(
        &mut self,
        ty: BasicTypeEnum<'a>,
        hir_type: TypeId,
        id: LocalId,
        literal: &ComplexLiteral,
        generics: &GenericSubstitute,
    ) -> SoulResult<()> {
        let const_operand = match self.lower_literal(literal, hir_type, generics) {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                self.new_loaded_operand(ty.const_zero(), hir_type, generics)?
            }
        };
        let local = crate::Local::Comptime(const_operand);
        self.push_local(id, local);
        Ok(())
    }

    fn insert_dummy_global(&mut self, local: LocalId) {
        let dummy_type = self.context.i8_type();
        let ptr = self
            .module
            .add_global(dummy_type, None, "global_error")
            .as_pointer_value();

        self.push_local(local, crate::Local::Runtime(ptr));
    }

    fn local_name(&self, id: LocalId) -> String {
        format!("_{}", id.index())
    }
}
