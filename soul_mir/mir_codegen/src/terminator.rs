//! Terminator codegen: the block-ending constructs (`Call`, `Assert`,
//! `SwitchInt`, `Return`, ...) that `function`'s per-block driver dispatches
//! to — split out since each is its own small, self-contained concern.

use inkwell::{
    AddressSpace, IntPredicate,
    module::Linkage,
    values::{BasicValueEnum, FunctionValue, PointerValue},
};
use mir_model::{BlockId, ConstValue, LocalId, Operand, Place, Terminator};
use soul_utils::{FunctionId, span::Span};

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
                msg,
                target,
                span,
            } => {
                self.codegen_assert(cond, *expected, msg, target, *span)?;
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

    /// `msg` is the panic message `assert(cond)`/`panic(msg)` lowering
    /// already attaches to this terminator (a `cstr`/`str` operand) — codegen
    /// for both used to discard it and call bare `abort()`; it now flows
    /// through to `panic_function` so a failing assert/panic actually prints
    /// its message before aborting. `span` is `Assert`'s own source location,
    /// turned into a `"file:line:col"` string (`location_string`) and passed
    /// alongside `msg`.
    fn codegen_assert(
        &mut self,
        cond: &Operand,
        expected: bool,
        msg: &Operand,
        target: &BlockId,
        span: Span,
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

        let ptr_ty = self.ctx.context.ptr_type(AddressSpace::default()).into();
        let msg_ptr = self.codegen_operand(msg, ptr_ty)?.into_pointer_value();
        let location_ptr = self.location_string(span);

        let panic_block = self
            .ctx
            .context
            .insert_basic_block_after(self.builder.get_insert_block().unwrap(), "panic");

        // An unconditional `panic(msg)` lowers to an `Assert` whose `target`
        // is never given a real block (see `lower_panic_intrinsic`'s docs:
        // the "ok" path is provably unreachable, so no block is inserted for
        // it) — branching straight to `panic_block` instead of indexing
        // `self.blocks[*target]` avoids an out-of-bounds panic on that case.
        match self.blocks.get(*target) {
            Some(&ok_block) => {
                self.builder
                    .build_conditional_branch(ok, ok_block, panic_block)
                    .map_err(llvm_err)?;
            }
            None => {
                self.builder
                    .build_unconditional_branch(panic_block)
                    .map_err(llvm_err)?;
            }
        }

        self.builder.position_at_end(panic_block);
        let panic_fn = self.panic_function()?;
        self.builder
            .build_call(
                panic_fn,
                &[msg_ptr.into(), location_ptr.into()],
                "panic_call",
            )
            .map_err(llvm_err)?;

        self.builder.build_unreachable().map_err(llvm_err)?;
        Ok(())
    }

    /// `"{path}:{line}:{col}"` for `span`'s *start* position (a single point,
    /// like Rust's own panic locations — not the `start..end` range `Span`'s
    /// `Debug` impl prints for diagnostics), materialized as its own global
    /// string constant (`codegen_string_constant`, reused from `rvalue.rs`).
    /// Falls back to `"<unknown location>"` if `span.module` isn't in
    /// `self.ctx.modules` — should never happen in practice, but this is a
    /// diagnostics nicety, not worth failing the whole codegen pass over.
    fn location_string(&self, span: Span) -> PointerValue<'ctx> {
        let location = match self.ctx.modules.get_path(span.module) {
            Some(path) => format!(
                "{}:{}:{}",
                path.display(),
                span.start.line,
                span.start.offset
            ),
            None => "<unknown location>".to_string(),
        };
        self.codegen_string_constant(&location)
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

    /// The C runtime's `abort()`, declared lazily (once per module) — used
    /// only by `panic_function` now (every panicking construct funnels
    /// through that instead of calling `abort()` directly).
    pub(crate) fn abort_function(&self) -> FunctionValue<'ctx> {
        if let Some(existing) = self.ctx.module.get_function("abort") {
            return existing;
        }
        let fn_type = self.ctx.context.void_type().fn_type(&[], false);
        self.ctx.module.add_function("abort", fn_type, None)
    }

    /// libc's variadic `printf`, declared lazily (once per module) — used
    /// only by `panic_function` to print a panic message before aborting.
    fn printf_function(&self) -> FunctionValue<'ctx> {
        if let Some(existing) = self.ctx.module.get_function("printf") {
            return existing;
        }
        let ptr_ty = self.ctx.context.ptr_type(AddressSpace::default());
        let fn_type = self.ctx.context.i32_type().fn_type(&[ptr_ty.into()], true);
        self.ctx.module.add_function("printf", fn_type, None)
    }

    /// libc's `fflush`, declared lazily (once per module) — `panic_function`
    /// calls this with a null `FILE*` (meaning "every open stream") right
    /// before `abort()`. Without it, `printf`'s message sits in a
    /// fully-buffered `stdout` and is silently lost: `abort()` terminates the
    /// process immediately, it doesn't run libc's normal at-exit flush.
    fn fflush_function(&self) -> FunctionValue<'ctx> {
        if let Some(existing) = self.ctx.module.get_function("fflush") {
            return existing;
        }
        let ptr_ty = self.ctx.context.ptr_type(AddressSpace::default());
        let fn_type = self.ctx.context.i32_type().fn_type(&[ptr_ty.into()], false);
        self.ctx.module.add_function("fflush", fn_type, None)
    }

    /// `"panic: %s\n  at %s\n"`, null-terminated — the one format string
    /// `panic_function` prints every message+location pair through. A
    /// private global rather than a per-call constant since there's only
    /// ever one of these per module (unlike `codegen_string_constant`'s
    /// per-literal globals).
    fn panic_format_string(&self) -> PointerValue<'ctx> {
        let const_str = self.ctx.context.const_string(b"panic: %s\n  at %s\n", true);
        let global = self
            .ctx
            .module
            .add_global(const_str.get_type(), None, "panic_fmt");
        global.set_initializer(&const_str);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.as_pointer_value()
    }

    /// The panic runtime: prints `msg` and `location` (both `cstr`-typed
    /// pointers) via `printf` then calls `abort()` — a Rust-`panic!`-style
    /// trap (no unwinding, no backtrace: this compiler has no unwinding
    /// model) instead of a bare, silent `abort()`. Declared *and defined*
    /// lazily (once per module, cached the same way as `abort_function`) the
    /// first time any panicking construct — an out-of-bounds slice index, an
    /// arithmetic overflow, a MIR-level `assert`/`panic()` — is actually
    /// codegen'd; every one of those funnels through this single function.
    /// Building its body reuses `self.builder` (there's no separate builder
    /// per LLVM function), so the caller's own insertion point is saved and
    /// restored around it.
    pub(crate) fn panic_function(&self) -> CodegenResult<FunctionValue<'ctx>> {
        if let Some(existing) = self.ctx.module.get_function("soul_panic") {
            return Ok(existing);
        }

        let ptr_ty = self.ctx.context.ptr_type(AddressSpace::default());
        let fn_type = self
            .ctx
            .context
            .void_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        let function = self.ctx.module.add_function("soul_panic", fn_type, None);

        let resume_block = self.builder.get_insert_block();
        let entry = self.ctx.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let format = self.panic_format_string();
        let msg = function
            .get_nth_param(0)
            .ok_or_else(|| err(CodegenErrorKind::MissingParameterValue { index: 0 }))?
            .into_pointer_value();
        let location = function
            .get_nth_param(1)
            .ok_or_else(|| err(CodegenErrorKind::MissingParameterValue { index: 1 }))?
            .into_pointer_value();

        let printf_fn = self.printf_function();
        self.builder
            .build_call(
                printf_fn,
                &[format.into(), msg.into(), location.into()],
                "printf_call",
            )
            .map_err(llvm_err)?;

        let fflush_fn = self.fflush_function();
        let null_stream = ptr_ty.const_null();
        self.builder
            .build_call(fflush_fn, &[null_stream.into()], "fflush_call")
            .map_err(llvm_err)?;

        let abort_fn = self.abort_function();
        self.builder
            .build_call(abort_fn, &[], "abort_call")
            .map_err(llvm_err)?;
        self.builder.build_unreachable().map_err(llvm_err)?;

        if let Some(block) = resume_block {
            self.builder.position_at_end(block);
        }

        Ok(function)
    }
}
