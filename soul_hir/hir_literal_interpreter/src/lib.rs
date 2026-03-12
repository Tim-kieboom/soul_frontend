use ast::Literal;
use hir::HirTree;

pub(crate) mod binary;
pub(crate) mod unary;
mod utils;
use hir_typed_context::HirTypedTable;
pub(crate) use utils::*;

use crate::{binary::interpret_binary, unary::interpret_unary};

pub fn try_literal_resolve_expression(
    hir: &HirTree,
    types: &HirTypedTable,
    value_id: hir::ExpressionId,
) -> Option<Literal> {
    let value = &hir.expressions[value_id];
    match &value.kind {
        hir::ExpressionKind::Null
        | hir::ExpressionKind::Block(_)
        | hir::ExpressionKind::DeRef(_)
        | hir::ExpressionKind::Literal(_)
        | hir::ExpressionKind::If { .. }
        | hir::ExpressionKind::Ref { .. }
        | hir::ExpressionKind::Function(_)
        | hir::ExpressionKind::Call { .. }
        | hir::ExpressionKind::Cast { .. }
        | hir::ExpressionKind::While { .. }
        | hir::ExpressionKind::InnerRawStackArray { .. } => None,

        hir::ExpressionKind::Load(place) => match place.node {
            hir::PlaceKind::Temp(_, _)
            | hir::PlaceKind::Local(_, _)
            | hir::PlaceKind::Deref(_, _)
            | hir::PlaceKind::Index { .. }
            | hir::PlaceKind::Field { .. } => None,
        },
        hir::ExpressionKind::Local(_) => None,
        hir::ExpressionKind::Unary(unary) => {
            let value = get_literal(hir, types, unary.expression)?;
            interpret_unary(&unary.operator, value.get_ref())
        }
        hir::ExpressionKind::Binary(binary) => {
            let left = get_literal(hir, types, binary.left)?;
            let right = get_literal(hir, types, binary.right)?;
            interpret_binary(left.get_ref(), &binary.operator, right.get_ref())
        }
    }
}

fn get_literal<'a>(
    hir: &'a HirTree,
    types: &HirTypedTable,
    value_id: hir::ExpressionId,
) -> Option<LiteralRef<'a>> {
    if let hir::ExpressionKind::Literal(literal) = &hir.expressions[value_id].kind {
        return Some(LiteralRef::Ref(literal));
    }
    try_literal_resolve_expression(hir, types, value_id).map(|literal| LiteralRef::Owner(literal))
}

/// enum to avoid allow owner and ref to avoid .clone()
enum LiteralRef<'a> {
    Owner(Literal),
    Ref(&'a Literal),
}
impl<'a> LiteralRef<'a> {
    fn get_ref(&self) -> &Literal {
        match self {
            LiteralRef::Owner(literal) => literal,
            LiteralRef::Ref(literal) => literal,
        }
    }
}
