use ast::Literal;
use hir::{ExpressionId, HirTree, LocalId};

pub(crate) mod binary;
pub(crate) mod unary;
mod utils;
use soul_utils::vec_map::VecMap;
use typed_hir::TypedHir;
pub(crate) use utils::*;

use crate::{binary::interpret_binary, unary::interpret_unary};

pub fn literal_resolve(hir: &HirTree, types: &TypedHir) -> VecMap<ExpressionId, Literal> {
    let mut literals = VecMap::const_default();
    let mut locals = VecMap::const_default();

    for (id, local_info) in hir.nodes.locals.entries() {
        let ty = types.types_map.id_to_type(types.types_table.locals[id]).expect("should have type");
        if ty.is_mutable() {
            continue;
        }

        match &local_info.kind {
            hir::LocalKind::Temp(value) 
            | hir::LocalKind::Variable(Some(value)) => _ = locals.insert(id, *value),
            _ => (),
        }
    }

    for value_id in hir.nodes.expressions.keys() {
        
        if let Some(literal) = try_literal_resolve_expression(hir, types, &locals, &literals, value_id) {
            literals.insert(value_id, literal);
        }
    }
 
    literals
}

fn try_literal_resolve_expression(
    hir: &HirTree,
    types: &TypedHir,
    locals: &VecMap<LocalId, ExpressionId>,
    literals: &VecMap<ExpressionId, Literal>,
    value_id: hir::ExpressionId,
) -> Option<Literal> {
    let value = &hir.nodes.expressions[value_id];
    match &value.kind {
        hir::ExpressionKind::Null
        | hir::ExpressionKind::Error
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

        hir::ExpressionKind::StructConstructor { .. } => None,

        hir::ExpressionKind::Load(place) => match &hir.nodes.places[*place].kind {
            hir::PlaceKind::Temp(id)
            | hir::PlaceKind::Local(id) => match locals.get(*id) {
                Some(value_id) => literals.get(*value_id).cloned(),
                None => None,
            },
            
            hir::PlaceKind::Deref(_)
            | hir::PlaceKind::Index { .. }
            | hir::PlaceKind::Field { .. } => None,
        },
        hir::ExpressionKind::Local(id) => match locals.get(*id) {
            Some(value_id) => literals.get(*value_id).cloned(),
            None => None,
        },
        hir::ExpressionKind::Unary(unary) => {
            let value = get_literal(hir, types, locals, literals, unary.expression)?;
            interpret_unary(&unary.operator, value.get_ref())
        }
        hir::ExpressionKind::Binary(binary) => {
            let left = get_literal(hir, types, locals, literals, binary.left)?;
            let right = get_literal(hir, types, locals, literals, binary.right)?;
            interpret_binary(left.get_ref(), &binary.operator, right.get_ref())
        }
    }
}

fn get_literal<'a>(
    hir: &'a HirTree,
    types: &TypedHir,
    locals: &VecMap<LocalId, ExpressionId>,
    literals: &VecMap<ExpressionId, Literal>,
    value_id: hir::ExpressionId,
) -> Option<LiteralRef<'a>> {
    if let hir::ExpressionKind::Literal(literal) = &hir.nodes.expressions[value_id].kind {
        return Some(LiteralRef::Ref(literal));
    }
    try_literal_resolve_expression(hir, types, locals, literals, value_id).map(|literal| LiteralRef::Owner(literal))
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
