//! MIR-to-LLVM-IR codegen. First (smallest-slice) pass: primitive scalar
//! locals only (no aggregates — nothing in MIR produces one yet), stack-slot
//! (`alloca`) locals rather than SSA/phi reconstruction (MIR isn't SSA —
//! e.g. a `while` loop reassigns the same local across blocks — so this
//! mirrors the simplest, well-known "every local gets a stack slot" codegen
//! strategy rather than building a whole SSA-reconstruction pass for M1).
//!
//! Emits an in-memory `inkwell::Module`; turning that into an object file/exe
//! via `llc`/a linker is a separate, currently-manual step (see
//! `docs/compiler-pipeline-plan.md`).

use anyhow::{Context as _, Result, anyhow, bail};
use ast_model::{AstStore, SoulType, operators::BinaryOperatorKind};
use inkwell::{
    IntPredicate,
    basic_block::BasicBlock as LlvmBlock,
    builder::Builder,
    context::Context,
    module::Module,
    types::IntType,
    values::{FunctionValue, IntValue, PointerValue},
};
use mir_model::{BlockId, ConstValue, Function, LocalId, MirProgram, Operand, Rvalue, Statement, Terminator};
use soul_utils::{
    FunctionId, collections::vec_map::{VecMap, VecMapIndex}, soul_names::PrimitiveTypes,
};

/// Builds an LLVM module containing every function in `functions`. `ast` is
/// only used to recover each function's source name (MIR itself only has
/// `FunctionId`s) — the entry point a linker looks for (`main`) has to match
/// the Soul function's actual declared name exactly.
pub fn codegen_module<'ctx>(
    context: &'ctx Context,
    module_name: &str,
    mir: &MirProgram,
    ast: &AstStore,
) -> Result<Module<'ctx>> {

    let module = context.create_module(module_name);
    let mut codegen = ModuleCodegen {
        context,
        module: &module,
        mir,
        ast,
        function_values: VecMap::new(),
    };

    codegen.declare_all_functions()?;
    for (id, function) in mir.functions.entries() {
        codegen.codegen_function(id, function)?;
    }

    Ok(module)
}

fn function_name(ast: &AstStore, id: FunctionId) -> Result<&str> {
    let kind = ast
        .functions
        .get(id)
        .ok_or_else(|| anyhow!("{id:?} has no AST entry"))?;

    Ok(kind.signature().name.as_str())
}

struct ModuleCodegen<'ctx, 'a> {
    ast: &'a AstStore,
    mir: &'a MirProgram,
    context: &'ctx Context,
    module: &'a Module<'ctx>,
    function_values: VecMap<FunctionId, FunctionValue<'ctx>>,
}

