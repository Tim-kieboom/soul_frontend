//! Terminator codegen: the block-ending constructs (`Call`, `Assert`,
//! `SwitchInt`, `Return`, ...) that `function`'s per-block driver dispatches
//! to — split out since each is its own small, self-contained concern.

use inkwell::{
    IntPredicate,
    values::{BasicValueEnum, FunctionValue},
};
use mir_model::{BlockId, ConstValue, LocalId, Operand, Place, Terminator};
use soul_utils::FunctionId;

use crate::{
    err,
    fault::{CodegenErrorKind, CodegenResult},
    function::FunctionCodegen,
    llvm_err,
    types::expect_int,
};

impl<'ctx, 'a> FunctionCodegen<'ctx, 'a> {
    pub(crate) fn codegen_terminator(&mut self, terminator: &Terminator) -> CodegenResult<()> {
        match terminator {
            Terminator::Goto(target) => {
                self.builder
                    .build_unconditional_branch(self.blocks[*target])
                    .map_err(llvm_err)?;
            }
            Terminator::SwitchInt {
                discriminant,
                targets,
                otherwise,
            } => {
                self.codegen_switchint(discriminant, targets, otherwise)?;
            }
            Terminator::Call {
                id,
                arguments,
                destination,
                target,
            } => {
                self.codegen_call(*id, arguments, destination, target)?;
            }
            Terminator::Assert {
                cond,
                expected,
                target,
                ..
            } => {
                self.codegen_assert(cond, *expected, target)?;
            }
            Terminator::Return => self.codegen_return()?,
            Terminator::Unreachable => {
                self.builder.build_unreachable().map_err(llvm_err)?;
            }
            Terminator::Drop { .. } => {
                return Err(err(CodegenErrorKind::DropUnsupported));
            }
        }
        Ok(())
    }

    fn codegen_return(&mut self) -> CodegenResult<()> {
        if let Some(local) = self.function.return_local {
            let ty = self.local_type(local)?;
            let value = self
                .builder
                .build_load(ty, self.locals[local], "ret")
                .map_err(llvm_err)?;

            if self.is_entry_point {
                self.build_entry_point_return(value)?;
            } else {
                self.builder.build_return(Some(&value)).map_err(llvm_err)?;
            }

            return Ok(());
        }

        if self.is_entry_point {
            let zero = self.ctx.context.i32_type().const_zero();
            self.builder.build_return(Some(&zero)).map_err(llvm_err)?;
            return Ok(());
        }

        self.builder.build_return(None).map_err(llvm_err)?;
        Ok(())
    }

