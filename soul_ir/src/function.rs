use mir_parser::mir::{Function, LocalId, Operand, OperandKind};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    ids::FunctionId,
    soul_error_internal,
    vec_map::VecMapIndex,
};

use crate::{IrOperand, LlvmBackend, build_error};

impl<'a> LlvmBackend<'a> {
    pub(crate) fn declare_function(&mut self, function_id: FunctionId) {
        let function = &self.mir.tree.functions[function_id];

        let function_type = self.context.i32_type().fn_type(&[], false);
        let llvm_function = self
            .module
            .add_function(function.name.as_str(), function_type, None);

        self.functions.insert(function_id, llvm_function);
    }

    pub(crate) fn lower_function(&mut self, function_id: FunctionId) {
        self.current.function_id = function_id;
        let function = &self.mir.tree.functions[function_id];

        self.create_blocks();
        self.allocate_function_locals(function);

        for block_id in &function.blocks {
            let llvm_block = self.blocks[*block_id];
            self.builder.position_at_end(llvm_block);
            self.lower_statement(*block_id);

            if let Err(err) = self.lower_terminator(*block_id) {
                self.log_error(err);
            }
        }
    }

    pub(crate) fn lower_operand(&self, condition: &Operand) -> SoulResult<IrOperand<'a>> {
        Ok(match &condition.kind {
            OperandKind::Temp(temp_id) => {
                self.temps.get(*temp_id)
                    .copied()
                    .ok_or(soul_error_internal!(format!("{:?} not found", *temp_id), None))?
            }
            OperandKind::Local(local_id) => {
                let local = &self.mir.tree.locals[*local_id];
                let ty = match self.lower_type(local.ty)? {
                    Some(val) => val,
                    None => self.context.i8_type().into(),
                };
                let ptr = self.locals[*local_id];
                let is_signed_interger = self
                    .types
                    .types
                    .get_type(local.ty)
                    .ok_or(soul_error_internal!(
                        format!("{:?} not found", local.ty),
                        None
                    ))?
                    .is_signed_interger();

                return self
                    .builder
                    .build_load(ty, ptr, "load")
                    .map(|value| IrOperand {
                        value,
                        is_signed_interger,
                    })
                    .map_err(|err| build_error(err));
            }
            OperandKind::Comptime(literal) => match literal {
                ast::Literal::Int(value) => {
                    let negative = *value < 0;
                    let value = self
                        .context
                        .i64_type()
                        .const_int(value.abs() as u64, negative)
                        .into();

                    IrOperand {
                        value,
                        is_signed_interger: true,
                    }
                }
                ast::Literal::Uint(value) => {
                    let value = self
                        .context
                        .i64_type()
                        .const_int(*value as u64, false)
                        .into();

                    IrOperand {
                        value,
                        is_signed_interger: false,
                    }
                }
                ast::Literal::Float(value) => {
                    let value = self.context.f64_type().const_float(*value).into();

                    IrOperand {
                        value,
                        is_signed_interger: false,
                    }
                }
                ast::Literal::Bool(value) => {
                    let value = self
                        .context
                        .bool_type()
                        .const_int(*value as u64, false)
                        .into();

                    IrOperand {
                        value,
                        is_signed_interger: false,
                    }
                }
                ast::Literal::Char(value) => {
                    let value = self
                        .context
                        .i8_type()
                        .const_int(*value as u64, false)
                        .into();

                    IrOperand {
                        value,
                        is_signed_interger: false,
                    }
                }
                ast::Literal::Str(_) => {
                    todo!("string literal not yet impl")
                }
            },
            OperandKind::Ref { .. } => {
                todo!("ref not yet impl")
            }
            OperandKind::None => {
                return Err(SoulError::new(
                    "operand should be Some(_)",
                    SoulErrorKind::LlvmError,
                    None,
                ));
            }
        })
    }

    pub(crate) fn allocate_function_locals(&mut self, function: &Function) {
        let entry = self.blocks[function.entry_block];
        self.builder.position_at_end(entry);

        for local_id in &function.locals {
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

    fn local_name(&mut self, id: LocalId) -> String {
        format!("_{}", id.index())
    }
}
