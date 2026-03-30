use crate::{GenericSubstitute, LlvmBackend};
use hir::TypeId;
use inkwell::values::FunctionValue;
use mir_parser::mir::{BlockId, FunctionBody, Operand, PlaceId, PlaceKind, Terminator};
use soul_utils::{error::SoulResult, ids::FunctionId, vec_map::VecMapIndex};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn create_block(
        &mut self,
        function_id: FunctionId,
        llvm_function: FunctionValue<'a>,
    ) {
        let function = &self.mir.tree.functions[function_id];

        let blocks = match &function.body {
            FunctionBody::External(_) => return,
            FunctionBody::Internal { blocks, .. } => blocks,
        };

        for block_id in blocks {
            let bb = self
                .context
                .append_basic_block(llvm_function, &name_block(*block_id));

            self.push_block(*block_id, bb);
        }
    }

    pub(crate) fn lower_terminator(
        &mut self,
        block_id: BlockId,
        generics: &GenericSubstitute,
    ) -> SoulResult<()> {
        let terminator = &self.mir.tree.blocks[block_id].terminator;

        match terminator {
            Terminator::Goto(target) => {
                _ = self
                    .builder
                    .build_unconditional_branch(self.get_block(*target))?
            }
            Terminator::Exit => {
                let i32 = self.context.i32_type();
                let exit_code = i32.const_int(0, false);
                let exit = self
                    .exit_function
                    .expect("should have initialize exit function");
                self.builder.build_call(exit, &[exit_code.into()])?;

                self.builder.build_unreachable()?;
            }
            Terminator::Return(value) => {
                if let Some(operand) = value {
                    let return_value = self.lower_operand(operand, generics)?;
                    self.builder.build_return(Some(&return_value.value))?
                } else {
                    self.builder.build_return(None)?
                };
            }
            Terminator::If {
                condition,
                then,
                arm,
            } => {
                let condition = self
                    .lower_operand(condition, generics)?
                    .value
                    .into_int_value();
                self.builder
                    .build_conditional_branch(
                        condition,
                        self.get_block(*then),
                        self.get_block(*arm),
                    )?;
            }
            Terminator::Unreachable => panic!("should not have unreachable"),
        };

        Ok(())
    }

    pub(crate) fn lower_call(
        &mut self,
        id: FunctionId,
        arguments: &Vec<Operand>,
        return_place: Option<PlaceId>,
        type_args: &Vec<TypeId>,
        generics: &GenericSubstitute,
    ) -> SoulResult<()> {
        let mut ir_arguments = Vec::with_capacity(arguments.len());
        for arg in arguments {
            let meta_data_value = self.lower_operand(arg, generics)?.value.into();
            ir_arguments.push(meta_data_value);
        }
        
        let prev = self.current;
        
        let function = self.get_or_create_function(id, type_args);
        let call = self
            .builder
            .build_call(function, ir_arguments.as_slice())?;
    
        self.current = prev;
        let place = match return_place {
            Some(val) => &self.mir.tree.places[val],
            None => return Ok(()),
        };

        let return_value = call.try_as_basic_value().unwrap_basic();
        match &place.kind {
            PlaceKind::Temp(temp_id) => {
                let value = self.new_loaded_operand(return_value, place.ty, generics)?;
                self.push_temp(*temp_id, value);
            }
            PlaceKind::Field{..} => panic!("call return value should be Place::Temp not Place::Field"),
            PlaceKind::Deref(_) => panic!("call return value should be Place::Temp not Place::Deref"),
            PlaceKind::Local(_) => panic!("call return value should be Place::Temp not Place::Local"),
        }

        Ok(())
    }
}

fn name_block(block_id: BlockId) -> String {
    format!("bb_{}", block_id.index())
}
