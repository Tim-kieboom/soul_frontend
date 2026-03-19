use inkwell::values::BasicValueEnum;
use mir_parser::mir::{BlockId, OperandKind, Place, PlaceId, Rvalue, RvalueKind, StatementKind};
use soul_utils::error::{SoulError, SoulErrorKind, SoulResult};

use crate::{LlvmBackend, build_error};

impl<'a> LlvmBackend<'a> {
    pub(crate) fn lower_statement(&mut self, block_id: BlockId) {
        let block = &self.mir.tree.blocks[block_id];
        for statement_id in &block.statements {
            let statement = &self.mir.tree.statements[*statement_id];
            match &statement.kind {
                StatementKind::Assign { place, value } => {
                    if let Err(err) = self.lower_assign(*place, value) {
                        self.log_error(err);
                    }
                }
                StatementKind::Eval(operand) => {
                    if let Err(err) = self.lower_operand(operand) {
                        self.log_error(err);
                    }
                }
                StatementKind::Call {
                    id,
                    arguments,
                    return_place,
                } => {
                    if let Err(err) = self.lower_call(*id, arguments, *return_place) {
                        self.log_error(err);
                    }
                }
                StatementKind::StorageDead(_) => (),
                StatementKind::StorageStart(_) => (),
            }
        }
    }

    fn lower_assign(&mut self, place_id: PlaceId, value: &Rvalue) -> SoulResult<()> {
        if rvalue_is_none(value) {
            return Ok(());
        }

        let ir_value = self.lower_rvalue(value)?;
        match &self.mir.tree.places[place_id] {
            Place::Temp(temp_id) => {
                self.temps.insert(*temp_id, ir_value);
            }
            Place::Local(local_id) => {
                let ptr = self.locals[*local_id];
                self.builder
                    .build_store(ptr, ir_value.value)
                    .map_err(build_error)?;
            }
            Place::Deref(operand) => {
                let ptr_value = self.lower_operand(operand)?.value;

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
