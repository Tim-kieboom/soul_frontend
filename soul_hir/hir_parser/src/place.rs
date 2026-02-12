use hir::{IdAlloc, LocalId};
use soul_utils::{error::{SoulError, SoulErrorKind}, soul_error_internal};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub fn lower_place(&mut self, place: &ast::Expression) -> hir::Place {
        match &place.node {
            ast::ExpressionKind::Index(index) => hir::Place::Index {
                base: Box::new(self.lower_place(&index.collection)),
                index: self.lower_expression(&index.index),
            },
            ast::ExpressionKind::Deref { id: _, inner } => {
                hir::Place::Deref(Box::new(self.lower_place(inner)))
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
                            Some(ident.span)
                        ));
                        LocalId::error()
                    }
                };
                hir::Place::Local(local)
            }
            other => {
                self.log_error(soul_error_internal!(
                    format!("{} can not be a Place", other.variant_str()),
                    Some(place.span)
                ));
                hir::Place::Local(LocalId::error())
            }
        }
    }
}
