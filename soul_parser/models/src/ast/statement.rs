use soul_utils::{Ident, soul_import_path::SoulImportPath, span::Spanned};

use crate::{Block, Expression, GenericDeclare, SoulType, scope::NodeId};

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
    pub callee_type: SoulType,
    pub callee_kind: CalleKind,
    /// Generic type parameters.
    pub generics: Vec<GenericDeclare>,
    /// Function parameters.
    pub parameters: Vec<(Ident, Expression)>,
    /// Return type, if specified.
    pub return_type: SoulType,
}

/// Optional `this` parameter type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CalleKind {
    /// `&this`
    MutRef,
    /// ``
    Static,
    /// `this`
    Consume,
    /// `@this`
    ConstRef,
}
impl CalleKind {
    pub fn display(&self) -> Option<&'static str> {
        match self {
            CalleKind::Static => None,
            CalleKind::MutRef => Some("&this"),
            CalleKind::Consume => Some("this"),
            CalleKind::ConstRef => Some("@this"),
        }
    }
}

/// A variable declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    /// The name of the variable.
    pub name: Ident,
    /// The type of the variable.
    pub ty: SoulType,
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
