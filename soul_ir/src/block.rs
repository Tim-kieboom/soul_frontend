use crate::{LlvmBackend, build_error};
use mir_parser::mir::{BlockId, Terminator};
use soul_utils::{error::SoulResult, vec_map::VecMapIndex};

impl<'a> LlvmBackend<'a> {
    pub(crate) fn create_blocks(&mut self) {
        let function = &self.mir.tree.functions[self.current.function_id];
        let llvm_function = self.functions[function.id];

        for block_id in &function.blocks {
            let bb = self
                .context
                .append_basic_block(llvm_function, &name_block(*block_id));

            self.blocks.insert(*block_id, bb);
        }
    }

    pub(crate) fn lower_terminator(&mut self, block_id: BlockId) -> SoulResult<()> {
        let terminator = &self.mir.tree.blocks[block_id].terminator;

        match terminator {
            Terminator::Goto(target) => {
                _ = self
                    .builder
                    .build_unconditional_branch(self.blocks[*target])
                    .map_err(build_error)?
            }
            Terminator::Exit => {
                _ = self
                    .builder
                    .build_unconditional_branch(self.blocks[self.mir.tree.exit_block])
                    .map_err(build_error)?
            }
            Terminator::Return(value) => {
                let result = if let Some(operand) = value {
                    let return_value = self.lower_operand(operand)?;
                    self.builder.build_return(Some(&return_value.value))
                } else {
                    self.builder.build_return(None)
                };

                result.map_err(build_error)?;
            }
            Terminator::If {
                condition,
                then,
                arm,
            } => {
                let condition = self.lower_operand(condition)?.value.into_int_value();
                self.builder
                    .build_conditional_branch(condition, self.blocks[*then], self.blocks[*arm])
                    .map_err(build_error)?;
            }
            Terminator::Call {
                id,
                arguments,
                return_place: _,
                next,
            } => {
                let mut ir_arguments = Vec::with_capacity(arguments.len());
                for arg in arguments {
                    let meta_data_value = self.lower_operand(arg)?.value.into();
                    ir_arguments.push(meta_data_value);
                }

                self.builder
                    .build_call(self.functions[*id], ir_arguments.as_slice(), "call_result")
                    .map_err(build_error)?;

                self.builder
                    .build_unconditional_branch(self.blocks[*next])
                    .map_err(build_error)?;
            }
            Terminator::Unreachable => panic!("should not have unreachable"),
        };

        Ok(())
    }
}

fn name_block(block_id: BlockId) -> String {
    format!("bb_{}", block_id.index())
}
