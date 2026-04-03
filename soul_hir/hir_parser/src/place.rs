use hir::{LocalId, Place, PlaceId, PlaceKind};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::IdAlloc,
    soul_error_internal,
};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub fn lower_place(&mut self, place: &ast::Expression) -> hir::PlaceId {
        let id = self.id_generator.alloc_place();
        let place = match &place.node {
            ast::ExpressionKind::Index(index) => Place::new(
                id,
                PlaceKind::Index {
                    base: self.lower_place(&index.collection),
                    index: self.lower_expression(&index.index),
                },
                place.span,
            ),
            ast::ExpressionKind::Deref { id: _, inner } => {
                Place::new(id, PlaceKind::Deref(self.lower_place(inner)), place.span)
            }
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
                Place::new(id, PlaceKind::Local(local), ident.span)
            }
            ast::ExpressionKind::FieldAccess(field) => return self.lower_field(field, place.span),
            other => {
                self.log_error(soul_error_internal!(
                    format!("{} can not be a Place", other.variant_str()),
                    Some(place.span)
                ));
                Place::new(id, PlaceKind::Local(LocalId::error()), place.span)
            }
        };

        self.insert_place(place)
    }

    pub(crate) fn find_local(&mut self, name: &Ident) -> Option<LocalId> {
        for store in self.scopes.iter().rev() {
            if let Some(id) = store.locals.get(name.as_str()).copied() {
                return Some(id);
            }
        }

        None
    }

    pub fn insert_place(&mut self, place: Place) -> PlaceId {
        let id = place.id;
        self.tree.nodes.places.insert(id, place);
        id
    }
}
