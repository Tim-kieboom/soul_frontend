use crate::{
    fault::{MirErrorKind, MirResult},
    function::{FunctionLowerer, r#type::require_primitive, utils::signed_primitive_min},
};
use ast_model as ast;
use ast_model::{SoulType, operators::BinaryOperatorKind};
use mir_model as mir;
use soul_utils::{TypeModifier, fault::Fault, soul_names::PrimitiveTypes, span::Span};

impl<'a> FunctionLowerer<'a> {
    /// `Add`/`Sub`/`Mul` trap on overflow via an ordinary MIR `Assert`
    /// instead of a codegen-level check: assigns `Rvalue::CheckedBinaryOp`
    /// into a fresh `(T, bool)` tuple temp, asserts the `bool` half (field
    /// `1`) is `false`, then hands back `Rvalue::Use` of the result half
    /// (field `0`) as if this had been an ordinary `BinaryOp` all along —
    /// so every existing call site (`lower_operand`'s nested-expression
    /// materialization, a top-level `lower_assignment`/`lower_variable`)
    /// needs no change at all.
    pub(super) fn lower_checked_binary_op(
        &mut self,
        op: BinaryOperatorKind,
        left: mir::Operand,
        right: mir::Operand,
        expr_id: ast::ExpressionId,
        span: Span,
    ) -> MirResult<mir::Rvalue> {
        // Prefer deriving the type from whichever operand is actually a
        // place: the resolver never types a `FieldAccess`/`Index`
        // *expression* (see `resolve_place_expression`'s docs), so
        // `self.declares.get_expression_type(expr_id)` alone would leave an
        // expression like `s[0] + s[1]` untyped even though each operand's
        // own place type is known. Only fall back to the resolver's
        // whole-expression type when both operands are bare constants
        // (`3 + 4`) and so carry no place of their own.
        let result_ty = self
            .operand_type(&left)
            .or_else(|| self.operand_type(&right))
            .or_else(|| self.declares.get_expression_type(expr_id).cloned())
            .ok_or_else(|| {
                Fault::error_with_kind(MirErrorKind::NestedExpressionHasNoResolvedType, Some(span))
            })?;

        require_primitive(&result_ty, span)?;

        let tuple_ty = SoulType::TupleKind(ast::TupleKind::Tuple(vec![
            result_ty,
            SoulType::Primitive(PrimitiveTypes::Boolean),
        ]));
        let tuple_local = self.alloc_local(tuple_ty, TypeModifier::Immut, span);
        let tuple_place = mir::Place::local(tuple_local);
        self.statements.push(mir::Statement::Assign(
            tuple_place.clone(),
            mir::Rvalue::CheckedBinaryOp(op, left, right),
        ));

        let mut overflowed_place = tuple_place.clone();
        overflowed_place.projection.push(mir::PlaceElem::Field(1));

        let msg_text = match op {
            BinaryOperatorKind::Add => "attempt to add with overflow",
            BinaryOperatorKind::Sub => "attempt to subtract with overflow",
            BinaryOperatorKind::Mul => "attempt to multiply with overflow",
            _ => unreachable!("lower_checked_binary_op is only called for Add/Sub/Mul"),
        };
        let msg = mir::Operand::Constant(mir::ConstValue::Str(msg_text.to_string()));

        let next = self.new_block();
        self.seal(
            mir::Terminator::Assert {
                cond: mir::Operand::Copy(overflowed_place),
                expected: false,
                msg,
                target: next,
                span,
            },
            Some(next),
        );

        let mut result_place = tuple_place;
        result_place.projection.push(mir::PlaceElem::Field(0));
        Ok(mir::Rvalue::Use(mir::Operand::Copy(result_place)))
    }

    /// `Div`/`Mod` trap on the two ways LLVM's `sdiv`/`srem`/`udiv`/`urem`
    /// are undefined behavior instead of crashing the process outright:
    /// dividing by zero (both signed and unsigned), and — signed only —
    /// `MIN / -1` / `MIN % -1` (the one case a signed division can't
    /// represent). Unlike `Add`/`Sub`/`Mul` there's no `{s,u}*.with.overflow`
    /// intrinsic for division, so this emits explicit `Assert`s ahead of an
    /// ordinary, now-safe `Rvalue::BinaryOp` instead of a `CheckedBinaryOp`
    /// tuple — mirrors rustc's own checked-division lowering, which does the
    /// same thing for the same reason.
    pub(super) fn lower_checked_div(
        &mut self,
        op: BinaryOperatorKind,
        left: mir::Operand,
        right: mir::Operand,
        expr_id: ast::ExpressionId,
        span: Span,
    ) -> MirResult<mir::Rvalue> {
        let operand_ty = self
            .operand_type(&left)
            .or_else(|| self.operand_type(&right))
            .or_else(|| self.declares.get_expression_type(expr_id).cloned())
            .ok_or_else(|| {
                Fault::error_with_kind(MirErrorKind::NestedExpressionHasNoResolvedType, Some(span))
            })?;
        require_primitive(&operand_ty, span)?;

        let zero_msg = match op {
            BinaryOperatorKind::Div => "attempt to divide by zero",
            BinaryOperatorKind::Mod => "attempt to calculate the remainder with a divisor of zero",
            _ => unreachable!("lower_checked_div is only called for Div/Mod"),
        };
        let is_zero = self.bool_temp(
            mir::Rvalue::BinaryOp(
                BinaryOperatorKind::Eq,
                right.clone(),
                mir::Operand::Constant(mir::ConstValue::Int(0)),
            ),
            span,
        );
        self.assert_false(mir::Operand::Copy(is_zero), zero_msg, span);

        let SoulType::Primitive(prim) = &operand_ty else {
            unreachable!("require_primitive already rejected anything else");
        };
        if let Some(min_value) = signed_primitive_min(*prim, &self.options.platform) {
            let overflow_msg = match op {
                BinaryOperatorKind::Div => "attempt to divide with overflow",
                BinaryOperatorKind::Mod => "attempt to calculate the remainder with overflow",
                _ => unreachable!("lower_checked_div is only called for Div/Mod"),
            };
            let is_min = self.bool_temp(
                mir::Rvalue::BinaryOp(
                    BinaryOperatorKind::Eq,
                    left.clone(),
                    mir::Operand::Constant(mir::ConstValue::Int(min_value)),
                ),
                span,
            );
            let is_neg_one = self.bool_temp(
                mir::Rvalue::BinaryOp(
                    BinaryOperatorKind::Eq,
                    right.clone(),
                    mir::Operand::Constant(mir::ConstValue::Int(-1)),
                ),
                span,
            );
            let overflows = self.bool_temp(
                mir::Rvalue::BinaryOp(
                    BinaryOperatorKind::LogAnd,
                    mir::Operand::Copy(is_min),
                    mir::Operand::Copy(is_neg_one),
                ),
                span,
            );
            self.assert_false(mir::Operand::Copy(overflows), overflow_msg, span);
        }

        Ok(mir::Rvalue::BinaryOp(op, left, right))
    }
}
