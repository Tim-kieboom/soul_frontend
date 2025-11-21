use crate::error::Span;

pub type Attribute = u8/*TODO impl type*/;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
    pub attributes: Vec<Attribute>,
}
impl<T> Spanned<T> {
    pub fn new(inner: T, span: Span) -> Self {
        Self {node: inner, span, attributes: vec![]}
    } 
}

