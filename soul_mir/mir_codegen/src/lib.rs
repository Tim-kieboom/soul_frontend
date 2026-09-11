//! MIR-to-LLVM-IR codegen. First (smallest-slice) pass: primitive scalar and
//! pointer (`cstr`/references) locals only (no aggregates — nothing in MIR
//! produces one yet), stack-slot (`alloca`) locals rather than SSA/phi
//! reconstruction (MIR isn't SSA — e.g. a `while` loop reassigns the same
//! local across blocks — so this mirrors the simplest, well-known "every
//! local gets a stack slot" codegen strategy rather than building a whole
//! SSA-reconstruction pass for M1).
//!
//! Values are represented as inkwell's own `BasicValueEnum`/`BasicTypeEnum`
//! (int-or-pointer-or-...) rather than a narrower `IntValue`-only type,
//! since `extern "C"` calls need pointer-typed parameters/arguments. Every
//! site that needs specifically an int (arithmetic/comparison operators,
//! branch conditions) checks explicitly via `expect_int` rather than calling
//! `BasicValueEnum::into_int_value()`, which panics on a mismatch instead of
//! producing a fault — this pass never trusts an earlier stage to rule that
//! out by construction.
//!
//! Emits an in-memory `inkwell::Module`; turning that into an object file/exe
//! via `llc`/a linker is a separate, currently-manual step (see
//! `docs/compiler-pipeline-plan.md`).

pub mod fault;

use std::cell::Cell;

use ast_model::{AstStore, SoulType, operators::BinaryOperatorKind};
use inkwell::{
    AddressSpace, IntPredicate,
    basic_block::BasicBlock as LlvmBlock,
    builder::{Builder, BuilderError},
    context::Context,
    module::{Linkage, Module},
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, IntType},
    values::{BasicValueEnum, FunctionValue, IntValue, PointerValue},
};
use mir_model::{
    BlockId, ConstValue, ExternFunction, Function, LocalId, MirProgram, Operand, Rvalue, Statement,
    Terminator,
};
use soul_utils::{
    FunctionId,
    collections::vec_map::{VecMap, VecMapIndex},
    compiler_options::{CompilerOptions, PlatformInfo},
    fault::Fault,
    soul_names::PrimitiveTypes,
    span::Span,
};

use crate::fault::{CodegenErrorKind, CodegenResult};

fn llvm_err(err: BuilderError) -> Fault<CodegenErrorKind> {
    Fault::error_with_kind(
        CodegenErrorKind::LlvmBuilderError {
            message: err.to_string().into_boxed_str(),
        },
        None,
    )
}

fn err(kind: CodegenErrorKind) -> Fault<CodegenErrorKind> {
    Fault::error_with_kind(kind, None)
}

/// Narrows a value to an `IntValue`, faulting (not panicking) if it's
/// actually a pointer — the guard every int-only operator/branch-condition
/// site goes through.
fn expect_int(value: BasicValueEnum<'_>) -> CodegenResult<IntValue<'_>> {
    match value {
        BasicValueEnum::IntValue(v) => Ok(v),
        _ => Err(err(CodegenErrorKind::ExpectedIntOperand)),
    }
}

/// Builds an LLVM module containing every function/extern declaration in
/// `mir`. `ast` is only used to recover each function's source name (MIR
/// itself only has `FunctionId`s) — the entry point a linker looks for
/// (`main`) has to match the Soul function's actual declared name exactly.
pub fn codegen_module<'ctx>(
    context: &'ctx Context,
    module_name: &str,
    mir: &MirProgram,
    ast: &AstStore,
    options: &CompilerOptions,
) -> CodegenResult<Module<'ctx>> {
    let module = context.create_module(module_name);
    let mut codegen = ModuleCodegen {
        context,
        module: &module,
        mir,
        ast,
        platform: options.platform,
        function_values: VecMap::new(),
        string_counter: Cell::new(0),
    };

    codegen.declare_all_functions()?;
    for (id, function) in mir.functions.entries() {
        codegen.codegen_function(id, function)?;
    }

    Ok(module)
}