impl<'ctx, 'a> ModuleCodegen<'ctx, 'a> {
    /// Declares every function's signature up front, so calls can reference a
    /// callee regardless of definition order (including mutual recursion).
    fn declare_all_functions(&mut self) -> Result<()> {
        for (id, function) in self.mir.functions.entries() {
            let name = function_name(self.ast, id)?;

            let param_types = function
                .locals
                .entries()
                .take(function.arg_count)
                .map(|(_, decl)| llvm_int_type(self.context, &decl.ty))
                .collect::<Result<Vec<_>>>()
                .with_context(|| format!("in `{name}`'s parameters"))?;

            let param_metadata_types = param_types
                .iter()
                .map(|ty| (*ty).into())
                .collect::<Vec<_>>();

            let return_type = function
                .return_local
                .map(|local| llvm_int_type(self.context, &function.locals[local].ty))
                .transpose()
                .with_context(|| format!("in `{name}`'s return type"))?;

            // The C ABI's `main` always returns `i32` (that's what becomes
            // the process exit code) regardless of Soul's declared return
            // type — `codegen_terminator`'s `Return` case zero-extends to
            // match whenever this widening actually applies.
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
        Ok(())
    }

    fn codegen_function(&mut self, id: FunctionId, function: &Function) -> Result<()> {
        let name = function_name(self.ast, id)?;
        let fn_value = *self.function_values.get(id)
            .ok_or_else(|| anyhow!("{id:?} not Found"))?;
        
        let builder = self.context.create_builder();

        // A dedicated `entry` block holding only the parameter/local allocas,
        // ahead of the MIR blocks proper — keeps every alloca in the
        // function's first block (what LLVM's mem2reg pass expects) without
        // having to special-case MIR's own entry block for it.
        let entry = self.context.append_basic_block(fn_value, "entry");
        builder.position_at_end(entry);

        let mut locals: VecMap<LocalId, PointerValue<'ctx>> = VecMap::new();
        for (local_id, decl) in function.locals.entries() {

            let ty = llvm_int_type(self.context, &decl.ty)
                .with_context(|| format!("in `{name}`'s local {local_id:?}"))?;

            let slot = builder
                .build_alloca(ty, &format!("_{}", local_id.index()))
                .map_err(|e| anyhow!("{e}"))?;

            locals.insert(local_id, slot);
        }
        // Params are `locals[0..arg_count]` *by position*, not by a
        // specific `LocalId` value — `LocalId`s are 1-based (whatever
        // `IdGenerator` happens to start at), not the 0-based index a naive
        // `LocalId::new_index(i)` would reconstruct.
        let param_locals: Vec<LocalId> = function
            .locals
            .entries()
            .take(function.arg_count)
            .map(|(id, _)| id)
            .collect();

        for (i, &param_local) in param_locals.iter().enumerate() {
            let param_value = fn_value
                .get_nth_param(i as u32)
                .ok_or_else(|| anyhow!("`{name}` is missing parameter {i}"))?;

            builder
                .build_store(locals[param_local], param_value)?;
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
            .ok_or_else(|| anyhow!("`{name}` has no blocks"))?;

        builder
            .build_unconditional_branch(blocks[mir_entry_id])?;

        let mut fn_codegen = FunctionCodegen {
            context: self.context,
            module: self.module,
            builder,
            function,
            name,
            is_entry_point: name == "main",
            locals,
            blocks,
            function_values: &self.function_values,
            functions: &self.mir.functions,
        };
        for (block_id, block) in function.blocks.entries() {
            fn_codegen.codegen_block(block_id, block)?;
        }

        Ok(())
    }
}

struct FunctionCodegen<'ctx, 'a> {
    context: &'ctx Context,
    module: &'a Module<'ctx>,
    builder: Builder<'ctx>,
    function: &'a Function,
    name: &'a str,
    is_entry_point: bool,
    locals: VecMap<LocalId, PointerValue<'ctx>>,
    blocks: VecMap<BlockId, LlvmBlock<'ctx>>,
    function_values: &'a VecMap<FunctionId, FunctionValue<'ctx>>,
    functions: &'a VecMap<FunctionId, Function>,
}

impl<'ctx, 'a> FunctionCodegen<'ctx, 'a> {
    fn codegen_block(&mut self, block_id: BlockId, block: &mir_model::BasicBlock) -> Result<()> {
        self.builder.position_at_end(self.blocks[block_id]);
        for statement in &block.statements {
            self.codegen_statement(statement)?;
        }
        self.codegen_terminator(&block.terminator)
    }

    fn codegen_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::Assign(place, rvalue) => {
                if !place.projection.is_empty() {
                    bail!(
                        "in `{}`: place projections (field/index/deref) aren't supported in this codegen slice",
                        self.name
                    );
                }
                let ty = self.local_type(place.local)?;
                let value = self.codegen_rvalue(rvalue, ty)?;
                self.builder
                    .build_store(self.locals[place.local], value)
                    .map_err(|e| anyhow!("{e}"))?;
                Ok(())
            }
            // Move/drop tracking has no runtime effect yet (see
            // `docs/mir-design.md`) — nothing to codegen for these until it does.
            Statement::MarkMoved(_) | Statement::SetDropFlag(..) | Statement::StorageDead(_) => {
                Ok(())
            }
        }
    }

