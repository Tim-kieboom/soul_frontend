use ast::Literal;
use hir::HirTree;

mod utils;
pub(crate) mod unary;
pub(crate) mod binary;
use hir_typed_context::HirTypedTable;
pub(crate) use utils::*;

use crate::{binary::interpret_binary, unary::interpret_unary};

pub fn try_literal_resolve_expression(hir: &HirTree, types: &HirTypedTable, value_id: hir::ExpressionId) -> Option<Literal> {
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

        hir::ExpressionKind::Load(place) => {
            match place.node {
                hir::PlaceKind::Temp(_, _) 
                | hir::PlaceKind::Local(_, _)
                | hir::PlaceKind::Deref(_, _) 
                | hir::PlaceKind::Index { .. }
                | hir::PlaceKind::Field { .. } => None,
            }
        }
        hir::ExpressionKind::Local(_) => {
            None
        }
        hir::ExpressionKind::Unary(unary) => {
            let value = get_literal(hir, types, unary.expression)?;
            interpret_unary(&unary.operator, &value)
        }
        hir::ExpressionKind::Binary(binary) => {
            let left = get_literal(hir, types, binary.left)?;
            let right = get_literal(hir, types, binary.right)?;
            interpret_binary(&left, &binary.operator, &right)
        }
    }
}

fn get_literal(hir: &HirTree, types: &HirTypedTable, value_id: hir::ExpressionId) -> Option<Literal> {
    if let hir::ExpressionKind::Literal(literal) = &hir.expressions[value_id].kind {
        return Some(literal.clone());
    }
    try_literal_resolve_expression(hir, types, value_id)
}