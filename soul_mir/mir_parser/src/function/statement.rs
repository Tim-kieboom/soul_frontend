use crate::{
    fault::{MirErrorKind, MirResult},
    function::FunctionLowerer,
};
use ast_model::{self as ast, SoulType};
use mir_model as mir;
use soul_utils::{TypeModifier, fault::Fault, intrinsics::IntrinsicFunction, span::Span};

impl<'a> FunctionLowerer<'a> {
    pub(super) fn lower_statement(
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

    /// Dispatches an intrinsic call reached in statement position. `Ok(None)`
    /// means `call` isn't an intrinsic at all (no `IntrinsicResolve` stored
    /// for it) — the caller should fall through to the ordinary
    /// `FunctionResolve`-based `lower_call` path instead.
    pub(super) fn try_lower_intrinsic_statement(
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

    /// Lowers `left = right` (compound assignments like `n -= 1` are already
    /// desugared by the parser into `left = left - 1` before this ever runs,
    /// so `lower_rvalue` handles the right-hand side with no special-casing).
    /// `left` is a bare, already-declared variable, or any other place
    /// expression `resolve_place_expression` accepts (a struct field write, a
    /// slice-index write) — `*p` as an assignment target still faults, since
    /// nothing lowers pointer-dereference places yet.
    fn lower_assignment(&mut self, assignment: &ast::Assignment) -> MirResult<()> {
        let left = &self.store.expressions[assignment.left];
        let place = match &left.node {
            ast::ExpressionKind::Variable(var) => {
                mir::Place::local(self.resolve_local(var, left.span)?)
            }
            ast::ExpressionKind::FieldAccess(_) | ast::ExpressionKind::Index(_) => {
                self.resolve_place_expression(assignment.left, left.span)?.0
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
    pub(super) fn lower_call(
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
                span,
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
                span,
            },
            None,
        );
        Ok(())
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
}
