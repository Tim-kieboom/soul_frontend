//! Per-function codegen: lowers a single MIR `Function`'s blocks/statements
//! into LLVM IR against an already-declared LLVM function value. Terminator
//! codegen lives in `terminator` and operand/rvalue-to-`BasicValueEnum`
//! codegen lives in `rvalue` instead — this file owns the per-block/
//! per-statement driver and place resolution.

use std::cell::Cell;

use ast_model::AstStore;
use inkwell::{
    basic_block::BasicBlock as LlvmBlock,
    builder::Builder,
    types::BasicTypeEnum,
    values::{FunctionValue, PointerValue},
};
use mir_model::{BlockId, ExternFunction, Function, LocalId, Place, PlaceElem, Statement};
use soul_utils::{
    FunctionId,
    collections::vec_map::{VecMap, VecMapIndex},
    span::ModuleId,
};

use crate::{
    ctx::CodegenCtx,
    err,
    fault::{CodegenErrorKind, CodegenResult},
    llvm_err,
    module::ModuleCodegen,
};

pub(crate) struct FunctionCodegen<'ctx, 'a> {
    pub(crate) ctx: CodegenCtx<'ctx, 'a>,
    /// The Soul module this function was declared in — not to be confused
    /// with `ctx.module`, the LLVM `Module` being emitted into.
    pub(crate) soul_module: Option<ModuleId>,
    pub(crate) builder: Builder<'ctx>,
    pub(crate) function: &'a Function,
    pub(crate) is_entry_point: bool,
    pub(crate) locals: VecMap<LocalId, PointerValue<'ctx>>,
    pub(crate) blocks: VecMap<BlockId, LlvmBlock<'ctx>>,
    pub(crate) function_values: &'a VecMap<FunctionId, FunctionValue<'ctx>>,
    pub(crate) functions: &'a VecMap<FunctionId, Function>,
    pub(crate) externs: &'a VecMap<FunctionId, ExternFunction>,
    pub(crate) string_counter: &'a Cell<usize>,
}
impl<'ctx, 'a> ModuleCodegen<'ctx, 'a> {
    pub(crate) fn codegen_function(
        &mut self,
        id: FunctionId,
        function: &Function,
    ) -> CodegenResult<()> {
        let name = function_name(self.ast, id)?;
        let module = self.module_of(id);
        let fn_value = *self
            .function_values
            .get(id)
            .ok_or_else(|| err(CodegenErrorKind::MissingAstEntry { id }))?;

        let builder = self.ctx.context.create_builder();

        // A dedicated `entry` block holding only the parameter/local allocas,
        // ahead of the MIR blocks proper — keeps every alloca in the
        // function's first block (what LLVM's mem2reg pass expects) without
        // having to special-case MIR's own entry block for it.
        let entry = self.ctx.context.append_basic_block(fn_value, "entry");
        builder.position_at_end(entry);

        let mut locals: VecMap<LocalId, PointerValue<'ctx>> = VecMap::new();
        for (local_id, decl) in function.locals.entries() {
            let ty = self.ctx.llvm_type(module, &decl.ty, Some(decl.span))?;
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
                .ctx
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
            ctx: self.ctx,
            soul_module: module,
            function_values: &self.function_values,
            functions: &self.mir.functions,
            externs: &self.mir.externs,
            string_counter: &self.string_counter,
            builder,
            function,
            is_entry_point: name == "main",
            locals,
            blocks,
        };

        for (block_id, block) in function.blocks.entries() {
            fn_codegen.codegen_block(block_id, block)?;
        }

        Ok(())
    }
}

impl<'ctx, 'a> FunctionCodegen<'ctx, 'a> {
    pub(crate) fn codegen_block(
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
                let (ptr, ty) = self.resolve_place(place)?;
                let value = self.codegen_rvalue(rvalue, ty)?;
                self.builder.build_store(ptr, value).map_err(llvm_err)?;
                Ok(())
            }
            // Move/drop tracking has no runtime effect yet (see
            // `docs/mir-design.md`) — nothing to codegen for these until it does.
            Statement::MarkMoved(_) | Statement::SetDropFlag(..) | Statement::StorageDead(_) => {
                Ok(())
            }
        }
    }

    pub(crate) fn local_type(&self, local: LocalId) -> CodegenResult<BasicTypeEnum<'ctx>> {
        let decl = &self.function.locals[local];
        self.ctx
            .llvm_type(self.soul_module, &decl.ty, Some(decl.span))
    }

    /// Resolves a `Place` to the pointer it reads/writes through and the
    /// LLVM type at that location — the base local's own alloca and type for
    /// an empty projection, or (today) a single `GEP` step through a struct
    /// field for a `[Field(index)]` projection. `PlaceElem::Index`/`Deref`
    /// aren't produced by any MIR lowering yet, so they still fault here.
    pub(crate) fn resolve_place(
        &self,
        place: &Place,
    ) -> CodegenResult<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> {
        let mut ptr = self.locals[place.local];
        let mut ty = self.local_type(place.local)?;

        for elem in &place.projection {
            let PlaceElem::Field(index) = elem else {
                return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
            };
            let BasicTypeEnum::StructType(struct_ty) = ty else {
                return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
            };
            let field_ty = struct_ty
                .get_field_type_at_index(*index as u32)
                .ok_or(CodegenErrorKind::PlaceProjectionUnsupported)
                .map_err(err)?;

            ptr = self
                .builder
                .build_struct_gep(struct_ty, ptr, *index as u32, "field_ptr")
                .map_err(llvm_err)?;
            ty = field_ty;
        }

        Ok((ptr, ty))
    }
}

pub(crate) fn function_name(ast: &AstStore, id: FunctionId) -> CodegenResult<&str> {
    let kind = ast
        .functions
        .get(id)
        .ok_or_else(|| err(CodegenErrorKind::MissingAstEntry { id }))?;

    Ok(kind.signature().name.as_str())
}
