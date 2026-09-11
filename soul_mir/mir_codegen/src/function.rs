//! Per-function codegen: lowers a single MIR `Function`'s blocks/statements
//! into LLVM IR against an already-declared LLVM function value. Terminator
//! codegen lives in `terminator` and operand/rvalue-to-`BasicValueEnum`
//! codegen lives in `rvalue` instead — this file owns the per-block/
//! per-statement driver and place resolution.

use std::cell::Cell;

use ast_model::{ArrayKind, AstStore, SoulType};
use inkwell::{
    AddressSpace, IntPredicate,
    basic_block::BasicBlock as LlvmBlock,
    builder::Builder,
    types::{BasicTypeEnum, StructType},
    values::{FunctionValue, IntValue, PointerValue},
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
    types::{expect_int, is_signed, resolve_struct},
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
    /// an empty projection, then one step per projection element: a `GEP`
    /// through a struct field for `Field(index)`, or a load-then-`GEP`
    /// through a slice's data pointer for `Index(local)` (see
    /// `step_into_index`). Tracks the *Soul* type (not just the LLVM type)
    /// through the walk — unlike a struct's fields (queryable straight off
    /// its LLVM `StructType`), an opaque LLVM pointer carries no pointee-type
    /// info at all, so the element type after an `Index` step has to come
    /// from the Soul-level `ArrayType` instead. `Deref` isn't produced by any
    /// MIR lowering yet, so it still faults here.
    pub(crate) fn resolve_place(
        &self,
        place: &Place,
    ) -> CodegenResult<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> {
        let mut ptr = self.locals[place.local];
        let mut soul_ty = self.function.locals[place.local].ty.clone();

        for elem in &place.projection {
            soul_ty = match elem {
                PlaceElem::Field(index) => self.step_into_field(&mut ptr, &soul_ty, *index)?,
                PlaceElem::Index(index_local) => {
                    self.step_into_index(&mut ptr, &soul_ty, *index_local)?
                }
                PlaceElem::Deref => return Err(err(CodegenErrorKind::PlaceProjectionUnsupported)),
            };
        }

        let ty = self.ctx.llvm_type(self.soul_module, &soul_ty, None)?;
        Ok((ptr, ty))
    }

    /// One `Field(index)` step: `soul_ty` must resolve to a declared struct;
    /// GEPs `ptr` to that field's address and returns the field's own type.
    fn step_into_field(
        &self,
        ptr: &mut PointerValue<'ctx>,
        soul_ty: &SoulType,
        index: usize,
    ) -> CodegenResult<SoulType> {
        let struct_ = resolve_struct(self.ctx.declares, self.soul_module, soul_ty)
            .ok_or_else(|| err(CodegenErrorKind::PlaceProjectionUnsupported))?;
        let field_ty = struct_
            .fields
            .get(index)
            .and_then(|field| field.value.ty.clone())
            .ok_or_else(|| err(CodegenErrorKind::PlaceProjectionUnsupported))?;

        let BasicTypeEnum::StructType(struct_llvm_ty) =
            self.ctx.llvm_type(self.soul_module, soul_ty, None)?
        else {
            return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
        };
        *ptr = self
            .builder
            .build_struct_gep(struct_llvm_ty, *ptr, index as u32, "field_ptr")
            .map_err(llvm_err)?;

        Ok(field_ty)
    }

    /// One `Index(index_local)` step: `soul_ty` must be a slice
    /// (`[&]T`/`[&mut]T`). Loads the slice's data pointer out of its `ptr`
    /// field (field 0 of the `{ptr, len}` fat pointer built by `array_type`),
    /// loads the runtime index out of `index_local`, checks it against the
    /// slice's own `len` field (`build_bounds_check`), then GEPs the data
    /// pointer by that index (element-sized steps, since the GEP is typed as
    /// the element's own LLVM type).
    fn step_into_index(
        &self,
        ptr: &mut PointerValue<'ctx>,
        soul_ty: &SoulType,
        index_local: LocalId,
    ) -> CodegenResult<SoulType> {
        let SoulType::Array(array) = soul_ty else {
            return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
        };
        if !matches!(array.kind, ArrayKind::MutSlice | ArrayKind::ConstSlice) {
            return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
        }
        let element_ty = (*array.of_type).clone();

        let BasicTypeEnum::StructType(slice_llvm_ty) =
            self.ctx.llvm_type(self.soul_module, soul_ty, None)?
        else {
            return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
        };
        let ptr_field_addr = self
            .builder
            .build_struct_gep(slice_llvm_ty, *ptr, 0, "slice_ptr_addr")
            .map_err(llvm_err)?;
        let opaque_ptr_ty = self.ctx.context.ptr_type(AddressSpace::default());
        let data_ptr = self
            .builder
            .build_load(opaque_ptr_ty, ptr_field_addr, "slice_ptr")
            .map_err(llvm_err)?
            .into_pointer_value();

        let index_llvm_ty = self.local_type(index_local)?;
        let index_value = self
            .builder
            .build_load(index_llvm_ty, self.locals[index_local], "index")
            .map_err(llvm_err)?;
        let index_value = expect_int(index_value)?;
        let index_signed = self.local_is_signed_int(index_local)?;

        self.build_bounds_check(slice_llvm_ty, *ptr, index_value, index_signed)?;

        let element_llvm_ty = self.ctx.llvm_type(self.soul_module, &element_ty, None)?;
        *ptr = unsafe {
            self.builder
                .build_gep(element_llvm_ty, data_ptr, &[index_value], "elem_ptr")
                .map_err(llvm_err)?
        };

        Ok(element_ty)
    }

    /// Whether `local`'s declared type is a signed integer — used only to
    /// pick sign- vs zero-extension when widening an index to the slice
    /// `len` field's width in `build_bounds_check`. Non-primitive/non-int
    /// locals can't reach here (`expect_int` on the loaded value already
    /// faults first), so this only ever inspects `SoulType::Primitive`.
    fn local_is_signed_int(&self, local: LocalId) -> CodegenResult<bool> {
        match &self.function.locals[local].ty {
            SoulType::Primitive(prim) => Ok(is_signed(*prim)),
            _ => Ok(false),
        }
    }

    /// Traps via `abort` if `index` is out of bounds for the slice at
    /// `slice_ptr` (`index_llvm_ty`'s bit width may differ from the `len`
    /// field's pointer-width — a `Variable` index keeps its own declared
    /// type rather than being retyped, see `operand_local` in `mir_parser` —
    /// so `index` is first widened/narrowed to `len`'s width, sign-extending
    /// for a signed index and zero-extending otherwise, before an unsigned
    /// `<` compare; a negative signed index sign-extends to a huge unsigned
    /// value and is caught the same way as an over-long one). Splits the
    /// current block into a `bounds_fail` block that aborts and a
    /// `bounds_ok` continuation where the caller's own GEP/load/store keeps
    /// emitting — mirrors `codegen_assert`'s panic-block shape.
    fn build_bounds_check(
        &self,
        slice_llvm_ty: StructType<'ctx>,
        slice_ptr: PointerValue<'ctx>,
        index: IntValue<'ctx>,
        index_signed: bool,
    ) -> CodegenResult<()> {
        let len_field_addr = self
            .builder
            .build_struct_gep(slice_llvm_ty, slice_ptr, 1, "slice_len_addr")
            .map_err(llvm_err)?;
        let len_llvm_ty = slice_llvm_ty
            .get_field_type_at_index(1)
            .ok_or_else(|| err(CodegenErrorKind::PlaceProjectionUnsupported))?
            .into_int_type();
        let len_value = self
            .builder
            .build_load(len_llvm_ty, len_field_addr, "slice_len")
            .map_err(llvm_err)?
            .into_int_value();

        let index_bits = index.get_type().get_bit_width();
        let len_bits = len_llvm_ty.get_bit_width();
        let index = match index_bits.cmp(&len_bits) {
            std::cmp::Ordering::Less if index_signed => self
                .builder
                .build_int_s_extend(index, len_llvm_ty, "idx_sext")
                .map_err(llvm_err)?,
            std::cmp::Ordering::Less => self
                .builder
                .build_int_z_extend(index, len_llvm_ty, "idx_zext")
                .map_err(llvm_err)?,
            std::cmp::Ordering::Greater => self
                .builder
                .build_int_truncate(index, len_llvm_ty, "idx_trunc")
                .map_err(llvm_err)?,
            std::cmp::Ordering::Equal => index,
        };

        let in_bounds = self
            .builder
            .build_int_compare(IntPredicate::ULT, index, len_value, "bounds_ok")
            .map_err(llvm_err)?;

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| err(CodegenErrorKind::PlaceProjectionUnsupported))?;
        let fail_block = self
            .ctx
            .context
            .insert_basic_block_after(current_block, "bounds_fail");
        let ok_block = self
            .ctx
            .context
            .insert_basic_block_after(fail_block, "bounds_ok");

        self.builder
            .build_conditional_branch(in_bounds, ok_block, fail_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(fail_block);
        let abort_fn = self.abort_function();
        self.builder
            .build_call(abort_fn, &[], "abort_call")
            .map_err(llvm_err)?;
        self.builder.build_unreachable().map_err(llvm_err)?;

        self.builder.position_at_end(ok_block);
        Ok(())
    }
}

pub(crate) fn function_name(ast: &AstStore, id: FunctionId) -> CodegenResult<&str> {
    let kind = ast
        .functions
        .get(id)
        .ok_or_else(|| err(CodegenErrorKind::MissingAstEntry { id }))?;

    Ok(kind.signature().name.as_str())
}
