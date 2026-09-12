use crate::{
    fault::{MirErrorKind, MirResult},
    function::{
        FunctionLowerer,
        r#type::require_primitive,
        utils::{is_checked_arith_op, is_checked_div_op, is_supported_binary_op},
    },
};
use ast_model::{self as ast, SoulType, operators::UnaryOperatorKind};
use mir_model as mir;
use soul_utils::{
    TypeModifier, compiler_options::MirOptions, fault::Fault, intrinsics::IntrinsicFunction,
    soul_names::PrimitiveTypes, span::Span,
};

impl<'a> FunctionLowerer<'a> {
    pub(super) fn lower_operand(&mut self, expr_id: ast::ExpressionId) -> MirResult<mir::Operand> {
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
            ast::ExpressionKind::Index(index) => self.lower_index_access(index, expr.span),
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

    pub(super) fn lower_rvalue(&mut self, expr_id: ast::ExpressionId) -> MirResult<mir::Rvalue> {
        let should_check_overflow = || {
            self.options
                .mir
                .contains(MirOptions::CHECK_ALGORITHMIC_OVERFLOW)
        };

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
                // Floats never go through the checked-arithmetic/checked-div
                // paths: IEEE 754 overflow saturates to `inf`/`-inf` rather
                // than being UB the way integer overflow and `INT_MIN / -1`
                // are, and there's no `{s,u}*.with.overflow`-style intrinsic
                // for floats for `lower_checked_binary_op` to call anyway.
                let is_float = self.is_float_operand(&left, &right, expr_id);
                if !is_float
                    && is_checked_arith_op(binary.operator.value)
                    && should_check_overflow()
                {
                    return self.lower_checked_binary_op(
                        binary.operator.value,
                        left,
                        right,
                        expr_id,
                        expr.span,
                    );
                }
                if !is_float && is_checked_div_op(binary.operator.value) && should_check_overflow()
                {
                    return self.lower_checked_div(
                        binary.operator.value,
                        left,
                        right,
                        expr_id,
                        expr.span,
                    );
                }
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
            ast::ExpressionKind::Ref(ref_) => self.lower_ref(ref_, expr.span),
            ast::ExpressionKind::Array(ast::AnyArray::Array(array)) => {
                self.lower_array_literal(array)
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

    /// Lowers `[v1, v2, ...]` into `Rvalue::Aggregate` — the resolver already
    /// validated the literal's arity against its target `[N]T` type, so this
    /// doesn't re-check it; a mismatch here would mean the resolver let bad
    /// input through, same defensive category as elsewhere in this lowerer.
    fn lower_array_literal(&mut self, array: &ast::Array) -> MirResult<mir::Rvalue> {
        let mut operands = Vec::with_capacity(array.values.len());
        for &value_id in &array.values {
            operands.push(self.lower_operand(value_id)?);
        }
        Ok(mir::Rvalue::Aggregate(mir::AggregateKind::Array, operands))
    }
}