    fn codegen_terminator(&mut self, terminator: &Terminator) -> Result<()> {
        match terminator {
            Terminator::Goto(target) => {
                self.builder
                    .build_unconditional_branch(self.blocks[*target])
                    .map_err(|e| anyhow!("{e}"))?;
            }
            Terminator::SwitchInt {
                discriminant,
                targets,
                otherwise,
            } => {
                // Only ever constructed from a bare `bool` condition in this
                // slice (`if`/`while`), so the discriminant is always `i1`.
                let bool_ty = self.context.bool_type();
                let cond = self.codegen_operand(discriminant, bool_ty)?;
                let [(value, target)] = targets.as_slice() else {
                    bail!(
                        "in `{}`: switchInt with != 1 target isn't supported in this codegen slice (only bool if/while conditions are constructed today)",
                        self.name
                    );
                };
                let ConstValue::Bool(expect_true) = value else {
                    bail!(
                        "in `{}`: switchInt target value must be a bool constant in this codegen slice",
                        self.name
                    );
                };
                let (then_block, else_block) = if *expect_true {
                    (self.blocks[*target], self.blocks[*otherwise])
                } else {
                    (self.blocks[*otherwise], self.blocks[*target])
                };
                self.builder
                    .build_conditional_branch(cond, then_block, else_block)
                    .map_err(|e| anyhow!("{e}"))?;
            }
            Terminator::Call {
                id,
                arguments,
                destination,
                target,
            } => {
                let callee_mir = self
                    .functions
                    .get(*id)
                    .ok_or_else(|| anyhow!("call to {id:?} has no MIR body"))?;

                let callee_value = *self
                    .function_values
                    .get(*id)
                    .ok_or_else(|| anyhow!("call to {id:?} was never declared"))?;

                // Same "by position, not by reconstructed `LocalId`" rule as
                // the parameter-store loop in `codegen_function`.
                let callee_param_locals: Vec<LocalId> = callee_mir
                    .locals
                    .entries()
                    .take(callee_mir.arg_count)
                    .map(|(id, _)| id)
                    .collect();
                let mut args = Vec::with_capacity(arguments.len());
                for (i, arg) in arguments.iter().enumerate() {
                    let param_local = *callee_param_locals.get(i).ok_or_else(|| {
                        anyhow!("call to {id:?} passes more arguments than it has parameters")
                    })?;
                    let param_ty = llvm_int_type(self.context, &callee_mir.locals[param_local].ty)
                        .with_context(|| format!("in `{}`'s call argument {i}", self.name))?;
                    args.push(self.codegen_operand(arg, param_ty)?.into());
                }

                let call = self
                    .builder
                    .build_call(callee_value, &args, "call")
                    .map_err(|e| anyhow!("{e}"))?;

                if let Some(place) = destination {
                    if !place.projection.is_empty() {
                        bail!(
                            "in `{}`: place projections aren't supported in this codegen slice",
                            self.name
                        );
                    }
                    let result = call
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow!("call result used but callee returns `none`"))?;
                    self.builder
                        .build_store(self.locals[place.local], result)
                        .map_err(|e| anyhow!("{e}"))?;
                }

                match target {
                    Some(target) => {
                        self.builder
                            .build_unconditional_branch(self.blocks[*target])
                            .map_err(|e| anyhow!("{e}"))?;
                    }
                    None => {
                        self.builder
                            .build_unreachable()
                            .map_err(|e| anyhow!("{e}"))?;
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
                let bool_ty = self.context.bool_type();
                let cond = self.codegen_operand(cond, bool_ty)?;
                let expect_true = self
                    .context
                    .bool_type()
                    .const_int(u64::from(*expected), false);
                let ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, cond, expect_true, "assert_ok")
                    .map_err(|e| anyhow!("{e}"))?;

                let panic_block = self
                    .context
                    .insert_basic_block_after(self.builder.get_insert_block().unwrap(), "panic");
                self.builder
                    .build_conditional_branch(ok, self.blocks[*target], panic_block)
                    .map_err(|e| anyhow!("{e}"))?;

                self.builder.position_at_end(panic_block);
                let abort_fn = self.abort_function()?;
                self.builder
                    .build_call(abort_fn, &[], "abort_call")
                    .map_err(|e| anyhow!("{e}"))?;
                self.builder
                    .build_unreachable()
                    .map_err(|e| anyhow!("{e}"))?;
            }
            Terminator::Return => match self.function.return_local {
                Some(local) => {
                    let ty = self.local_type(local)?;
                    let value = self
                        .builder
                        .build_load(ty, self.locals[local], "ret")
                        .map_err(|e| anyhow!("{e}"))?
                        .into_int_value();
                    // `main`'s LLVM-level return is forced to `i32` (see
                    // `declare_all_functions`) regardless of Soul's declared
                    // return type — widen to match, since the process exit
                    // code convention needs that exact ABI.
                    if self.is_entry_point {
                        let i32_ty = self.context.i32_type();
                        let widened = self
                            .builder
                            .build_int_z_extend(value, i32_ty, "exit_code")
                            .map_err(|e| anyhow!("{e}"))?;
                        self.builder
                            .build_return(Some(&widened))
                            .map_err(|e| anyhow!("{e}"))?;
                    } else {
                        self.builder
                            .build_return(Some(&value))
                            .map_err(|e| anyhow!("{e}"))?;
                    }
                }
                None if self.is_entry_point => {
                    let zero = self.context.i32_type().const_zero();
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| anyhow!("{e}"))?;
                }
                None => {
                    self.builder
                        .build_return(None)
                        .map_err(|e| anyhow!("{e}"))?;
                }
            },
            Terminator::Unreachable => {
                self.builder
                    .build_unreachable()
                    .map_err(|e| anyhow!("{e}"))?;
            }
            Terminator::Drop { .. } => {
                bail!(
                    "in `{}`: `Drop` isn't constructed by lowering yet and isn't supported in codegen either",
                    self.name
                );
            }
        }
        Ok(())
    }

    /// The C runtime's `abort()`, declared lazily (once per module) the
    /// first time an `assert`/`panic` is actually codegen'd.
    fn abort_function(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(existing) = self.module.get_function("abort") {
            return Ok(existing);
        }
        let fn_type = self.context.void_type().fn_type(&[], false);
        Ok(self.module.add_function("abort", fn_type, None))
    }

    fn codegen_rvalue(
        &mut self,
        rvalue: &Rvalue,
        result_ty: IntType<'ctx>,
    ) -> Result<IntValue<'ctx>> {
        match rvalue {
            Rvalue::Use(operand) => self.codegen_operand(operand, result_ty),
            Rvalue::BinaryOp(op, left, right) => {
                let operand_ty = self
                    .operand_type(left)
                    .or_else(|| self.operand_type(right))
                    .unwrap_or(result_ty);
                
                let l = self.codegen_operand(left, operand_ty)?;
                let r = self.codegen_operand(right, operand_ty)?;
                let signed = self.operand_is_signed(left) || self.operand_is_signed(right);
                self.codegen_binary_op(*op, l, r, signed)
            }
            Rvalue::UnaryOp(op, operand) => {
                let bool_ty = self.context.bool_type();
                let value = self.codegen_operand(operand, bool_ty)?;
                match op {
                    ast_model::operators::UnaryOperatorKind::Not => Ok(self
                        .builder
                        .build_not(value, "not")
                        .map_err(|e| anyhow!("{e}"))?),
                    other => bail!(
                        "in `{}`: unary operator `{other:?}` isn't supported in this codegen slice",
                        self.name
                    ),
                }
            }
            Rvalue::Ref { .. } | Rvalue::Aggregate(..) | Rvalue::Cast(..) => {
                bail!(
                    "in `{}`: references/aggregates/casts aren't supported in this codegen slice",
                    self.name
                )
            }
        }
    }

    fn codegen_binary_op(
        &mut self,
        op: BinaryOperatorKind,
        l: IntValue<'ctx>,
        r: IntValue<'ctx>,
        signed: bool,
    ) -> Result<IntValue<'ctx>> {
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
            other => bail!(
                "in `{}`: binary operator `{other:?}` isn't supported in this codegen slice",
                self.name
            ),
        };
        v.map_err(|e| anyhow!("{e}"))
    }

    fn codegen_operand(&mut self, operand: &Operand, ty: IntType<'ctx>) -> Result<IntValue<'ctx>> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if !place.projection.is_empty() {
                    bail!(
                        "in `{}`: place projections aren't supported in this codegen slice",
                        self.name
                    );
                }
                let value = self
                    .builder
                    .build_load(ty, self.locals[place.local], "load")
                    .map_err(|e| anyhow!("{e}"))?;
                Ok(value.into_int_value())
            }
            Operand::Constant(value) => const_int(ty, value),
        }
    }

    fn local_type(&self, local: LocalId) -> Result<IntType<'ctx>> {
        llvm_int_type(self.context, &self.function.locals[local].ty)
            .with_context(|| format!("in `{}`'s local {local:?}", self.name))
    }

    /// The LLVM type a place-backed operand is stored as, if it is one — a
    /// bare constant operand carries no type of its own (see the module docs).
    fn operand_type(&self, operand: &Operand) -> Option<IntType<'ctx>> {
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

