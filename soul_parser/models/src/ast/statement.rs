use soul_utils::span::Span;
use soul_utils::{Ident, soul_import_path::SoulImportPath, span::Spanned};

use crate::ast::{Block, Expression, ExpressionKind, FunctionCall, GenericDeclare, NamedTupleType, SoulType};
use crate::scope::NodeId;
/// A statement in the Soul language, wrapped with source location information.
pub type Statement = Spanned<StatementKind>;

/// The different kinds of statements that can appear in the language.
///
/// Each variant corresponds to a syntactic construct, ranging from expressions
/// to type definitions and control structures.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    /// Imported paths
    Import(Vec<SoulImportPath>),

    /// A standalone expression.
    Expression(Expression),

    /// A variable declaration.
    Variable(Variable),
    /// An assignment to an existing variable.
    Assignment(Assignment),

    /// A function declaration (with body block).
    Function(Function),
}

/// A function definition with a signature and body block.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Function {
    /// The function's signature (name, parameters, return type, etc.).
    pub signature: Spanned<FunctionSignature>,
    /// The function's body block.
    pub block: Block,
    pub node_id: Option<NodeId>,
}

/// A function signature describing a function's interface.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionSignature {
    /// The name of the function.
    pub name: Ident,
    pub methode_type: SoulType,
    pub function_kind: FunctionKind,
    /// Generic type parameters.
    pub generics: Vec<GenericDeclare>,
    /// Function parameters.
    pub parameters: NamedTupleType,
    /// Return type, if specified.
    pub return_type: SoulType,
}

/// Optional `this` parameter type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FunctionKind {
    /// `&this`
    MutRef,
    /// ``
    Static,
    /// `this`
    Consume,
    /// `@this`
    ConstRef,
}
impl FunctionKind {
    pub fn display(&self) -> Option<&'static str> {
        match self {
            FunctionKind::Static => None,
            FunctionKind::MutRef => Some("&this"),
            FunctionKind::Consume => Some("this"),
            FunctionKind::ConstRef => Some("@this"),
        }
    }
}

/// A variable declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    /// The name of the variable.
    pub name: Ident,
    /// The type of the variable.
    pub ty: Option<SoulType>,
    /// Optional initial value expression.
    pub initialize_value: Option<Expression>,

    pub node_id: Option<NodeId>,
}

/// An assignment statement, e.g., `x = y + 1`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    /// The left-hand side expression (the variable being assigned to).
    pub left: Expression,
    /// The right-hand side expression (the value being assigned).
    pub right: Expression,
}

pub trait StatementHelpers {
    fn new_block(block: Block, span: Span) -> Self;
    fn from_expression(expression: Expression) -> Self;
    fn from_function(function: Spanned<Function>) -> Self;
    fn new_variable(variable: Variable, span: Span) -> Self;
    fn from_function_call(function: Spanned<FunctionCall>) -> Self;
}
impl StatementHelpers for Statement {
    fn new_block(block: Block, span: Span) -> Self {
        let expr = Expression::new(
            ExpressionKind::Block(block), 
            span,
        );

        Self::new(
            StatementKind::Expression(expr), 
            span,
        )
    }
    
    fn from_expression(expression: Expression) -> Self {
        let span = expression.span;
        let attributes = expression.attributes.clone();
        Self::with_atribute(StatementKind::Expression(expression), span, attributes)
    }
    
    fn from_function_call(function: Spanned<FunctionCall>) -> Self {
        Self::with_atribute(
            StatementKind::Expression(
                Expression::new(ExpressionKind::FunctionCall(function.node), function.span), 
            ),
            function.span,
            function.attributes,
        )
    }

    fn from_function(function: Spanned<Function>) -> Self {
        Self::with_atribute(
            StatementKind::Function(function.node),
            function.span,
            function.attributes,
        )
    }

    fn new_variable(variable: Variable, span: Span) -> Self {
        Self::new(StatementKind::Variable(variable), span)
    }
}
