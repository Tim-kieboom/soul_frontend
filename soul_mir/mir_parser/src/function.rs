use ast_model::{
    self as ast, SoulType,
    declare_store::DeclareStore,
    operators::{BinaryOperatorKind, UnaryOperatorKind},
};
use mir_model as mir;
use soul_utils::{
    TypeModifier,
    collections::vec_map::VecMap,
    fault::Fault,
    ids::IdGenerator,
    intrinsics::IntrinsicFunction,
    soul_names::PrimitiveTypes,
    span::{ModuleId, Span},
};

use crate::fault::{MirErrorKind, MirResult};

/// The `(header, exit)` block pair of an enclosing loop, so a nested `break`/
/// `continue` can find its target regardless of how deep inside `if`s it is.
/// Soul has no labeled break/continue, so this is a plain stack: `break`/
/// `continue` always target the innermost entry.
struct LoopTargets {
    header: mir::BlockId,
    exit: mir::BlockId,
}

pub struct FunctionLowerer<'a> {
    store: &'a ast::AstStore,
    declares: &'a DeclareStore,
    /// The module the function currently being lowered was declared in — set
    /// at the start of each `lower()` call, needed to resolve a struct-typed
    /// `SoulType::Stub`'s bare name back to its declaration (struct names are
    /// only unique per module).
    module: Option<ModuleId>,
    local_alloc: IdGenerator<mir::LocalId>,
    node_to_local: VecMap<ast::NodeId, mir::LocalId>,
    locals: VecMap<mir::LocalId, mir::LocalDecl>,

    block_alloc: IdGenerator<mir::BlockId>,
    blocks: VecMap<mir::BlockId, mir::BasicBlock>,

    current: Option<mir::BlockId>,
    statements: Vec<mir::Statement>,
    loops: Vec<LoopTargets>,
}
impl<'a> FunctionLowerer<'a> {
    pub(crate) fn new(store: &'a ast::AstStore, declares: &'a DeclareStore) -> Self {
        Self {
            store,
            declares,
            module: None,
            current: None,
            loops: vec![],
            statements: vec![],
            locals: VecMap::new(),
            blocks: VecMap::new(),
            node_to_local: VecMap::new(),
            local_alloc: IdGenerator::new(),
            block_alloc: IdGenerator::new(),
        }
    }

    pub(crate) fn lower(&mut self, function: &ast::Function) -> MirResult<mir::Function> {
        self.reset();

        let signature = &function.signature.value;
        let fn_span = function.signature.span;
        self.module = self
            .declares
            .get_function(signature.id)
            .map(|(_, module)| *module);

        for parameter in &signature.parameters {
            let span = parameter.name.span();
            self.require_lowerable(&parameter.ty, span)?;
            let modifier = parameter.mutable.to_type_modifier();
            let local = self.alloc_local(parameter.ty.clone(), modifier, span);
            self.node_to_local.insert(parameter.id, local);
        }

        let arg_count = signature.parameters.len();
        // A `none`(void)-returning function has nothing to hold a return value
        // in, so it gets no `return_local` at all — see `Function::return_local`.
        let is_none_return = matches!(signature.return_type, SoulType::None);
        let return_local = if is_none_return {
            None
        } else {
            self.require_lowerable(&signature.return_type, signature.name.span())?;
            Some(self.alloc_local(
                signature.return_type.clone(),
                TypeModifier::Mut,
                signature.name.span(),
            ))
        };

        let entry = self.new_block();
        self.current = Some(entry);

        let statement_ids = self.store.blocks[function.block].statements.clone();
        self.lower_body(return_local, &statement_ids)?;

        if self.current.is_some() {
            if is_none_return {
                // Falling off the end of a `none`-returning function is valid
                // (an implicit `return`) — unlike every other return type,
                // where it's `MissingReturnStatement`.
                self.seal(mir::Terminator::Return, None);
            } else {
                return Err(Fault::error_with_kind(
                    MirErrorKind::MissingReturnStatement,
                    Some(fn_span),
                ));
            }
        }

        Ok(mir::Function {
            id: signature.id,
            locals: std::mem::take(&mut self.locals),
            blocks: std::mem::take(&mut self.blocks),
            arg_count,
            return_local,
        })
    }

