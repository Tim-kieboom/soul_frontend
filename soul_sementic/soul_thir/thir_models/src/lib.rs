use soul_utils::span::Span;

mod statement;
mod expression;
pub use crate::statement::*; 
pub use crate::expression::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Body {
    pub statements: Vec<Statement>,
    pub span: Span,
}