fn const_int<'ctx>(ty: IntType<'ctx>, value: &ConstValue) -> Result<IntValue<'ctx>> {
    Ok(match value {
        ConstValue::Bool(b) => ty.const_int(u64::from(*b), false),
        ConstValue::Int(n) => ty.const_int(*n as u64, true),
        ConstValue::Uint(n) => ty.const_int(*n as u64, false),
        other => bail!("constant `{other:?}` isn't supported in this codegen slice"),
    })
}

fn is_signed(prim: PrimitiveTypes) -> bool {
    use PrimitiveTypes::*;
    matches!(
        prim,
        CInt | UntypedInt | Int | Int8 | Int16 | Int32 | Int64 | Int128
    )
}

fn llvm_int_type<'ctx>(context: &'ctx Context, ty: &SoulType) -> Result<IntType<'ctx>> {
    let SoulType::Primitive(prim) = ty else {
        bail!("type `{ty:?}` isn't a primitive scalar, which is all this codegen slice supports");
    };
    use PrimitiveTypes::*;
    Ok(match prim {
        Boolean => context.bool_type(),
        Int8 | Uint8 => context.i8_type(),
        Int16 | Uint16 | Char16 => context.i16_type(),
        Int32 | Uint32 | Char | Char32 => context.i32_type(),
        Int64 | Uint64 | Char64 => context.i64_type(),
        Int128 | Uint128 => context.i128_type(),
        // Platform-sized: this codegen slice only targets 64-bit hosts.
        Int | Uint | CInt | CUint | UntypedInt | UntypedUint => context.i64_type(),
        Char8 => context.i8_type(),
        other => bail!("type `{other:?}` isn't supported in this codegen slice"),
    })
}
