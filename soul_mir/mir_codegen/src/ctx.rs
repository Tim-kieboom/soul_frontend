//! The small, unchanging-per-module context every codegen struct needs: the
//! LLVM context/module, the resolved declarations, and the target
//! platform's int widths. Every field is a reference or a small `Copy`
//! struct, so `CodegenCtx` itself is `Copy` — `ModuleCodegen` and
//! `FunctionCodegen` each just hold their own copy of it instead of one
//! borrowing pieces off the other (which is what `FunctionCodegen` used to
//! do field-by-field, and what motivated giving both structs a matching
//! `llvm_type` wrapper method in the first place).

use ast_model::{SoulType, declare_store::DeclareStore};
use inkwell::{context::Context, module::Module, types::BasicTypeEnum};
use soul_utils::{
    collections::module_store::ModuleStore,
    compiler_options::PlatformInfo,
    span::{ModuleId, Span},
};

use crate::{fault::CodegenResult, types::llvm_type};

#[derive(Clone, Copy)]
pub(crate) struct CodegenCtx<'ctx, 'a> {
    pub(crate) context: &'ctx Context,
    pub(crate) module: &'a Module<'ctx>,
    pub(crate) declares: &'a DeclareStore,
    /// Resolves a `Span`'s `ModuleId` back to the source file it came from —
    /// needed only to format a panic's `"file:line:col"` location
    /// (`terminator::codegen_assert`), nothing else in codegen touches it.
    pub(crate) modules: &'a ModuleStore,
    pub(crate) platform: PlatformInfo,
}

impl<'ctx, 'a> CodegenCtx<'ctx, 'a> {
    pub(crate) fn llvm_type(
        &self,
        module: Option<ModuleId>,
        ty: &SoulType,
        span: Option<Span>,
    ) -> CodegenResult<BasicTypeEnum<'ctx>> {
        llvm_type(
            self.context,
            &self.platform,
            self.declares,
            module,
            ty,
            span,
        )
    }
}