fn function_name(ast: &AstStore, id: FunctionId) -> CodegenResult<&str> {
    let kind = ast
        .functions
        .get(id)
        .ok_or_else(|| err(CodegenErrorKind::MissingAstEntry { id }))?;

    Ok(kind.signature().name.as_str())
}

struct ModuleCodegen<'ctx, 'a> {
    ast: &'a AstStore,
    mir: &'a MirProgram,
    context: &'ctx Context,
    module: &'a Module<'ctx>,
    platform: PlatformInfo,
    function_values: VecMap<FunctionId, FunctionValue<'ctx>>,
    /// Shared across every function's codegen so string-literal globals get
    /// module-wide-unique names, not per-function-restarting ones.
    string_counter: Cell<usize>,
}

impl<'ctx, 'a> ModuleCodegen<'ctx, 'a> {
    /// Declares every function's signature up front, so calls can reference a
    /// callee regardless of definition order (including mutual recursion) —
    /// and declares every `extern "C"` function as a bodyless external
    /// declaration (never passed to `codegen_function`, so no blocks are
    /// ever appended to it — that omission alone is what makes it a true
    /// external declaration rather than a defined-but-empty function).
    fn declare_all_functions(&mut self) -> CodegenResult<()> {
        for (id, function) in self.mir.functions.entries() {
            let name = function_name(self.ast, id)?;

            let param_types = function
                .locals
                .entries()
                .take(function.arg_count)
                .map(|(_, decl)| llvm_type(self.context, &self.platform, &decl.ty, Some(decl.span)))
                .collect::<CodegenResult<Vec<_>>>()?;

            let param_metadata_types = param_metadata(&param_types);

            let return_type = function
                .return_local
                .map(|local| {
                    let decl = &function.locals[local];
                    llvm_type(self.context, &self.platform, &decl.ty, Some(decl.span))
                })
                .transpose()?;

            // The C ABI's `main` always returns `i32` (that's what becomes
            // the process exit code) regardless of Soul's declared return
            // type — `codegen_terminator`'s `Return` case widens to match
            // whenever this differs from the natural return type.
            let fn_type = if name == "main" {
                self.context
                    .i32_type()
                    .fn_type(&param_metadata_types, false)
            } else {
                match return_type {
                    Some(ty) => ty.fn_type(&param_metadata_types, false),
                    None => self
                        .context
                        .void_type()
                        .fn_type(&param_metadata_types, false),
                }
            };

            let fn_value = self.module.add_function(name, fn_type, None);
            self.function_values.insert(id, fn_value);
        }

        for (id, extern_fn) in self.mir.externs.entries() {
            let name = function_name(self.ast, id)?;

            let param_types = extern_fn
                .params
                .iter()
                .map(|ty| llvm_type(self.context, &self.platform, ty, None))
                .collect::<CodegenResult<Vec<_>>>()?;
            let param_metadata_types = param_metadata(&param_types);

            let fn_type = match &extern_fn.return_type {
                Some(ty) => llvm_type(self.context, &self.platform, ty, None)?
                    .fn_type(&param_metadata_types, false),
                None => self
                    .context
                    .void_type()
                    .fn_type(&param_metadata_types, false),
            };

            let fn_value = self
                .module
                .add_function(name, fn_type, Some(Linkage::External));
            self.function_values.insert(id, fn_value);
        }
        Ok(())
    }

