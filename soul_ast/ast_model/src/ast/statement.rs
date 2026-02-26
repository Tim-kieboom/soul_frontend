use enum_variant_name_const::EnumVariantNameConst;
use soul_utils::soul_names::TypeModifier;
use soul_utils::span::{ItemMetaData, Span};
use soul_utils::{Ident, soul_import_path::SoulImportPath, span::Spanned};

use crate::ast::{Block, Expression, ExpressionKind, FunctionCall, NamedTupleType, SoulType};
use crate::scope::NodeId;
/// A statement in the Soul language, wrapped with source location information.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Statement {
    pub node: StatementKind,
    pub span: Span,
    pub meta_data: ItemMetaData,
}

/// The different kinds of statements that can appear in the language.
///
/// Each variant corresponds to a syntactic construct, ranging from expressions
/// to type definitions and control structures.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, EnumVariantNameConst)]
pub enum StatementKind {
    /// Imported paths
    Import(Import),

    /// A standalone expression.
    Expression {
        id: Option<NodeId>,
        expression: Expression,
        ends_semicolon: bool,
    },

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
    pub paths: Vec<ImportPath>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportPath {
    pub module: SoulImportPath,
    pub kind: ImportKind,
}
impl ImportPath {
    pub fn new() -> Self {
        Self {
            module: SoulImportPath::new(),
            kind: ImportKind::This,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportKind {
    All,
    This,
    Items(Vec<Ident>),
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

impl Statement {
    pub fn new(node: StatementKind, span: Span) -> Self {
        Self {
            node,
            span,
            meta_data: ItemMetaData::default_const(),
        }
    }

    pub fn with_meta_data(node: StatementKind, span: Span, meta_data: ItemMetaData) -> Self {
        Self {
            node,
            span,
            meta_data,
        }
    }

    pub fn new_block(block: Block, span: Span, ends_semicolon: bool) -> Self {
        let expression = Expression::new(ExpressionKind::Block(block), span);

        Self::new(
            StatementKind::Expression {
                id: None,
                expression,
                ends_semicolon,
            },
            span,
        )
    }

    pub fn from_expression(expression: Expression, ends_semicolon: bool) -> Self {
        let Expression { node, span } = expression;
        let expression = Expression::new(node, span);
        Self::new(
            StatementKind::Expression {
                id: None,
                expression,
                ends_semicolon,
            },
            span,
        )
    }

    pub fn from_function_call(function: Spanned<FunctionCall>, ends_semicolon: bool) -> Self {
        let Spanned { node, span } = function;
        Self::new(
            StatementKind::Expression {
                id: None,
                expression: Expression::new(ExpressionKind::FunctionCall(node), span),
                ends_semicolon,
            },
            span,
        )
    }

    pub fn from_function(function: Spanned<Function>) -> Self {
        let Spanned { node, span } = function;
        Self::new(StatementKind::Function(node), span)
    }

    pub fn new_variable(variable: Variable, span: Span) -> Self {
        Self::new(StatementKind::Variable(variable), span)
    }
}
