use crate::{
    fault::{MirErrorKind, MirResult},
    function::{FunctionLowerer, is_float_primitive},
};
use ast_model::{self as ast, SoulType, operators::UnaryOperatorKind};
use mir_model as mir;
use soul_utils::{fault::Fault, soul_names::PrimitiveTypes, span::Span};

impl<'a> FunctionLowerer<'a> {
    /// The Soul type held at `place`, walking its projection the same way
    /// `resolve_field_place`/`resolve_index_place` computed it in the first
    /// place — a pure, side-effect-free re-derivation (no statements
    /// emitted, unlike those two), used by `operand_type` to type an
    /// already-lowered operand without re-lowering it. `None` for anything
    /// this walk can't resolve (a `Deref`, an unresolvable struct/field).
    pub(super) fn place_type(&self, place: &mir::Place) -> Option<SoulType> {
        let mut ty = self.locals.get(place.local)?.ty.clone();
        for elem in &place.projection {
            ty = match elem {
                mir::PlaceElem::Field(index) => {
                    if let SoulType::TupleKind(ast::TupleKind::Tuple(types)) = &ty {
                        types.get(*index)?.clone()
                    } else {
                        let struct_ = self.resolve_struct(&ty)?;
                        struct_.fields.get(*index)?.value.ty.clone()?
                    }
                }
                mir::PlaceElem::Index(_) => {
                    let SoulType::Array(array) = &ty else {
                        return None;
                    };
                    (*array.of_type).clone()
                }
                mir::PlaceElem::Deref => return None,
            };
        }
        Some(ty)
    }

    /// The Soul type an already-lowered operand carries, if any — a bare
    /// constant carries no type of its own (mirrors `mir_codegen::rvalue`'s
    /// `operand_type`, one layer up: Soul types here, LLVM types there).
    /// Needed because the resolver never assigns a type to a `FieldAccess`/
    /// `Index` *expression* (see `resolve_place_expression`'s docs), so a
    /// checked binary op between two such operands can't be typed by
    /// looking the original AST expression up in `self.declares` — it has
    /// to walk the already-built `Place` instead.
    pub(super) fn operand_type(&self, operand: &mir::Operand) -> Option<SoulType> {
        match operand {
            mir::Operand::Copy(place) | mir::Operand::Move(place) => self.place_type(place),
            mir::Operand::Constant(_) => None,
        }
    }

    /// Whether a binary expression's operands are float-typed — same
    /// operand-then-whole-expression fallback `lower_checked_binary_op`/
    /// `lower_checked_div` use to type themselves, reused here so
    /// `lower_rvalue` can decide *before* routing into either of those
    /// whether this is even an integer operation to begin with.
    pub(super) fn is_float_operand(
        &self,
        left: &mir::Operand,
        right: &mir::Operand,
        expr_id: ast::ExpressionId,
    ) -> bool {
        let ty = self
            .operand_type(left)
            .or_else(|| self.operand_type(right))
            .or_else(|| self.declares.get_expression_type(expr_id).cloned());
        matches!(ty, Some(SoulType::Primitive(p)) if is_float_primitive(p))
    }

    pub(super) fn expression_is_bool(&self, expr_id: ast::ExpressionId) -> bool {
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

    /// Resolves a struct-typed `SoulType::Stub`'s bare name back to its
    /// `Struct` declaration in this function's module. `None` for anything
    /// that isn't a `Stub`, or a `Stub` that doesn't name a struct in scope
    /// (an enum/trait, a generic, or an unresolved name).
    pub(super) fn resolve_struct(&self, ty: &SoulType) -> Option<&ast::Struct> {
        let SoulType::Stub(stub) = ty else {
            return None;
        };
        self.declares.get_struct_by_name(&stub.name, self.module?)
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
}

pub(super) fn require_primitive(ty: &SoulType, span: Span) -> MirResult<()> {
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
