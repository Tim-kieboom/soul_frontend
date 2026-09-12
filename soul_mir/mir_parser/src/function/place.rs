use crate::{
    fault::{MirErrorKind, MirResult},
    function::FunctionLowerer,
};
use ast_model::{self as ast, SoulType, operators::BinaryOperatorKind};
use mir_model as mir;
use soul_utils::{
    TypeModifier, compiler_options::MirOptions, fault::Fault, soul_names::PrimitiveTypes,
    span::Span,
};

impl<'a> FunctionLowerer<'a> {
    /// Lowers `object.field` as a read — see `resolve_field_place`.
    pub(super) fn lower_field_access(
        &mut self,
        field_access: &ast::FieldAccess,
        span: Span,
    ) -> MirResult<mir::Operand> {
        Ok(mir::Operand::Copy(
            self.resolve_field_place(field_access, span)?.0,
        ))
    }

    /// Lowers `collection[index]` as a read — see `resolve_index_place`.
    pub(super) fn lower_index_access(
        &mut self,
        index: &ast::Index,
        span: Span,
    ) -> MirResult<mir::Operand> {
        Ok(mir::Operand::Copy(self.resolve_index_place(index, span)?.0))
    }

    /// Resolves an arbitrary "place expression" — a variable, a field access
    /// (`object.field`), an index (`collection[i]`), or any nesting of those
    /// — into a `Place` plus its resolved type. The shared entry point
    /// `resolve_field_place`'s object, `resolve_index_place`'s collection,
    /// and `lower_ref`'s referenced value all recurse through, so
    /// `o.items[i].x` ends up as one `Place` with a three-element
    /// projection, not a chain of temporaries. Anything else (a
    /// call/constructor result, ...) still faults — those aren't places
    /// this slice can project through.
    pub(super) fn resolve_place_expression(
        &mut self,
        expr_id: ast::ExpressionId,
        span: Span,
    ) -> MirResult<(mir::Place, SoulType)> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ast::ExpressionKind::Variable(var) => {
                let local = self.resolve_local(var, expr.span)?;
                Ok((mir::Place::local(local), self.locals[local].ty.clone()))
            }
            ast::ExpressionKind::FieldAccess(field_access) => {
                self.resolve_field_place(field_access, expr.span)
            }
            ast::ExpressionKind::Index(index) => self.resolve_index_place(index, expr.span),
            _ => Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedPlaceExpression,
                Some(span),
            )),
        }
    }

    /// Lowers `&value`/`@value`. For a fixed-size array (`[N]T`) place, this
    /// produces a slice instead of a plain pointer: a bare pointer to an
    /// array carries no length, so — decided up front so a later bounds
    /// check can land without an ABI break — referencing an array always
    /// builds the `{ptr, len}` fat pointer (`AggregateKind::Array`, `len` a
    /// compile-time constant since only fixed-size arrays are supported
    /// here) rather than a plain `Rvalue::Ref`. Anything else (an ordinary
    /// `&x`) still lowers to a plain `Rvalue::Ref` — the bifurcation is
    /// entirely at the MIR-lowering level, `Ref` itself stays bare-pointer
    /// only.
    pub(super) fn lower_ref(&mut self, ref_: &ast::Ref, span: Span) -> MirResult<mir::Rvalue> {
        let (place, value_ty) = self.resolve_place_expression(ref_.value, span)?;
        let mutable = ref_.is_mutable;

        let SoulType::Array(array) = &value_ty else {
            return Ok(mir::Rvalue::Ref { mutable, place });
        };
        let ast::ArrayKind::StackArray(len) = array.kind else {
            return Err(Fault::error_with_kind(
                MirErrorKind::ArrayReferenceUnsupported {
                    ty: format!("{value_ty:?}").into(),
                },
                Some(span),
            ));
        };
        let element_ty = (*array.of_type).clone();

        let ptr_ty = SoulType::Reference(ast::ReferenceType {
            inner: Box::new(element_ty),
            lifetime: None,
            mutable: if mutable {
                soul_utils::Mutable::Mut
            } else {
                soul_utils::Mutable::Immut
            },
        });
        let ptr_temp = self.alloc_local(ptr_ty, TypeModifier::Immut, span);
        self.statements.push(mir::Statement::Assign(
            mir::Place::local(ptr_temp),
            mir::Rvalue::Ref { mutable, place },
        ));

        Ok(mir::Rvalue::Aggregate(
            mir::AggregateKind::Array,
            vec![
                mir::Operand::Copy(mir::Place::local(ptr_temp)),
                mir::Operand::Constant(mir::ConstValue::Uint(len as u128)),
            ],
        ))
    }

    /// Lowers `object.field` into a `Place` with a `Field` projection
    /// appended onto the object's own place — read or write, straight off
    /// whatever storage the struct value already lives in (no temp/copy).
    /// Also returns the resolved place's own type (the innermost field's
    /// declared type), since a recursive caller needs it to resolve the
    /// *next* struct.
    fn resolve_field_place(
        &mut self,
        field_access: &ast::FieldAccess,
        span: Span,
    ) -> MirResult<(mir::Place, SoulType)> {
        let object_span = self.store.expressions[field_access.object].span;
        let (mut place, object_ty) =
            self.resolve_place_expression(field_access.object, object_span)?;

        let struct_ = self.resolve_struct(&object_ty).ok_or_else(|| {
            Fault::error_with_kind(
                MirErrorKind::NonPrimitiveType {
                    ty: format!("{object_ty:?}").into(),
                },
                Some(span),
            )
        })?;

        let field_name = field_access.field.as_str();
        let (index, field_ty) = struct_
            .fields
            .iter()
            .enumerate()
            .find_map(|(index, field)| {
                let is_match = matches!(&field.value.pattern, ast::VarPattern::Simple { binding, .. } if binding.ident.as_str() == field_name);
                is_match.then(|| (index, field.value.ty.clone()))
            })
            .ok_or_else(|| {
                Fault::error_with_kind(
                    MirErrorKind::StructFieldNotFound {
                        struct_name: struct_.name.as_str().into(),
                        field: field_name.into(),
                    },
                    Some(span),
                )
            })?;

        let field_ty = field_ty.ok_or_else(|| {
            Fault::error_with_kind(MirErrorKind::VariableHasNoResolvedType, Some(span))
        })?;

        place.projection.push(mir::PlaceElem::Field(index));
        Ok((place, field_ty))
    }

    /// Lowers `collection[index]` into a `Place` with an `Index` projection
    /// appended onto the collection's own place. Only a slice
    /// (`[&]T`/`[&mut]T`) collection is supported — indexing directly into a
    /// fixed-size array/wildcard/heap array isn't (per the M1 scope: those
    /// only ever get *referenced* into a slice first, see `lower_ref`).
    fn resolve_index_place(
        &mut self,
        index: &ast::Index,
        span: Span,
    ) -> MirResult<(mir::Place, SoulType)> {
        let collection_span = self.store.expressions[index.collection].span;
        let (mut place, collection_ty) =
            self.resolve_place_expression(index.collection, collection_span)?;

        let SoulType::Array(array) = &collection_ty else {
            return Err(Fault::error_with_kind(
                MirErrorKind::IndexTargetNotASlice {
                    ty: format!("{collection_ty:?}").into(),
                },
                Some(span),
            ));
        };
        if !matches!(
            array.kind,
            ast::ArrayKind::MutSlice | ast::ArrayKind::ConstSlice
        ) {
            return Err(Fault::error_with_kind(
                MirErrorKind::IndexTargetNotASlice {
                    ty: format!("{collection_ty:?}").into(),
                },
                Some(span),
            ));
        }
        let element_ty = (*array.of_type).clone();

        // Indices are always non-negative offsets in this slice — always
        // materialize into a `uint` temp rather than trying to preserve
        // whatever concrete int type the index expression happened to have
        // (matching the existing "untyped int literal defaults to `uint`"
        // convention elsewhere in this lowerer).
        let index_local =
            self.operand_local(index.index, SoulType::Primitive(PrimitiveTypes::Uint), span)?;

        if self
            .options
            .mir
            .contains(MirOptions::CHECK_INDEX_OUT_OF_BOUNDS)
        {
            self.emit_bounds_check(&place, index_local, span);
        }

        place.projection.push(mir::PlaceElem::Index(index_local));
        Ok((place, element_ty))
    }

    /// `assert(index < collection.len())` — mirrors rustc's own `Len` +
    /// comparison + `Assert` shape: bounds checking is an ordinary MIR
    /// terminator here, not a codegen-level "insert a panicking branch
    /// here" mechanism (`mir_codegen` only has to implement `Rvalue::Len`
    /// and the already-generic `Assert` terminator). `index_local` is cast
    /// to `uint` first if it isn't already one — `operand_local` reuses a
    /// bare-`Variable` index's own declared type as-is (see its docs), but
    /// `Len` is always `uint`-typed and the comparison needs matching
    /// widths, so a non-`uint` index goes through `Rvalue::Cast` first.
    fn emit_bounds_check(
        &mut self,
        collection: &mir::Place,
        index_local: mir::LocalId,
        span: Span,
    ) {
        const UINT: SoulType = SoulType::Primitive(PrimitiveTypes::Uint);

        let len_local = self.alloc_local(UINT, TypeModifier::Immut, span);
        self.statements.push(mir::Statement::Assign(
            mir::Place::local(len_local),
            mir::Rvalue::Len(collection.clone()),
        ));

        let index_local = if self.locals[index_local]
            .ty
            .is_primitive_kind(PrimitiveTypes::Uint)
        {
            index_local
        } else {
            let cast = self.alloc_local(UINT, TypeModifier::Immut, span);
            self.statements.push(mir::Statement::Assign(
                mir::Place::local(cast),
                mir::Rvalue::Cast(mir::Operand::Copy(mir::Place::local(index_local)), UINT),
            ));
            cast
        };

        let cond_local = self.alloc_local(
            SoulType::Primitive(PrimitiveTypes::Boolean),
            TypeModifier::Immut,
            span,
        );
        self.statements.push(mir::Statement::Assign(
            mir::Place::local(cond_local),
            mir::Rvalue::BinaryOp(
                BinaryOperatorKind::Lt,
                mir::Operand::Copy(mir::Place::local(index_local)),
                mir::Operand::Copy(mir::Place::local(len_local)),
            ),
        ));

        let msg = mir::Operand::Constant(mir::ConstValue::Str("index out of bounds".to_string()));
        let next = self.new_block();
        self.seal(
            mir::Terminator::Assert {
                cond: mir::Operand::Copy(mir::Place::local(cond_local)),
                expected: true,
                msg,
                target: next,
                span,
            },
            Some(next),
        );
    }

    /// Materializes an expression into a `LocalId` holding its value —
    /// `PlaceElem::Index` needs an actual local to reference, not an
    /// arbitrary `Operand`. Reuses an already-existing local as-is when the
    /// expression is just a bare variable (no extra temp/copy, and no risk
    /// of a width mismatch from re-typing it as `ty`); otherwise allocates a
    /// fresh temp of type `ty` and assigns into it.
    fn operand_local(
        &mut self,
        expr_id: ast::ExpressionId,
        ty: SoulType,
        span: Span,
    ) -> MirResult<mir::LocalId> {
        let expr = &self.store.expressions[expr_id];
        if let ast::ExpressionKind::Variable(var) = &expr.node {
            return self.resolve_local(var, expr.span);
        }

        let rvalue = self.lower_rvalue(expr_id)?;
        let temp = self.alloc_local(ty, TypeModifier::Immut, span);
        self.statements
            .push(mir::Statement::Assign(mir::Place::local(temp), rvalue));
        Ok(temp)
    }
}