    fn reset(&mut self) {
        self.loops.clear();
        self.current = None;
        self.module = None;
        self.blocks.clear();
        self.locals.clear();
        self.statements.clear();
        self.node_to_local.clear();
        self.local_alloc = IdGenerator::new();
        self.block_alloc = IdGenerator::new();
    }

    /// Accepts primitives and structs whose name resolves to a declaration in
    /// this function's module — the boundary this lowering slice actually
    /// knows how to turn into MIR locals/places. Everything else (arrays,
    /// references, generics, an undeclared/unresolvable name) still faults,
    /// same as before struct support existed.
    fn require_lowerable(&self, ty: &SoulType, span: Span) -> MirResult<()> {
        if matches!(ty, SoulType::Primitive(_)) || self.resolve_struct(ty).is_some() {
            return Ok(());
        }
        Err(Fault::error_with_kind(
            MirErrorKind::NonPrimitiveType {
                ty: format!("{ty:?}").into(),
            },
            Some(span),
        ))
    }

    /// Resolves a struct-typed `SoulType::Stub`'s bare name back to its
    /// `Struct` declaration in this function's module. `None` for anything
    /// that isn't a `Stub`, or a `Stub` that doesn't name a struct in scope
    /// (an enum/trait, a generic, or an unresolved name).
    fn resolve_struct(&self, ty: &SoulType) -> Option<&ast::Struct> {
        let SoulType::Stub(stub) = ty else {
            return None;
        };
        self.declares.get_struct_by_name(&stub.name, self.module?)
    }

    fn new_block(&mut self) -> mir::BlockId {
        self.block_alloc.alloc()
    }

    fn is_terminated(&self) -> bool {
        self.current.is_none()
    }

    fn seal(&mut self, terminator: mir::Terminator, next: Option<mir::BlockId>) {
        if let Some(current) = self.current.take() {
            let statements = std::mem::take(&mut self.statements);
            self.blocks.insert(
                current,
                mir::BasicBlock {
                    terminator,
                    statements,
                },
            );
        }
        self.current = next;
    }

    fn lower_body(
        &mut self,
        return_local: Option<mir::LocalId>,
        statements: &[ast::StatementId],
    ) -> MirResult<()> {
        for &id in statements {
            let statement = &self.store.statements[id];
            if self.is_terminated() {
                return Err(Fault::error_with_kind(
                    MirErrorKind::UnreachableStatement,
                    Some(statement.span),
                ));
            }
            self.lower_statement(return_local, statement)?;
        }
        Ok(())
    }

