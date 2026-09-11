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
//!
//! Split into: `ctx` (the small `Copy` bundle of context/module/declares/
//! platform that `ModuleCodegen` and `FunctionCodegen` each hold a copy of,
//! rather than duplicating those four fields directly), `module` (declares
//! every function/extern's LLVM signature, then drives one `FunctionCodegen`
//! per real function body), `function` (per-function driver —
//! blocks/statements — and place resolution), `terminator`
//! (terminator-specific codegen: `Call`/`Assert`/`SwitchInt`/`Return`),
//! `rvalue` (operand/rvalue-to-`BasicValueEnum` codegen, a distinct concern
//! from control flow), and `types` (the Soul-type-to-LLVM-type mapping and
//! the small constant/int-width helpers that go with it).

pub mod fault;

mod ctx;
mod function;
mod module;
mod rvalue;
mod terminator;
mod types;

use ast_model::AstTree;
use inkwell::{builder::BuilderError, context::Context, module::Module};
use mir_model::MirProgram;
use soul_utils::{compiler_options::CompilerOptions, fault::Fault};

use crate::{
    fault::{CodegenErrorKind, CodegenResult},
    module::codegen_module,
};

pub fn to_llvm<'ctx>(
    context: &'ctx Context,
    mir: &MirProgram,
    ast: &AstTree,
    options: &CompilerOptions,
) -> CodegenResult<Module<'ctx>> {
    codegen_module(
        context,
        "soul_module",
        mir,
        &ast.crates.store,
        &ast.declares,
        options,
    )
}

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
