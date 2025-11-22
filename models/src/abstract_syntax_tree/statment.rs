use crate::abstract_syntax_tree::{block::Block, enum_like::{Enum, Union}, expression::Expression, function::Function, objects::{Class, Struct, Trait}, soul_type::SoulType, spanned::Spanned};

/// A statement in the Soul language, wrapped with source location information.
pub type Statement = Spanned<StatementKind>;

/// The different kinds of statements that can appear in the language.
///
/// Each variant corresponds to a syntactic construct, ranging from expressions
/// to type definitions and control structures.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    /// A standalone expression.
    Expression(Expression),

    /// A variable declaration.
    Variable(Ident),
    /// An assignment to an existing variable.
    Assignment(Assignment),
    
    /// A function declaration (with body block).
    Function(Function),
    /// A scoped `use` block (soul version of rusts 'impl' with optional trait implementation).
    UseBlock(UseBlock),

    /// A class declaration.
    Class(Class),
    /// A struct declaration.
    Struct(Struct),
    /// A trait declaration.
    Trait(Trait),
    
    /// An enum declaration (c like enum).
    Enum(Enum),
    /// A union declaration (rust like enum).
    Union(Union),

    /// Marker for closing a block (used during parsing).
    CloseBlock,
}

/// An identifier (variable name, type name, etc.).
pub type Ident = String;

/// An assignment statement, e.g., `x = y + 1`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    /// The left-hand side expression (the variable being assigned to).
    pub variable: Expression,
    /// The right-hand side expression (the value being assigned).
    pub value: Expression,
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
}

/// A `use` block (similar to Rust's `impl` block).
///
/// Can optionally implement a trait for a type, or just add methods to a type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UseBlock {
    /// The trait being implemented, if any.
    pub impl_trait: Option<SoulType>,
    /// The type this block is for.
    pub ty: SoulType,
    /// The block containing method definitions.
    pub block: Block,
}