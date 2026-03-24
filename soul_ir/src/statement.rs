use inkwell::values::BasicValueEnum;
use mir_parser::mir::{BlockId, OperandKind, Place, PlaceId, Rvalue, RvalueKind, StatementKind};
use soul_utils::{error::{SoulError, SoulErrorKind, SoulResult}, soul_error_internal};

use crate::{GenericSubstitute, LlvmBackend, build_error};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn lower_block(&mut self, block_id: BlockId, generics: &GenericSubstitute) {
        let block = &self.mir.tree.blocks[block_id];
        for statement_id in &block.statements {
            let statement = &self.mir.tree.statements[*statement_id];
            match &statement.kind {
                StatementKind::Assign { place, value } => {
                    if let Err(err) = self.lower_assign(*place, value, generics) {
                        self.log_error(err);
                    }
                }
                StatementKind::Eval(operand) => {
                    if let Err(err) = self.lower_operand(operand, generics) {
                        self.log_error(err);
                    }
                }
                StatementKind::Call {
                    id,
                    arguments,
                    return_place,
                    type_args: call_type_args,
                } => {
                    let prev = self.current;
                    if let Err(err) =
                        self.lower_call(*id, arguments, *return_place, call_type_args, generics)
                    {
                        self.log_error(err);
                    }
                    self.current = prev;
                }
                StatementKind::StorageDead(_) => (),
                StatementKind::StorageStart(_) => (),
            }
        }
    }

    fn lower_assign(
        &mut self,
        place_id: PlaceId,
        value: &Rvalue,
        generics: &GenericSubstitute,
    ) -> SoulResult<()> {
        if rvalue_is_none(value) {
            return Ok(());
        }

        let ir_value = self.lower_rvalue(value, generics)?;
        match &self.mir.tree.places[place_id] {
            Place::Temp(temp_id) => {
                self.push_temp(*temp_id, ir_value);
            }
            Place::Local(local_id) => {
                let local = self.get_local(*local_id);
                let ptr = match local {
                    crate::Local::Runtime(val) => val,
                    crate::Local::Comptime(_) => {
                        return Err(soul_error_internal!(format!("{:?} is comptime so should not be assignable", local_id), None)); 
                    }
                };

                self.builder
                    .build_store(ptr, ir_value.value)
                    .map_err(build_error)?;
            }
            Place::Deref(operand) => {
                let ptr_value = self.lower_operand(operand, generics)?.value;

                let ptr = match ptr_value {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => {
                        return Err(SoulError::new(
                            "deref operand must be a pointer",
                            SoulErrorKind::LlvmError,
                            None,
                        ));
                    }
                };

                self.builder
                    .build_store(ptr, ir_value.value)
                    .map_err(build_error)?;
            }
        }

        Ok(())
    }
}

fn rvalue_is_none(rvalue: &Rvalue) -> bool {
    match &rvalue.kind {
        RvalueKind::Use(operand) => matches!(operand.kind, OperandKind::None),
        _ => false,
    }
}
