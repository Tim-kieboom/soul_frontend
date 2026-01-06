use soul_utils::{Ident, span::Span};

use crate::{SoulType, scope::NodeId};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GenericDeclare {
    pub node_id: Option<NodeId>,
    pub kind: GenericDeclareKind,
    pub span: Span,
}

/// A generic parameter (lifetime or type).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GenericDeclareKind {
    /// A lifetime parameter.
    Lifetime(Ident),
    /// A type parameter.
    Type {
        name: Ident,
        default: Option<SoulType>,
    },
}

impl GenericDeclare {
    pub fn new_lifetime(ident: Ident, span: Span) -> Self {
        Self {
            span,
            node_id: None,
            kind: GenericDeclareKind::Lifetime(ident),
        }
    }

    pub fn new_type(name: Ident, default: Option<SoulType>, span: Span) -> Self {
        Self {
            span,
            node_id: None,
            kind: GenericDeclareKind::Type { name, default },
        }
    }
}