    fn codegen_function(&mut self, id: FunctionId, function: &Function) -> CodegenResult<()> {
        let name = function_name(self.ast, id)?;
        let fn_value = *self
            .function_values
            .get(id)
            .ok_or_else(|| err(CodegenErrorKind::MissingAstEntry { id }))?;

        let builder = self.context.create_builder();

        // A dedicated `entry` block holding only the parameter/local allocas,
        // ahead of the MIR blocks proper — keeps every alloca in the
        // function's first block (what LLVM's mem2reg pass expects) without
        // having to special-case MIR's own entry block for it.
        let entry = self.context.append_basic_block(fn_value, "entry");
        builder.position_at_end(entry);

        let mut locals: VecMap<LocalId, PointerValue<'ctx>> = VecMap::new();
        for (local_id, decl) in function.locals.entries() {
            let ty = llvm_type(self.context, &self.platform, &decl.ty, Some(decl.span))?;
            let slot = builder
                .build_alloca(ty, &format!("_{}", local_id.index()))
                .map_err(llvm_err)?;
            locals.insert(local_id, slot);
        }

        // Params are `locals[0..arg_count]` *by position*: the actual
        // `LocalId`s backing them aren't necessarily `0..arg_count` as
        // values (whatever `IdGenerator` happens to start at), so they're
        // read off `function.locals` itself rather than reconstructed.
        let param_locals: Vec<LocalId> = function
            .locals
            .entries()
            .take(function.arg_count)
            .map(|(id, _)| id)
            .collect();

        for (i, &param_local) in param_locals.iter().enumerate() {
            let param_value = fn_value
                .get_nth_param(i as u32)
                .ok_or_else(|| err(CodegenErrorKind::MissingParameterValue { index: i }))?;

            builder
                .build_store(locals[param_local], param_value)
                .map_err(llvm_err)?;
        }

        let mut blocks: VecMap<BlockId, LlvmBlock<'ctx>> = VecMap::new();
        for (block_id, _) in function.blocks.entries() {
            let llvm_block = self
                .context
                .append_basic_block(fn_value, &format!("bb{}", block_id.index()));
            blocks.insert(block_id, llvm_block);
        }

        // MIR's entry block is always whichever `BlockId` was allocated first
        // in `lower()` (every construct allocates its own block ids only
        // after that), so it's always the lowest-index — and first via
        // `entries()` — block in the function.
        let (mir_entry_id, _) = function
            .blocks
            .entries()
            .next()
            .ok_or_else(|| err(CodegenErrorKind::FunctionHasNoBlocks))?;

        builder
            .build_unconditional_branch(blocks[mir_entry_id])
            .map_err(llvm_err)?;

        let mut fn_codegen = FunctionCodegen {
            context: self.context,
            module: self.module,
            platform: self.platform,
            builder,
            function,
            is_entry_point: name == "main",
            locals,
            blocks,
            function_values: &self.function_values,
            functions: &self.mir.functions,
            externs: &self.mir.externs,
            string_counter: &self.string_counter,
        };
        for (block_id, block) in function.blocks.entries() {
            fn_codegen.codegen_block(block_id, block)?;
        }

        Ok(())
    }
}

fn param_metadata<'ctx>(types: &[BasicTypeEnum<'ctx>]) -> Vec<BasicMetadataTypeEnum<'ctx>> {
    types.iter().map(|ty| (*ty).into()).collect()
}

struct FunctionCodegen<'ctx, 'a> {
    context: &'ctx Context,
    module: &'a Module<'ctx>,
    platform: PlatformInfo,
    builder: Builder<'ctx>,
    function: &'a Function,
    is_entry_point: bool,
    locals: VecMap<LocalId, PointerValue<'ctx>>,
    blocks: VecMap<BlockId, LlvmBlock<'ctx>>,
    function_values: &'a VecMap<FunctionId, FunctionValue<'ctx>>,
    functions: &'a VecMap<FunctionId, Function>,
    externs: &'a VecMap<FunctionId, ExternFunction>,
    string_counter: &'a Cell<usize>,
}

impl<'ctx, 'a> FunctionCodegen<'ctx, 'a> {
    fn codegen_block(
        &mut self,
        block_id: BlockId,
        block: &mir_model::BasicBlock,
    ) -> CodegenResult<()> {
        self.builder.position_at_end(self.blocks[block_id]);
        for statement in &block.statements {
            self.codegen_statement(statement)?;
        }
        self.codegen_terminator(&block.terminator)
    }

    fn codegen_statement(&mut self, statement: &Statement) -> CodegenResult<()> {
        match statement {
            Statement::Assign(place, rvalue) => {
                if !place.projection.is_empty() {
                    return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
                }
                let ty = self.local_type(place.local)?;
                let value = self.codegen_rvalue(rvalue, ty)?;
                self.builder
                    .build_store(self.locals[place.local], value)
                    .map_err(llvm_err)?;
                Ok(())
            }
            // Move/drop tracking has no runtime effect yet (see
            // `docs/mir-design.md`) — nothing to codegen for these until it does.
            Statement::MarkMoved(_) | Statement::SetDropFlag(..) | Statement::StorageDead(_) => {
                Ok(())
            }
        }
    }

