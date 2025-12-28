use std::hash::Hash;

use crate::{abstract_syntax_tree::statment::Ident, error::Span};

/// An attribute identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Attribute {
    pub name: Ident,
    pub values: Vec<Ident>,
}

/// A node wrapped with source location information and attributes.
///
/// All AST nodes are wrapped in `Spanned` to track their location in the
/// source code for error reporting and debugging.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Spanned<T> {
    /// The actual AST node.
    pub node: T,
    /// The source code span where this node appears.
    pub span: Span,
    /// Additional attributes associated with this node.
    pub attributes: Vec<Attribute>,
}
impl<T: Hash> Hash for Spanned<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}
impl<T> Spanned<T> {
    pub fn new(inner: T, span: Span) -> Self {
        Self {
            node: inner,
            span,
            attributes: vec![],
        }
    }

    pub fn with_atribute(inner: T, span: Span, attributes: Vec<Attribute>) -> Self {
        Self {
            node: inner,
            span,
            attributes,
        }
    }
}
