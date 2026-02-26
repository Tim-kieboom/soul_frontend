use soul_utils::span::Spanned;

use crate::{ExpressionId, FieldId, LocalId, PlaceId};

/// A memory location that can be read from or written to.
///
/// Places represent l-values in the language and are used for
/// assignments, loads, references, and indexing.
pub type Place = Spanned<PlaceKind>;

/// A memory location that can be read from or written to.
///
/// Places represent l-values in the language and are used for
/// assignments, loads, references, and indexing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlaceKind {
    /// A local variable.
    Local(LocalId, PlaceId),

    /// Dereference of another place.
    Deref(Box<Place>, PlaceId),

    /// Indexed access into an aggregate.
    Index {
        id: PlaceId,
        base: Box<Place>,
        index: ExpressionId,
    },

    /// Field access within a composite type.
    Field {
        id: PlaceId,
        base: Box<Place>,
        index: FieldId,
    },
}
impl PlaceKind {
    pub fn get_id(&self) -> PlaceId {
        match self {
            PlaceKind::Local(_, place_id) => *place_id,
            PlaceKind::Deref(_, place_id) => *place_id,
            PlaceKind::Index { id, .. } => *id,
            PlaceKind::Field { id, .. } => *id,
        }
    }
}