    fn codegen_terminator(&mut self, terminator: &Terminator) -> CodegenResult<()> {
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
                // Only ever constructed from a bare `bool` condition in this
                // slice (`if`/`while`), so the discriminant is always `i1`.
                let bool_ty = self.context.bool_type().into();
                let cond = expect_int(self.codegen_operand(discriminant, bool_ty)?)?;
                let [(value, target)] = targets.as_slice() else {
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
            }
            Terminator::Call {
                id,
                arguments,
                destination,
                target,
            } => {
                // A callee is either a real `Function` (with a body, so its
                // param types come from its locals) or an `extern "C"`
                // declaration (no body, types come straight from the
                // `ExternFunction` signature) — never both, never neither.
                let param_types = if let Some(callee_mir) = self.functions.get(*id) {
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
                            llvm_type(self.context, &self.platform, &decl.ty, Some(decl.span))
                        })
                        .collect::<CodegenResult<Vec<_>>>()?
                } else if let Some(extern_fn) = self.externs.get(*id) {
                    extern_fn
                        .params
                        .iter()
                        .map(|ty| llvm_type(self.context, &self.platform, ty, None))
                        .collect::<CodegenResult<Vec<_>>>()?
                } else {
                    return Err(err(CodegenErrorKind::CallHasNoMirBody { id: *id }));
                };

                let callee_value = *self
                    .function_values
                    .get(*id)
                    .ok_or_else(|| err(CodegenErrorKind::CallNeverDeclared { id: *id }))?;

                if arguments.len() > param_types.len() {
                    return Err(err(CodegenErrorKind::CallArgumentCountMismatch { id: *id }));
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
            }
            Terminator::Assert {
                cond,
                expected,
                target,
                ..
            } => {
                // The message is deliberately not surfaced yet (no I/O in
                // this slice, see `docs/compiler-pipeline-plan.md`'s M1
                // scope) — a failed assert/an unconditional `panic` both just
                // abort the process.
                let bool_ty = self.context.bool_type().into();
                let cond = expect_int(self.codegen_operand(cond, bool_ty)?)?;
                let expect_true = self
                    .context
                    .bool_type()
                    .const_int(u64::from(*expected), false);
                let ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, cond, expect_true, "assert_ok")
                    .map_err(llvm_err)?;

