use soul_utils::{span::Span, Ident};

use crate::{hir_type::LazyTypeId, ExpressionId, LocalId, PlaceId};

/// A memory location that can be read from or written to.
///
/// Places represent l-values in the language and are used for
/// assignments, loads, references, and indexing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Place {
    pub id: PlaceId,
    pub kind: PlaceKind,
    pub span: Span,
}
impl Place {
    pub fn new(id: PlaceId, kind: PlaceKind, span: Span) -> Self {
        Self { id, kind, span }
    }
}

/// A memory location that can be read from or written to.
///
/// Places represent l-values in the language and are used for
/// assignments, loads, references, and indexing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlaceKind {
    /// A local variable.
    Local(LocalId),

    /// A temperary local variable.
    Temp(LocalId),

    /// Dereference of another place.
    Deref(PlaceId),

    /// Indexed access into an aggregate.
    Index { base: PlaceId, index: ExpressionId },

    /// Field access within a composite type.
    Field { base: PlaceId, field: Ident },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalInfo {
    pub ty: LazyTypeId,
    pub kind: LocalKind,
    pub span: Option<Span>,
}
impl LocalInfo {
    pub fn is_temp(&self) -> bool {
        matches!(self.kind, LocalKind::Temp(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LocalKind {
    Variable(Option<ExpressionId>),
    Temp(ExpressionId),
    Parameter,
}
impl LocalKind {
    pub fn display_variant(&self) -> &str {
        match self {
            LocalKind::Variable(_) => "Variable",
            LocalKind::Temp(_) => "Temp",
            LocalKind::Parameter => "Parameter",
        }
    }
}
