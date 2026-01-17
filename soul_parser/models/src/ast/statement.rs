use soul_utils::soul_names::TypeModifier;
use soul_utils::span::Span;
use soul_utils::{Ident, soul_import_path::SoulImportPath, span::Spanned};

use crate::ast::{Block, Expression, ExpressionKind, FunctionCall, NamedTupleType, SoulType};
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
    Import(Import),

    /// A standalone expression.
    Expression{id: Option<NodeId>, expression: Expression},

    /// A variable declaration.
    Variable(Variable),
    /// An assignment to an existing variable.
    Assignment(Assignment),

    /// A function declaration (with body block).
    Function(Function),
}

/// Imported paths
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Import {
    pub id: Option<NodeId>,
    pub paths: Vec<SoulImportPath>
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
    /// Function parameters.
    pub parameters: NamedTupleType,
    /// Return type, if specified.
    pub return_type: SoulType,
}

/// Optional `this` parameter type.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// The type of the variable (if type unknown typemodifier instead).
    pub ty: VarTypeKind,
    /// Optional initial value expression.
    pub initialize_value: Option<Expression>,

    pub node_id: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum VarTypeKind {
    NonInveredType(SoulType),
    InveredType(TypeModifier),
}

/// An assignment statement, e.g., `x = y + 1`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    pub node_id: Option<NodeId>,
    /// The left-hand side expression (the variable being assigned to).
    pub left: Expression,
    /// The right-hand side expression (the value being assigned).
    pub right: Expression,
}

impl VarTypeKind {
    pub fn get_modifier(&self) -> Option<TypeModifier> {
        match self {
            VarTypeKind::NonInveredType(soul_type) => soul_type.modifier,
            VarTypeKind::InveredType(type_modifier) => Some(*type_modifier),
        }
    }
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
        let expression = Expression::new(
            ExpressionKind::Block(block), 
            span,
        );

        Self::new(
            StatementKind::Expression{id: None, expression}, 
            span,
        )
    }
    
    fn from_expression(expression: Expression) -> Self {
        let (node, meta_data) = expression.consume();
        let expression = Expression::new(node, meta_data.span);
        Self::with_meta_data(StatementKind::Expression{id: None, expression}, meta_data)
    }
    
    fn from_function_call(function: Spanned<FunctionCall>) -> Self {
        let (node, meta_data) = function.consume();
        Self::with_meta_data(
            StatementKind::Expression{
                id: None, 
                expression: Expression::new(ExpressionKind::FunctionCall(node), meta_data.span), 
            },
            meta_data
        )
    }

    fn from_function(function: Spanned<Function>) -> Self {
        let (node, meta_data) = function.consume();
        Self::with_meta_data(
            StatementKind::Function(node),
            meta_data
        )
    }

    fn new_variable(variable: Variable, span: Span) -> Self {
        Self::new(StatementKind::Variable(variable), span)
    }
}
