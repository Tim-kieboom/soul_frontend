use hir::{IdAlloc, LocalId, Place, PlaceKind};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_error_internal,
};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub fn lower_place(&mut self, place: &ast::Expression) -> hir::Place {
        match &place.node {
            ast::ExpressionKind::Index(index) => Place::new(
                PlaceKind::Index {
                    base: Box::new(self.lower_place(&index.collection)),
                    index: self.lower_expression(&index.index),
                },
                place.span,
            ),
            ast::ExpressionKind::Deref { id: _, inner } => Place::new(
                PlaceKind::Deref(Box::new(self.lower_place(inner))),
                place.span,
            ),
            ast::ExpressionKind::Variable {
                id: _,
                ident,
                resolved: _,
            } => {
                let local = match self.find_local(ident) {
                    Some(val) => val,
                    None => {
                        self.log_error(SoulError::new(
                            format!("'{}' not found in scope", ident.as_str()),
                            SoulErrorKind::NotFoundInScope,
                            Some(ident.span),
                        ));
                        LocalId::error()
                    }
                };
                Place::new(PlaceKind::Local(local), ident.span)
            }
            other => {
                self.log_error(soul_error_internal!(
                    format!("{} can not be a Place", other.variant_str()),
                    Some(place.span)
                ));
                Place::new(PlaceKind::Local(LocalId::error()), place.span)
            }
        }
    }
}