    fn codegen_assert(
        &mut self,
        cond: &Operand,
        expected: bool,
        target: &BlockId,
    ) -> CodegenResult<()> {
        let bool_ty = self.ctx.context.bool_type().into();
        let cond = expect_int(self.codegen_operand(cond, bool_ty)?)?;
        let expect_true = self
            .ctx
            .context
            .bool_type()
            .const_int(u64::from(expected), false);

        let ok = self
            .builder
            .build_int_compare(IntPredicate::EQ, cond, expect_true, "assert_ok")
            .map_err(llvm_err)?;

        let panic_block = self
            .ctx
            .context
            .insert_basic_block_after(self.builder.get_insert_block().unwrap(), "panic");

        self.builder
            .build_conditional_branch(ok, self.blocks[*target], panic_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(panic_block);
        let abort_fn = self.abort_function();
        self.builder
            .build_call(abort_fn, &[], "abort_call")
            .map_err(llvm_err)?;

        self.builder.build_unreachable().map_err(llvm_err)?;
        Ok(())
    }

    fn codegen_call(
        &mut self,
        id: FunctionId,
        arguments: &[Operand],
        destination: &Option<Place>,
        target: &Option<BlockId>,
    ) -> CodegenResult<()> {
        let callee_module = self
            .ctx
            .declares
            .get_function(id)
            .map(|(_, module)| *module);
        let param_types = if let Some(callee_mir) = self.functions.get(id) {
            let callee_param_locals: Vec<LocalId> = callee_mir
                .locals
                .entries()
                .take(callee_mir.arg_count)
                .map(|(id, _)| id)
                .collect();

            callee_param_locals
                .iter()
                .map(|&local| {
                    let decl = &callee_mir.locals[local];
                    self.ctx.llvm_type(callee_module, &decl.ty, Some(decl.span))
                })
                .collect::<CodegenResult<Vec<_>>>()?
        } else if let Some(extern_fn) = self.externs.get(id) {
            extern_fn
                .params
                .iter()
                .map(|ty| self.ctx.llvm_type(callee_module, ty, None))
                .collect::<CodegenResult<Vec<_>>>()?
        } else {
            return Err(err(CodegenErrorKind::CallHasNoMirBody { id }));
        };

        let callee_value = *self
            .function_values
            .get(id)
            .ok_or_else(|| err(CodegenErrorKind::CallNeverDeclared { id }))?;

        if arguments.len() > param_types.len() {
            return Err(err(CodegenErrorKind::CallArgumentCountMismatch { id }));
        }

        let mut args = Vec::with_capacity(arguments.len());
        for (arg, param_ty) in arguments.iter().zip(param_types.iter()) {
            args.push(self.codegen_operand(arg, *param_ty)?.into());
        }

        let call = self
            .builder
            .build_call(callee_value, &args, "call")
            .map_err(llvm_err)?;

        if let Some(place) = destination {
            if !place.projection.is_empty() {
                return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
            }

            let result = call
                .try_as_basic_value()
                .left()
                .ok_or_else(|| err(CodegenErrorKind::CallResultIsNone))?;

            self.builder
                .build_store(self.locals[place.local], result)
                .map_err(llvm_err)?;
        }

        match target {
            Some(target) => {
                self.builder
                    .build_unconditional_branch(self.blocks[*target])
                    .map_err(llvm_err)?;
            }
            None => {
                self.builder.build_unreachable().map_err(llvm_err)?;
            }
        }

        Ok(())
    }

    fn codegen_switchint(
        &mut self,
        discriminant: &Operand,
        targets: &[(ConstValue, BlockId)],
        otherwise: &BlockId,
    ) -> CodegenResult<()> {
        let bool_ty = self.ctx.context.bool_type().into();
        let cond = expect_int(self.codegen_operand(discriminant, bool_ty)?)?;
        let [(value, target)] = targets else {
            return Err(err(CodegenErrorKind::SwitchIntTargetCountUnsupported));
        };
        let ConstValue::Bool(expect_true) = value else {
            return Err(err(CodegenErrorKind::SwitchIntTargetValueUnsupported));
        };
        let (then_block, else_block) = if *expect_true {
            (self.blocks[*target], self.blocks[*otherwise])
        } else {
            (self.blocks[*otherwise], self.blocks[*target])
        };
        self.builder
            .build_conditional_branch(cond, then_block, else_block)
            .map_err(llvm_err)?;

        Ok(())
    }

    /// `main`'s LLVM-level return is forced to `i32` (see
    /// `declare_all_functions`) since that's what the C ABI/process exit
    /// code convention needs, regardless of Soul's declared return type.
    /// Narrower int types (`bool`, `u8`, ...) are zero-extended; `i32`
    /// itself passes through unchanged (LLVM's `zext` requires the
    /// destination to be strictly wider than the source — a same-width
    /// "extension" is invalid IR, not a no-op); anything wider, or a
    /// pointer, is a real, reported error rather than a silent truncation
    /// or a panic.
    fn build_entry_point_return(&self, value: BasicValueEnum<'ctx>) -> CodegenResult<()> {
        let value = expect_int(value)?;
        let i32_ty = self.ctx.context.i32_type();
        let bits = value.get_type().get_bit_width();
        let value = match bits.cmp(&32) {
            std::cmp::Ordering::Less => self
                .builder
                .build_int_z_extend(value, i32_ty, "exit_code")
                .map_err(llvm_err)?,
            std::cmp::Ordering::Equal => value,
            std::cmp::Ordering::Greater => {
                return Err(err(CodegenErrorKind::EntryPointReturnTypeTooWide));
            }
        };
        self.builder.build_return(Some(&value)).map_err(llvm_err)?;
        Ok(())
    }

    /// The C runtime's `abort()`, declared lazily (once per module) the
    /// first time an `assert`/`panic` is actually codegen'd. Also used by
    /// `function::build_bounds_check` for out-of-bounds slice indexing.
    pub(crate) fn abort_function(&self) -> FunctionValue<'ctx> {
        if let Some(existing) = self.ctx.module.get_function("abort") {
            return existing;
        }
        let fn_type = self.ctx.context.void_type().fn_type(&[], false);
        self.ctx.module.add_function("abort", fn_type, None)
    }
}
