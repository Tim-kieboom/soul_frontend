//! Driver that runs the MIR-lowering stage over an already-resolved `AstTree`,
//! mirroring `ast_run::to_ast` — that crate wires tokenizer+parser+resolver
//! together, this one wires the (much narrower, see `mir_parser`) MIR-lowering
//! pass on top of its output. Kept as a separate stage rather than folded into
//! `to_ast` itself: MIR lowering is a genuinely separate pipeline step (see
//! `docs/mir-design.md`), and today's lowering pass only covers a small subset
//! of the language, so its failures shouldn't be conflated with "the program
//! failed to parse/resolve."
//!
//! Failures use the same fault-reporting mechanism as every other stage: a
//! function this slice can't lower doesn't return an error to the caller here,
//! it pushes a `Fault` into `context` (see `mir_parser`'s docs for why that's
//! the expected, non-fatal outcome for most of the language today) — so a
//! caller collects/prints MIR faults exactly the way it already collects parse
//! and resolve faults, rather than a separate ad hoc error list.

#[cfg(test)]
mod tests;

use std::time::Instant;

use ast_model::{AstTree, FunctionKind};
use mir_model::Function;
use mir_parser::{fault::MirErrorKind, lower_function};
use soul_utils::{
    CrateContext, FunctionId,
    collections::{benchmark::Benchmark, vec_map::VecMap},
    compiler_options::CompilerOptions,
};

pub struct MirProgram {
    pub functions: VecMap<FunctionId, Function>,
}
impl MirProgram {
    pub const fn empty() -> Self {
        Self {
            functions: VecMap::const_default(),
        }
    }
}

pub fn to_mir<K>(
    ast: &AstTree<K>,
    benchmark: &mut Benchmark,
    context: &mut CrateContext<MirErrorKind>,
    _options: &CompilerOptions,
) -> MirProgram {
    let time = Instant::now();

    let mut functions = VecMap::new();
    for (id, kind) in ast.crates.store.functions.entries() {
        if !matches!(kind, FunctionKind::Normal(_)) {
            continue;
        }

        match lower_function(&ast.crates.store, &ast.declares, id) {
            Ok(mir_function) => {
                functions.insert(id, mir_function);
            }
            Err(fault) => context.faults.push(fault),
        }
    }

    benchmark.add_benchmark("mir", time.elapsed());
    MirProgram { functions }
}
