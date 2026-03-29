use hir::{DisplayType, LazyTypeId, LocalId, PlaceId, PlaceKind};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    ids::IdAlloc,
};

use crate::{TypedHirContext, type_helpers::TypeHelpers};

impl<'a> TypedHirContext<'a> {
    pub(crate) fn infer_place(&mut self, place_id: PlaceId) -> LazyTypeId {
        let span = self.get_place(place_id).span;
        let ty = match &self.get_place(place_id).kind {
            PlaceKind::Temp(id) | PlaceKind::Local(id) => {
                if *id == LocalId::error() {
                    LazyTypeId::error()
                } else {
                    self.locals[*id]
                }
            }
            PlaceKind::Deref(place) => {
                let inner = self.infer_place(*place);
                let ty = match self.resolve_type_strict(inner, span) {
                    Some(val) => val,
                    None => return LazyTypeId::error(),
                };

                let deref = self
                    .id_to_type(ty)
                    .try_deref(&self.types, &self.infers, span);
                match deref {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        LazyTypeId::error()
                    }
                }
            }
            PlaceKind::Index { base, .. } => {
                let base = self.infer_place(*base);
                let resolved = match self.resolve_type_strict(base, span) {
                    Some(val) => val,
                    None => return LazyTypeId::error(),
                };

                let base_type = self.id_to_type(resolved);
                match &base_type.kind {
                    hir::HirTypeKind::Array { element, .. } => *element,
                    _ => {
                        self.log_error(SoulError::new(
                            format!(
                                "can only use index on an array type '{}' is not an array type",
                                self.id_to_type(resolved)
                                    .display(&self.types, &self.infers)
                            ),
                            SoulErrorKind::UnifyTypeError,
                            Some(span),
                        ));
                        LazyTypeId::error()
                    }
                }
            }
            PlaceKind::Field { base, field } => {
                let base = *base;
                let name = field.to_string();
                let object = self.infer_place(base);
                match self.get_field_access(object, name.as_str(), span) {
                    Some(field_id) => {
                        self.field_names.insert(field_id, name);
                        self.place_fields.insert(place_id, field_id);
                        self.fields[field_id].field_type
                    }
                    None => LazyTypeId::error(),
                }
            }
        };
        self.places.insert(place_id, ty);
        ty
    }
}
