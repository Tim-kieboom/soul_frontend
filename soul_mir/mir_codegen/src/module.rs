//! Module-level codegen: declares every function/extern's LLVM signature up
//! front, then hands each real function's body off to `FunctionCodegen`.

use std::cell::Cell;

use ast_model::{AstStore, declare_store::DeclareStore};
use inkwell::{
    context::Context,
    module::{Linkage, Module},
    types::BasicType,
    values::FunctionValue,
};
use mir_model::MirProgram;
use soul_utils::{
    FunctionId,
    collections::vec_map::VecMap,
    compiler_options::{CompilerOptions, PlatformInfo},
    span::ModuleId,
};

use crate::{fault::CodegenResult, function::function_name, types::param_metadata};

/// Builds an LLVM module containing every function/extern declaration in
/// `mir`. `ast` is only used to recover each function's source name (MIR
/// itself only has `FunctionId`s) — the entry point a linker looks for
/// (`main`) has to match the Soul function's actual declared name exactly.
pub(crate) fn codegen_module<'ctx>(
    context: &'ctx Context,
    module_name: &str,
    mir: &MirProgram,
    ast: &AstStore,
    declares: &DeclareStore,
    options: &CompilerOptions,
) -> CodegenResult<Module<'ctx>> {
    let module = context.create_module(module_name);
    let mut codegen = ModuleCodegen {
        mir,
        ast,
        context,
        declares,
        module: &module,
        platform: options.platform,
        string_counter: Cell::new(0),
        function_values: VecMap::new(),
    };

    codegen.declare_all_functions()?;
    for (id, function) in mir.functions.entries() {
        codegen.codegen_function(id, function)?;
    }

    Ok(module)
}

pub(crate) struct ModuleCodegen<'ctx, 'a> {
    pub ast: &'a AstStore,
    pub mir: &'a MirProgram,
    pub declares: &'a DeclareStore,
    pub context: &'ctx Context,
    pub module: &'a Module<'ctx>,
    pub platform: PlatformInfo,
    pub function_values: VecMap<FunctionId, FunctionValue<'ctx>>,
    /// Shared across every function's codegen so string-literal globals get
    /// module-wide-unique names, not per-function-restarting ones.
    pub string_counter: Cell<usize>,
}

impl<'ctx, 'a> ModuleCodegen<'ctx, 'a> {
    /// The module a function/extern was declared in, needed to resolve a
    /// struct-typed `SoulType::Stub`'s bare name back to its declaration.
    pub(crate) fn module_of(&self, id: FunctionId) -> Option<ModuleId> {
        self.declares.get_function(id).map(|(_, module)| *module)
    }

    /// Declares every function's signature up front, so calls can reference a
    /// callee regardless of definition order (including mutual recursion) —
    /// and declares every `extern "C"` function as a bodyless external
    /// declaration (never passed to `codegen_function`, so no blocks are
    /// ever appended to it — that omission alone is what makes it a true
    /// external declaration rather than a defined-but-empty function).
    fn declare_all_functions(&mut self) -> CodegenResult<()> {
        for (id, function) in self.mir.functions.entries() {
            let name = function_name(self.ast, id)?;
            let module = self.module_of(id);

            let param_types = function
                .locals
                .entries()
                .take(function.arg_count)
                .map(|(_, decl)| self.llvm_type(module, &decl.ty, Some(decl.span)))
                .collect::<CodegenResult<Vec<_>>>()?;

            let param_metadata_types = param_metadata(&param_types);

            let return_type = function
                .return_local
                .map(|local| {
                    let decl = &function.locals[local];
                    self.llvm_type(module, &decl.ty, Some(decl.span))
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
            let module = self.module_of(id);

            let param_types = extern_fn
                .params
                .iter()
                .map(|ty| self.llvm_type(module, ty, None))
                .collect::<CodegenResult<Vec<_>>>()?;
            let param_metadata_types = param_metadata(&param_types);

            let fn_type = match &extern_fn.return_type {
                Some(ty) => self
                    .llvm_type(module, ty, None)?
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
}
