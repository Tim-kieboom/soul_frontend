use soul_utils::span::Spanned;

use crate::{ExpressionId, FieldId, LocalId};

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
    Local(LocalId),

    /// Dereference of another place.
    Deref(Box<Place>),

    /// Indexed access into an aggregate.
    Index {
        base: Box<Place>,
        index: ExpressionId,
    },

    /// Field access within a composite type.
    Field { base: Box<Place>, index: FieldId },
}
