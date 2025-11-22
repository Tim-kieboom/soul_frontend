use crate::error::Span;

/// An attribute identifier (TODO: implement proper type).
pub type Attribute = u8;

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
impl<T> Spanned<T> {
    /// Creates a new `Spanned` node with the given inner value and span.
    pub fn new(inner: T, span: Span) -> Self {
        Self {node: inner, span, attributes: vec![]}
    } 
}