    fn lower_statement(
        &mut self,
        return_local: Option<mir::LocalId>,
        statement: &ast::Statement,
    ) -> MirResult<()> {
        match &statement.node {
            ast::StatementKind::Variable(variable) => self.lower_variable(statement, variable),
            ast::StatementKind::Assignment(assignment) => self.lower_assignment(assignment),
            ast::StatementKind::Expression { expression, .. } => {
                self.lower_expression_statement(return_local, statement, *expression)
            }
            _ => Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedStatementKind,
                Some(statement.span),
            )),
        }
    }

    /// Lowers `left = right` (compound assignments like `n -= 1` are already
    /// desugared by the parser into `left = left - 1` before this ever runs,
    /// so `lower_rvalue` handles the right-hand side with no special-casing).
    /// `left` is a bare, already-declared variable, or a struct field write
    /// (`p.x = 1`, via the same `Field`-projection machinery as a field
    /// read) — `arr[i]`/`*p` as an assignment target still faults, since
    /// nothing lowers those places yet.
    fn lower_assignment(&mut self, assignment: &ast::Assignment) -> MirResult<()> {
        let left = &self.store.expressions[assignment.left];
        let place = match &left.node {
            ast::ExpressionKind::Variable(var) => {
                mir::Place::local(self.resolve_local(var, left.span)?)
            }
            ast::ExpressionKind::FieldAccess(field_access) => {
                self.resolve_field_place(field_access, left.span)?.0
            }
            _ => {
                return Err(Fault::error_with_kind(
                    MirErrorKind::AssignmentTargetUnsupported,
                    Some(left.span),
                ));
            }
        };

        let rvalue = self.lower_rvalue(assignment.right)?;
        self.statements.push(mir::Statement::Assign(place, rvalue));
        Ok(())
    }

    fn resolve_local(&self, var: &ast::VariableExpression, span: Span) -> MirResult<mir::LocalId> {
        let Some(resolved) = self.declares.get_variable_resolve(var.id) else {
            return Err(Fault::error_with_kind(
                MirErrorKind::VariableHasNoResolvedBinding,
                Some(span),
            ));
        };

        let Some(local) = self.node_to_local.get(resolved) else {
            return Err(Fault::error_with_kind(
                MirErrorKind::VariableNotBoundToLocal,
                Some(span),
            ));
        };

        Ok(*local)
    }

    /// Lowers `object.field` into a `Place` with a `Field` projection
    /// appended onto the object's own place — read or write, straight off
    /// whatever storage the struct value already lives in (no temp/copy).
    /// `object` is either a bare variable, or itself a field access
    /// (`o.inner.x`), resolved recursively — one `Field` projection gets
    /// pushed per `.field` step, so `o.inner.x` ends up as a single `Place`
    /// with a two-element projection, not a chain of temporaries. Also
    /// returns the resolved place's own type (the innermost field's declared
    /// type), since a recursive caller needs it to resolve the *next* struct.
    /// Anything else as the object (a call/constructor result, an index
    /// expression, ...) still faults — those aren't places this slice can
    /// project through.
    fn resolve_field_place(
        &self,
        field_access: &ast::FieldAccess,
        span: Span,
    ) -> MirResult<(mir::Place, SoulType)> {
        let object = &self.store.expressions[field_access.object];
        let (mut place, object_ty) = match &object.node {
            ast::ExpressionKind::Variable(var) => {
                let local = self.resolve_local(var, object.span)?;
                (mir::Place::local(local), self.locals[local].ty.clone())
            }
            ast::ExpressionKind::FieldAccess(inner) => {
                self.resolve_field_place(inner, object.span)?
            }
            _ => {
                return Err(Fault::error_with_kind(
                    MirErrorKind::UnsupportedFieldAccessObject,
                    Some(span),
                ));
            }
        };

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

    /// Lowers `object.field` as a read — see `resolve_field_place`.
    fn lower_field_access(
        &self,
        field_access: &ast::FieldAccess,
        span: Span,
    ) -> MirResult<mir::Operand> {
        Ok(mir::Operand::Copy(
            self.resolve_field_place(field_access, span)?.0,
        ))
    }

    /// The one argument an `assert`/`panic` intrinsic call takes, guarded
    /// against arity mismatches — the resolver logs a fault on a wrong count
    /// but still stores the resolution and lets the call through, so this
    /// must not assume `call.arguments` has the expected length.
    fn intrinsic_sole_argument(
        &self,
        call: &ast::FunctionCall,
        kind: IntrinsicFunction,
        span: Span,
    ) -> MirResult<ast::ExpressionId> {
        match call.arguments.as_slice() {
            [argument] => Ok(argument.value),
            _ => Err(Fault::error_with_kind(
                MirErrorKind::IntrinsicArityMismatch {
                    name: kind.as_str().into(),
                    expected: kind.arity(),
                    got: call.arguments.len(),
                },
                Some(span),
            )),
        }
    }

    /// Lowers `assert(cond)`: continues normally if `cond` is `true`,
    /// otherwise panics. `cond` goes through the same `lower_bool_condition`
    /// machinery as an `if`/`while` condition — same restrictions apply.
    fn lower_assert_intrinsic(&mut self, call: &ast::FunctionCall, span: Span) -> MirResult<()> {
        let cond_expr = self.intrinsic_sole_argument(call, IntrinsicFunction::Assert, span)?;
        let cond = self.lower_bool_condition(cond_expr)?;
        let msg = mir::Operand::Constant(mir::ConstValue::Str("assertion failed".to_string()));

        let next = self.new_block();
        self.seal(
            mir::Terminator::Assert {
                cond,
                expected: true,
                msg,
                target: next,
            },
            Some(next),
        );
        Ok(())
    }

    /// Lowers `panic(msg)`: unconditionally diverges. Modeled as an `Assert`
    /// that's always false against `expected: true`, so it always takes the
    /// panic path — `target` is allocated (the shape needs a `BlockId`) but
    /// genuinely unreachable, so no block is ever inserted for it, and the
    /// cursor becomes unreachable afterward (`next: None`), same as `return`.
    fn lower_panic_intrinsic(&mut self, call: &ast::FunctionCall, span: Span) -> MirResult<()> {
        let msg_expr = self.intrinsic_sole_argument(call, IntrinsicFunction::Panic, span)?;
        let msg = self.lower_operand(msg_expr)?;

        let dead = self.new_block();
        self.seal(
            mir::Terminator::Assert {
                cond: mir::Operand::Constant(mir::ConstValue::Bool(false)),
                expected: true,
                msg,
                target: dead,
            },
            None,
        );
        Ok(())
    }

    /// Dispatches an intrinsic call reached in statement position. `Ok(None)`
    /// means `call` isn't an intrinsic at all (no `IntrinsicResolve` stored
    /// for it) — the caller should fall through to the ordinary
    /// `FunctionResolve`-based `lower_call` path instead.
    fn try_lower_intrinsic_statement(
        &mut self,
        call: &ast::FunctionCall,
        span: Span,
    ) -> MirResult<Option<()>> {
        let Some(resolve) = self.declares.get_intrinsic_resolve(call.id) else {
            return Ok(None);
        };
        match resolve.kind {
            IntrinsicFunction::Assert => self.lower_assert_intrinsic(call, span)?,
            IntrinsicFunction::Panic => self.lower_panic_intrinsic(call, span)?,
            other => {
                return Err(Fault::error_with_kind(
                    MirErrorKind::UnsupportedIntrinsic {
                        name: other.as_str().into(),
                    },
                    Some(span),
                ));
            }
        }
        Ok(Some(()))
    }

    /// Lowers a free-function call `name(args...)`. Only the plain shape is
    /// supported this slice: no method-call callee, no generics, no named
    /// arguments, no `defer f()` — each of those gets `UnsupportedCallShape`.
    ///
    /// A call is a *terminator* (`mir_model::Terminator::Call`), not a plain
    /// statement, because it can diverge — so this seals the current block
    /// with the call and opens a fresh one as the continuation, returning the
    /// operand that reads the result (if any) out of that new block. Every
    /// caller of this (an operand deep inside `a + f(b)`, a loop condition,
    /// etc.) only ever looks at the *returned* operand, never assumes which
    /// block is current afterward, so this composes with the rest of the
    /// lowerer for free — nothing needs to change at the call sites.
    fn lower_call(
        &mut self,
        call: &ast::FunctionCall,
        span: Span,
        want_result: bool,
    ) -> MirResult<Option<mir::Operand>> {
        if call.callee.is_some() || !call.generics.is_empty() {
            return Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedCallShape,
                Some(span),
            ));
        }

        let Some(resolve) = self.declares.get_function_resolve(call.id) else {
            return Err(Fault::error_with_kind(
                MirErrorKind::FunctionCallHasNoResolvedTarget,
                Some(span),
            ));
        };
        if resolve.is_defer {
            return Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedCallShape,
                Some(span),
            ));
        }

        let Some((signature, _)) = self.declares.get_function(resolve.id) else {
            return Err(Fault::error_with_kind(
                MirErrorKind::FunctionCallHasNoResolvedTarget,
                Some(span),
            ));
        };
        let return_type = signature.return_type.clone();

        let mut args = Vec::with_capacity(call.arguments.len());
        for argument in &call.arguments {
            if argument.name.is_some() {
                return Err(Fault::error_with_kind(
                    MirErrorKind::UnsupportedCallShape,
                    Some(span),
                ));
            }
            args.push(self.lower_operand(argument.value)?);
        }

        let is_none_return = matches!(return_type, SoulType::None);
        let destination_local = if want_result && !is_none_return {
            self.require_lowerable(&return_type, span)?;
            Some(self.alloc_local(return_type, TypeModifier::Immut, span))
        } else {
            None
        };
        let destination = destination_local.map(mir::Place::local);

        let next = self.new_block();
        self.seal(
            mir::Terminator::Call {
                id: resolve.id,
                arguments: args,
                destination,
                target: Some(next),
            },
            Some(next),
        );

        Ok(destination_local.map(|local| mir::Operand::Copy(mir::Place::local(local))))
    }

    fn lower_expression_statement(
        &mut self,
        return_local: Option<mir::LocalId>,
        stmt: &ast::Statement,
        expression: ast::ExpressionId,
    ) -> MirResult<()> {
        let expr = &self.store.expressions[expression];
        match &expr.node {
            ast::ExpressionKind::Return(Some(value_id)) => {
                // A `return <expr>` in a `none`-returning function would mean
                // the resolver let a value flow into a `none` context, which
                // it type-checks against — trust that and treat this as
                // defensive, not user-reachable.
                let Some(return_local) = return_local else {
                    return Err(Fault::error_with_kind(
                        MirErrorKind::UnexpectedReturnValue,
                        Some(expr.span),
                    ));
                };
                let rvalue = self.lower_rvalue(*value_id)?;
                self.statements.push(mir::Statement::Assign(
                    mir::Place::local(return_local),
                    rvalue,
                ));
                self.seal(mir::Terminator::Return, None);
                Ok(())
            }
            ast::ExpressionKind::Return(None) => {
                self.seal(mir::Terminator::Return, None);
                Ok(())
            }
            ast::ExpressionKind::If(if_expr) => self.lower_if(return_local, if_expr, expr.span),
            ast::ExpressionKind::For(for_expr) => self.lower_for(return_local, for_expr, expr.span),
            ast::ExpressionKind::Break => self.lower_break(expr.span),
            ast::ExpressionKind::Continue => self.lower_continue(expr.span),
            ast::ExpressionKind::FunctionCall(call) => {
                if self
                    .try_lower_intrinsic_statement(call, expr.span)?
                    .is_some()
                {
                    return Ok(());
                }
                const WANTS_NO_RESULT: bool = false;
                self.lower_call(call, expr.span, WANTS_NO_RESULT)?;
                Ok(())
            }
            _ => Err(Fault::error_with_kind(
                MirErrorKind::NonReturnTerminalStatementUnsupported,
                Some(stmt.span),
            )),
        }
    }

    fn lower_if(
        &mut self,
        return_local: Option<mir::LocalId>,
        if_expr: &ast::If,
        span: Span,
    ) -> MirResult<()> {
        let ast::IfCondition::Expression(condition_id) = &if_expr.condition else {
            return Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedConditionExpression,
                Some(span),
            ));
        };
        let discriminant = self.lower_bool_condition(*condition_id)?;

        let then_id = self.new_block();
        let join_id = self.new_block();
        let else_id = if_expr.branch.as_ref().map(|_| self.new_block());
        let false_target = else_id.unwrap_or(join_id);

        self.seal(
            mir::Terminator::SwitchInt {
                discriminant,
                targets: vec![(mir::ConstValue::Bool(true), then_id)],
                otherwise: false_target,
            },
            Some(then_id),
        );

        let then_statements = self.store.blocks[if_expr.block].statements.clone();
        self.lower_body(return_local, &then_statements)?;
        let then_reaches_join = !self.is_terminated();
        if then_reaches_join {
            self.seal(mir::Terminator::Goto(join_id), None);
        }

        let else_reaches_join = match &if_expr.branch {
            None => true,
            Some(branch) => {
                self.current = else_id;
                match branch {
                    ast::IfBranch::Else(block_id) => {
                        let else_statements = self.store.blocks[*block_id].statements.clone();
                        self.lower_body(return_local, &else_statements)?;
                    }
                    ast::IfBranch::If(nested_if) => {
                        self.lower_if(return_local, nested_if, span)?;
                    }
                }

                let reaches = !self.is_terminated();
                if reaches {
                    self.seal(mir::Terminator::Goto(join_id), None);
                }
                reaches
            }
        };

        self.current = (then_reaches_join || else_reaches_join).then_some(join_id);
        Ok(())
    }

    fn lower_for(
        &mut self,
        return_local: Option<mir::LocalId>,
        for_expr: &ast::For,
        span: Span,
    ) -> MirResult<()> {
        let ast::ForCondition::While(condition_id) = &for_expr.condition else {
            return Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedLoopCondition,
                Some(span),
            ));
        };

        let header_id = self.new_block();
        let body_id = self.new_block();
        let exit_id = self.new_block();

        self.seal(mir::Terminator::Goto(header_id), Some(header_id));

        let discriminant = self.lower_bool_condition(*condition_id)?;
        self.seal(
            mir::Terminator::SwitchInt {
                discriminant,
                targets: vec![(mir::ConstValue::Bool(true), body_id)],
                otherwise: exit_id,
            },
            Some(body_id),
        );

        self.loops.push(LoopTargets {
            header: header_id,
            exit: exit_id,
        });
        let body_statements = self.store.blocks[for_expr.block].statements.clone();
        let body_result = self.lower_body(return_local, &body_statements);
        self.loops.pop();
        body_result?;

        if !self.is_terminated() {
            self.seal(mir::Terminator::Goto(header_id), None);
        }

        // The exit block is always reachable via the header's false edge,
        // regardless of how the body ended.
        self.current = Some(exit_id);
        Ok(())
    }

    fn lower_break(&mut self, span: Span) -> MirResult<()> {
        let Some(target) = self.loops.last().map(|l| l.exit) else {
            return Err(Fault::error_with_kind(
                MirErrorKind::BreakOutsideLoop,
                Some(span),
            ));
        };
        self.seal(mir::Terminator::Goto(target), None);
        Ok(())
    }

    fn lower_continue(&mut self, span: Span) -> MirResult<()> {
        let Some(target) = self.loops.last().map(|l| l.header) else {
            return Err(Fault::error_with_kind(
                MirErrorKind::ContinueOutsideLoop,
                Some(span),
            ));
        };
        self.seal(mir::Terminator::Goto(target), None);
        Ok(())
    }

    fn lower_bool_condition(&mut self, expr_id: ast::ExpressionId) -> MirResult<mir::Operand> {
        if !self.expression_is_bool(expr_id) {
            let span = self.store.expressions[expr_id].span;
            return Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedConditionExpression,
                Some(span),
            ));
        }
        self.lower_operand(expr_id)
    }

    fn expression_is_bool(&self, expr_id: ast::ExpressionId) -> bool {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ast::ExpressionKind::Literal((_, ast::Literal::Bool(_))) => true,
            ast::ExpressionKind::Variable(var) => self.is_type_boolean(var),
            ast::ExpressionKind::Unary(unary) => {
                matches!(unary.operator.value, UnaryOperatorKind::Not)
            }
            ast::ExpressionKind::Binary(_) => matches!(
                self.declares.get_expression_type(expr_id),
                Some(SoulType::Primitive(PrimitiveTypes::Boolean))
            ),
            _ => false,
        }
    }

    fn is_type_boolean(&self, var: &ast_model::VariableExpression) -> bool {
        let Some(resolved) = self.declares.get_variable_resolve(var.id) else {
            return false;
        };

        let Some((_, Some(ty), _)) = self.declares.get_variable_type(resolved) else {
            return false;
        };

        ty.is_primitive_kind(PrimitiveTypes::Boolean)
    }

    fn lower_variable(
        &mut self,
        stmt: &ast::Statement,
        var: &ast::Variable,
    ) -> Result<(), Fault<MirErrorKind>> {
        let ast::VarPattern::Simple { binding, modifier } = &var.pattern else {
            return Err(Fault::error_with_kind(
                MirErrorKind::NonSimpleVariablePatternUnsupported,
                Some(stmt.span),
            ));
        };

        let Some(init) = var.initialize_value else {
            return Err(Fault::error_with_kind(
                MirErrorKind::UninitializedVariableUnsupported,
                Some(stmt.span),
            ));
        };

        let ty = self
            .declares
            .get_variable_type(binding.id)
            .and_then(|(_, ty, _)| ty.clone())
            .ok_or_else(|| {
                Fault::error_with_kind(
                    MirErrorKind::VariableHasNoResolvedType,
                    Some(binding.ident.span()),
                )
            })?;

        self.require_lowerable(&ty, binding.ident.span())?;
        let rvalue = self.lower_rvalue(init)?;
        let local = self.alloc_local(ty, *modifier, binding.ident.span());
        self.node_to_local.insert(binding.id, local);
        self.statements
            .push(mir::Statement::Assign(mir::Place::local(local), rvalue));
        Ok(())
    }

    fn alloc_local(&mut self, ty: mir::Type, mutability: TypeModifier, span: Span) -> mir::LocalId {
        let id = self.local_alloc.alloc();
        self.locals.insert(
            id,
            mir::LocalDecl {
                ty,
                mutability,
                span,
            },
        );
        id
    }

    fn lower_rvalue(&mut self, expr_id: ast::ExpressionId) -> MirResult<mir::Rvalue> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ast::ExpressionKind::Binary(binary) => {
                if !is_supported_binary_op(binary.operator.value) {
                    return Err(Fault::error_with_kind(
                        MirErrorKind::UnsupportedBinaryOperator,
                        Some(expr.span),
                    ));
                }
                let left = self.lower_operand(binary.left)?;
                let right = self.lower_operand(binary.right)?;
                Ok(mir::Rvalue::BinaryOp(binary.operator.value, left, right))
            }
            ast::ExpressionKind::Unary(unary) => {
                if !matches!(unary.operator.value, UnaryOperatorKind::Not) {
                    return Err(Fault::error_with_kind(
                        MirErrorKind::UnsupportedUnaryOperator,
                        Some(expr.span),
                    ));
                }
                let operand = self.lower_operand(unary.value)?;
                Ok(mir::Rvalue::UnaryOp(unary.operator.value, operand))
            }
            ast::ExpressionKind::StructConstructor(ctor) => {
                self.lower_struct_constructor(ctor, expr.span)
            }
            _ => Ok(mir::Rvalue::Use(self.lower_operand(expr_id)?)),
        }
    }

    /// Lowers `Struct{field: value, ...}` into `Rvalue::Aggregate`, with the
    /// operands reordered to match the struct's own declared field order (not
    /// constructor-literal order) — that's what `PlaceElem::Field(usize)`
    /// indexes into later, both here and at every field-read site.
    fn lower_struct_constructor(
        &mut self,
        ctor: &ast::StructConstructor,
        span: Span,
    ) -> MirResult<mir::Rvalue> {
        if ctor.defaults {
            return Err(Fault::error_with_kind(
                MirErrorKind::StructConstructorDefaultsUnsupported,
                Some(span),
            ));
        }

        // Cloned so the field list doesn't keep borrowing `self.declares`
        // across the `&mut self` calls to `lower_operand` below.
        let struct_ = self
            .resolve_struct(&ctor.struct_type)
            .cloned()
            .ok_or_else(|| {
                Fault::error_with_kind(
                    MirErrorKind::NonPrimitiveType {
                        ty: format!("{:?}", ctor.struct_type).into(),
                    },
                    Some(span),
                )
            })?;

        let mut operands = Vec::with_capacity(struct_.fields.len());
        for field in &struct_.fields {
            let ast::VarPattern::Simple { binding, .. } = &field.value.pattern else {
                return Err(Fault::error_with_kind(
                    MirErrorKind::NonSimpleVariablePatternUnsupported,
                    Some(span),
                ));
            };
            let field_name = binding.ident.as_str();

            let value_id = ctor
                .values
                .iter()
                .find(|(name, _)| name.as_str() == field_name)
                .map(|(_, value_id)| *value_id)
                .ok_or_else(|| {
                    Fault::error_with_kind(
                        MirErrorKind::StructFieldNotFound {
                            struct_name: struct_.name.as_str().into(),
                            field: field_name.into(),
                        },
                        Some(span),
                    )
                })?;

            operands.push(self.lower_operand(value_id)?);
        }

        Ok(mir::Rvalue::Aggregate(mir::AggregateKind::Struct, operands))
    }

    fn lower_operand(&mut self, expr_id: ast::ExpressionId) -> MirResult<mir::Operand> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ast::ExpressionKind::Literal((_, literal)) => {
                Ok(mir::Operand::Constant(literal.clone()))
            }
            ast::ExpressionKind::Variable(var) => {
                let local = self.resolve_local(var, expr.span)?;
                Ok(mir::Operand::Copy(mir::Place::local(local)))
            }
            ast::ExpressionKind::FieldAccess(field_access) => {
                self.lower_field_access(field_access, expr.span)
            }
            ast::ExpressionKind::Binary(_) => {
                let span = expr.span;
                let ty = self
                    .declares
                    .get_expression_type(expr_id)
                    .cloned()
                    .ok_or_else(|| {
                        Fault::error_with_kind(
                            MirErrorKind::NestedExpressionHasNoResolvedType,
                            Some(span),
                        )
                    })?;

                require_primitive(&ty, span)?;

                let rvalue = self.lower_rvalue(expr_id)?;
                let temp = self.alloc_local(ty, TypeModifier::Immut, span);
                self.statements
                    .push(mir::Statement::Assign(mir::Place::local(temp), rvalue));
                Ok(mir::Operand::Copy(mir::Place::local(temp)))
            }
            // Only `!` is supported (checked inside `lower_rvalue`, which this
            // calls first), and it's always `bool`-typed — unlike `Binary`,
            // there's no resolver-recorded type to look up for `Unary`.
            ast::ExpressionKind::Unary(_) => {
                let span = expr.span;
                let rvalue = self.lower_rvalue(expr_id)?;
                let temp = self.alloc_local(
                    SoulType::Primitive(PrimitiveTypes::Boolean),
                    TypeModifier::Immut,
                    span,
                );
                self.statements
                    .push(mir::Statement::Assign(mir::Place::local(temp), rvalue));
                Ok(mir::Operand::Copy(mir::Place::local(temp)))
            }
            ast::ExpressionKind::FunctionCall(call) => {
                let span = expr.span;
                // Every intrinsic today is either `none`-returning
                // (`assert`/`panic`) or not yet lowerable at all — there's no
                // intrinsic that can currently produce a usable value, so any
                // intrinsic call reached here is always an error.
                if let Some(resolve) = self.declares.get_intrinsic_resolve(call.id) {
                    return Err(Fault::error_with_kind(
                        match resolve.kind {
                            IntrinsicFunction::Assert | IntrinsicFunction::Panic => {
                                MirErrorKind::CannotUseNoneValueAsOperand
                            }
                            other => MirErrorKind::UnsupportedIntrinsic {
                                name: other.as_str().into(),
                            },
                        },
                        Some(span),
                    ));
                }

                const WANTS_RESULT: bool = true;
                match self.lower_call(call, span, WANTS_RESULT)? {
                    Some(operand) => Ok(operand),
                    // Only reachable if the resolver let a `none`-returning
                    // call's result flow into a value context — trust the
                    // resolver's own type-checking to prevent this, same as
                    // `UnexpectedReturnValue`.
                    None => Err(Fault::error_with_kind(
                        MirErrorKind::CannotUseNoneValueAsOperand,
                        Some(span),
                    )),
                }
            }
            _ => Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedOperandExpression,
                Some(expr.span),
            )),
        }
    }
}

fn require_primitive(ty: &SoulType, span: Span) -> MirResult<()> {
    if matches!(ty, SoulType::Primitive(_)) {
        Ok(())
    } else {
        Err(Fault::error_with_kind(
            MirErrorKind::NonPrimitiveType {
                ty: format!("{ty:?}").into(),
            },
            Some(span),
        ))
    }
}

fn is_supported_binary_op(op: BinaryOperatorKind) -> bool {
    matches!(
        op,
        BinaryOperatorKind::Add
            | BinaryOperatorKind::Sub
            | BinaryOperatorKind::Mul
            | BinaryOperatorKind::Div
            | BinaryOperatorKind::Mod
            | BinaryOperatorKind::Eq
            | BinaryOperatorKind::NotEq
            | BinaryOperatorKind::Lt
            | BinaryOperatorKind::Gt
            | BinaryOperatorKind::Le
            | BinaryOperatorKind::Ge
            | BinaryOperatorKind::LogAnd
            | BinaryOperatorKind::LogOr
    )
}