                let panic_block = self
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
            }
            Terminator::Return => match self.function.return_local {
                Some(local) => {
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
                }
                None if self.is_entry_point => {
                    let zero = self.context.i32_type().const_zero();
                    self.builder.build_return(Some(&zero)).map_err(llvm_err)?;
                }
                None => {
                    self.builder.build_return(None).map_err(llvm_err)?;
                }
            },
            Terminator::Unreachable => {
                self.builder.build_unreachable().map_err(llvm_err)?;
            }
            Terminator::Drop { .. } => {
                return Err(err(CodegenErrorKind::DropUnsupported));
            }
        }
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
        let i32_ty = self.context.i32_type();
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
    /// first time an `assert`/`panic` is actually codegen'd.
    fn abort_function(&self) -> FunctionValue<'ctx> {
        if let Some(existing) = self.module.get_function("abort") {
            return existing;
        }
        let fn_type = self.context.void_type().fn_type(&[], false);
        self.module.add_function("abort", fn_type, None)
    }

    /// Materializes a Soul string literal as a null-terminated LLVM global
    /// byte-array constant and returns a pointer to it — the only way a
    /// `cstr` value currently comes into existence (there's no other
    /// `cstr`-producing expression in this slice, so this is the sole
    /// producer of one). Not deduplicated across equal literals: correctness
    /// over compactness for this first slice.
    fn codegen_string_constant(&mut self, s: &str) -> PointerValue<'ctx> {
        let id = self.string_counter.get();
        self.string_counter.set(id + 1);

        let const_str = self.context.const_string(s.as_bytes(), true);
        let global = self
            .module
            .add_global(const_str.get_type(), None, &format!("str.{id}"));
        global.set_initializer(&const_str);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.as_pointer_value()
    }

    fn codegen_rvalue(
        &mut self,
        rvalue: &Rvalue,
        result_ty: BasicTypeEnum<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match rvalue {
            Rvalue::Use(operand) => self.codegen_operand(operand, result_ty),
            Rvalue::BinaryOp(op, left, right) => {
                let operand_ty = self
                    .operand_type(left)
                    .or_else(|| self.operand_type(right))
                    .unwrap_or(result_ty);

                let l = expect_int(self.codegen_operand(left, operand_ty)?)?;
                let r = expect_int(self.codegen_operand(right, operand_ty)?)?;
                let signed = self.operand_is_signed(left) || self.operand_is_signed(right);
                Ok(self.codegen_binary_op(*op, l, r, signed)?.into())
            }
            Rvalue::UnaryOp(op, operand) => {
                let bool_ty = self.context.bool_type().into();
                let value = expect_int(self.codegen_operand(operand, bool_ty)?)?;
                match op {
                    ast_model::operators::UnaryOperatorKind::Not => Ok(self
                        .builder
                        .build_not(value, "not")
                        .map_err(llvm_err)?
                        .into()),
                    other => Err(err(CodegenErrorKind::UnsupportedUnaryOperator {
                        op: format!("{other:?}").into_boxed_str(),
                    })),
                }
            }
            Rvalue::Ref { .. } | Rvalue::Aggregate(..) | Rvalue::Cast(..) => {
                Err(err(CodegenErrorKind::UnsupportedRvalue))
            }
        }
    }

    fn codegen_binary_op(
        &mut self,
        op: BinaryOperatorKind,
        l: IntValue<'ctx>,
        r: IntValue<'ctx>,
        signed: bool,
    ) -> CodegenResult<IntValue<'ctx>> {
        use BinaryOperatorKind::*;
        let b = &self.builder;
        let v = match op {
            Add => b.build_int_add(l, r, "add"),
            Sub => b.build_int_sub(l, r, "sub"),
            Mul => b.build_int_mul(l, r, "mul"),
            Div if signed => b.build_int_signed_div(l, r, "sdiv"),
            Div => b.build_int_unsigned_div(l, r, "udiv"),
            Mod if signed => b.build_int_signed_rem(l, r, "srem"),
            Mod => b.build_int_unsigned_rem(l, r, "urem"),
            Eq => b.build_int_compare(IntPredicate::EQ, l, r, "eq"),
            NotEq => b.build_int_compare(IntPredicate::NE, l, r, "ne"),
            Lt if signed => b.build_int_compare(IntPredicate::SLT, l, r, "lt"),
            Lt => b.build_int_compare(IntPredicate::ULT, l, r, "lt"),
            Gt if signed => b.build_int_compare(IntPredicate::SGT, l, r, "gt"),
            Gt => b.build_int_compare(IntPredicate::UGT, l, r, "gt"),
            Le if signed => b.build_int_compare(IntPredicate::SLE, l, r, "le"),
            Le => b.build_int_compare(IntPredicate::ULE, l, r, "le"),
            Ge if signed => b.build_int_compare(IntPredicate::SGE, l, r, "ge"),
            Ge => b.build_int_compare(IntPredicate::UGE, l, r, "ge"),
            LogAnd => b.build_and(l, r, "and"),
            LogOr => b.build_or(l, r, "or"),
            other => {
                return Err(err(CodegenErrorKind::UnsupportedBinaryOperator {
                    op: format!("{other:?}").into_boxed_str(),
                }));
            }
        };
        v.map_err(llvm_err)
    }

    fn codegen_operand(
        &mut self,
        operand: &Operand,
        ty: BasicTypeEnum<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if !place.projection.is_empty() {
                    return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
                }
                self.builder
                    .build_load(ty, self.locals[place.local], "load")
                    .map_err(llvm_err)
            }
            Operand::Constant(value) => self.codegen_constant(ty, value),
        }
    }

    fn codegen_constant(
        &mut self,
        ty: BasicTypeEnum<'ctx>,
        value: &ConstValue,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match ty {
            BasicTypeEnum::IntType(int_ty) => Ok(const_int(int_ty, value)?.into()),
            BasicTypeEnum::PointerType(_) => match value {
                ConstValue::Str(s) | ConstValue::Cstr(s) => {
                    Ok(self.codegen_string_constant(s).into())
                }
                other => Err(err(CodegenErrorKind::UnsupportedConstant {
                    value: format!("{other:?}").into_boxed_str(),
                })),
            },
            other => Err(err(CodegenErrorKind::UnsupportedPrimitiveType {
                ty: format!("{other:?}").into_boxed_str(),
            })),
        }
    }

    fn local_type(&self, local: LocalId) -> CodegenResult<BasicTypeEnum<'ctx>> {
        let decl = &self.function.locals[local];
        llvm_type(self.context, &self.platform, &decl.ty, Some(decl.span))
    }

    /// The LLVM type a place-backed operand is stored as, if it is one — a
    /// bare constant operand carries no type of its own (see the module docs).
    fn operand_type(&self, operand: &Operand) -> Option<BasicTypeEnum<'ctx>> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) if place.projection.is_empty() => {
                self.local_type(place.local).ok()
            }
            _ => None,
        }
    }

    fn operand_is_signed(&self, operand: &Operand) -> bool {
        match operand {
            Operand::Copy(place) | Operand::Move(place) if place.projection.is_empty() => {
                matches!(&self.function.locals[place.local].ty, SoulType::Primitive(p) if is_signed(*p))
            }
            Operand::Constant(ConstValue::Int(_)) => true,
            _ => false,
        }
    }
}

