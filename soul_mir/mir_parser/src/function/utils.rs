use ast_model::{SoulType, operators::BinaryOperatorKind};
use mir_model as mir;
use soul_utils::{
    TypeModifier, compiler_options::PlatformInfo, soul_names::PrimitiveTypes, span::Span,
};

use crate::function::FunctionLowerer;

impl<'a> FunctionLowerer<'a> {
    /// Assigns `rvalue` into a fresh `bool` temp and returns its `Place` —
    /// the small building block `lower_checked_div`'s guard conditions
    /// (`== 0`, `== MIN`, `&&`, ...) are built from.
    pub(super) fn bool_temp(&mut self, rvalue: mir::Rvalue, span: Span) -> mir::Place {
        let temp = self.alloc_local(
            SoulType::Primitive(PrimitiveTypes::Boolean),
            TypeModifier::Immut,
            span,
        );
        self.statements
            .push(mir::Statement::Assign(mir::Place::local(temp), rvalue));
        mir::Place::local(temp)
    }

    /// Seals the current block with `Terminator::Assert { cond, expected:
    /// false, .. }` — i.e. traps if `cond` is `true` — and opens a fresh
    /// continuation block as the new cursor. Shared by every "trap if this
    /// bad condition holds" check `lower_checked_div` builds.
    pub(super) fn assert_false(&mut self, cond: mir::Operand, msg_text: &str, span: Span) {
        let msg = mir::Operand::Constant(mir::ConstValue::Str(msg_text.to_string()));
        let next = self.new_block();
        self.seal(
            mir::Terminator::Assert {
                cond,
                expected: false,
                msg,
                target: next,
                span,
            },
            Some(next),
        );
    }
}

pub(super) fn is_supported_binary_op(op: BinaryOperatorKind) -> bool {
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

/// `Add`/`Sub`/`Mul` — the ops `lower_checked_binary_op` traps on overflow
/// for. Div/Mod go through the separate `lower_checked_div` instead (see
/// `is_checked_div_op`): division has no `{s,u}*.with.overflow`-style
/// intrinsic to call, so it can't reuse the tuple-producing shape.
pub(super) fn is_checked_arith_op(op: BinaryOperatorKind) -> bool {
    matches!(
        op,
        BinaryOperatorKind::Add | BinaryOperatorKind::Sub | BinaryOperatorKind::Mul
    )
}

/// `Div`/`Mod` — the ops `lower_checked_div` guards with explicit `Assert`s
/// ahead of an ordinary `BinaryOp` (division by zero for both; `MIN / -1`/
/// `MIN % -1` for a signed operand only — the one case a signed division
/// can't represent, since the mathematical result overflows the type).
pub(super) fn is_checked_div_op(op: BinaryOperatorKind) -> bool {
    matches!(op, BinaryOperatorKind::Div | BinaryOperatorKind::Mod)
}

/// Mirrors `mir_codegen::types::is_float` one layer up (`SoulType` here,
/// `PrimitiveTypes` there) — used by `is_float_operand` to keep floats out
/// of the checked-arithmetic/checked-div lowering paths.
pub(super) fn is_float_primitive(prim: PrimitiveTypes) -> bool {
    matches!(
        prim,
        PrimitiveTypes::Float16
            | PrimitiveTypes::Float32
            | PrimitiveTypes::Float64
            | PrimitiveTypes::UntypedFloat
    )
}

/// The minimum representable value of a signed primitive integer type, or
/// `None` if `prim` isn't a signed integer at all — used only by
/// `lower_checked_div` to build the `MIN`-comparison constant for a division
/// overflow check. `Int`/`UntypedInt`/`CInt` are platform-sized (see
/// `PlatformInfo`'s own doc comment on why nothing upstream of codegen is
/// normally supposed to read it) — this is the one place in `mir_parser`
/// that peeks it, since the *correct* `MIN` bit pattern genuinely depends on
/// the concrete width, and there's no way to express "the minimum value of
/// whatever width this ends up being" as a single width-agnostic MIR
/// constant the way `0`/`-1` already are.
pub(super) fn signed_primitive_min(prim: PrimitiveTypes, platform: &PlatformInfo) -> Option<i128> {
    use PrimitiveTypes::{CInt, Int, Int8, Int16, Int32, Int64, Int128, UntypedInt};
    let bits = match prim {
        Int8 => 8,
        Int16 => 16,
        Int32 => 32,
        Int64 => 64,
        Int128 => 128,
        Int | UntypedInt => platform.pointer_bits,
        CInt => platform.c_int_bits,
        _ => return None,
    };
    Some(match bits {
        8 => i8::MIN as i128,
        16 => i16::MIN as i128,
        32 => i32::MIN as i128,
        64 => i64::MIN as i128,
        128 => i128::MIN,
        // PlatformInfo only ever produces 32/64-bit pointer/C-int widths
        // today; this only exists so the match is exhaustive.
        _ => i64::MIN as i128,
    })
}
