//! AST-to-MIR lowering. First (smallest-testable) slice only: a single non-generic
//! function whose parameters/return/locals are all primitive scalars, whose body is
//! a flat sequence of `name := <expr>` variable declarations built from arithmetic
//! and (arbitrarily nested) sub-expressions, followed by exactly one `return <expr>`.
//! No control flow, no calls, no aggregates yet — each is a separate follow-on slice
//! (see `docs/mir-design.md` at the repo root).
//!
//! Anything outside that subset is rejected with a `Fault` rather than panicking or
//! silently mis-lowering — the same fault-reporting mechanism (`soul_utils::fault`)
//! every other pipeline stage uses, so a caller collects/prints lowering failures the
//! same way it collects parse or resolve faults. This pass runs on already-name-
//! resolved, well-typed input, so every fault here means "not supported by this
//! slice yet," not "the input program is invalid."

pub mod fault;
#[cfg(test)]
mod tests;

use ast_model::{self as ast, AstStore, declare_store::DeclareStore};

use mir_model as mir;
use soul_utils::{FunctionId, collections::vec_map::VecMap, fault::Fault};

use crate::{
    fault::{MirErrorKind, MirResult},
    function::FunctionLowerer,
};

mod function;

pub struct MirLowerer<'a> {
    store: &'a AstStore,
    function_lowerer: FunctionLowerer<'a>,
    functions: VecMap<FunctionId, mir::Function>,
}
impl<'a> MirLowerer<'a> {
    pub fn new(store: &'a AstStore, declares: &'a DeclareStore) -> Self {
        Self {
            functions: VecMap::new(),
            function_lowerer: FunctionLowerer::new(store, declares),
            store,
        }
    }

    pub fn into_functions(self) -> VecMap<FunctionId, mir::Function> {
        self.functions
    }

    pub fn lower_function(&mut self, id: FunctionId) -> MirResult<()> {
        let function = match &self.store.functions[id] {
            ast::FunctionKind::Normal(function) => function,
            ast::FunctionKind::Signature(signature) => {
                return Err(Fault::error_with_kind(
                    MirErrorKind::SignatureOnlyFunctionHasNoBody,
                    Some(signature.span),
                ));
            }
        };

        let id = function.signature.value.id;
        let mir_function = self.function_lowerer.lower(function)?;
        self.functions.insert(id, mir_function);
        Ok(())
    }
}