fn const_int<'ctx>(ty: IntType<'ctx>, value: &ConstValue) -> CodegenResult<IntValue<'ctx>> {
    Ok(match value {
        ConstValue::Bool(b) => ty.const_int(u64::from(*b), false),
        ConstValue::Int(n) => ty.const_int(*n as u64, true),
        ConstValue::Uint(n) => ty.const_int(*n as u64, false),
        other => {
            return Err(err(CodegenErrorKind::UnsupportedConstant {
                value: format!("{other:?}").into_boxed_str(),
            }));
        }
    })
}

fn is_signed(prim: PrimitiveTypes) -> bool {
    use PrimitiveTypes::*;
    matches!(
        prim,
        CInt | UntypedInt | Int | Int8 | Int16 | Int32 | Int64 | Int128
    )
}

/// Maps a Soul type to its LLVM representation. Integers/`bool` map to the
/// matching `IntType`; `cstr` and any reference/pointer type map to an
/// (opaque, LLVM-16-style) pointer type — everything else (aggregates,
/// floats, ...) isn't supported in this codegen slice yet.
fn llvm_type<'ctx>(
    context: &'ctx Context,
    platform: &PlatformInfo,
    ty: &SoulType,
    span: Option<Span>,
) -> CodegenResult<BasicTypeEnum<'ctx>> {
    use PrimitiveTypes::*;
    match ty {
        SoulType::Primitive(prim) => Ok(match prim {
            Boolean => context.bool_type().into(),
            Int8 | Uint8 => context.i8_type().into(),
            Int16 | Uint16 | Char16 => context.i16_type().into(),
            Int32 | Uint32 | Char | Char32 => context.i32_type().into(),
            Int64 | Uint64 | Char64 => context.i64_type().into(),
            Int128 | Uint128 => context.i128_type().into(),
            // Platform-sized (pointer-width).
            Int | Uint | UntypedInt | UntypedUint => {
                context.custom_width_int_type(platform.pointer_bits).into()
            }
            // C's `int`/`unsigned int` — always 32 bits here, regardless of pointer width.
            CInt | CUint => context.custom_width_int_type(platform.c_int_bits).into(),
            Char8 => context.i8_type().into(),
            CStr => context.ptr_type(AddressSpace::default()).into(),
            other => {
                return Err(Fault::error_with_kind(
                    CodegenErrorKind::UnsupportedPrimitiveType {
                        ty: format!("{other:?}").into_boxed_str(),
                    },
                    span,
                ));
            }
        }),
        SoulType::Reference(_) | SoulType::Pointer(_) => {
            Ok(context.ptr_type(AddressSpace::default()).into())
        }
        other => Err(Fault::error_with_kind(
            CodegenErrorKind::NonPrimitiveType {
                ty: format!("{other:?}").into_boxed_str(),
            },
            span,
        )),
    }
}
