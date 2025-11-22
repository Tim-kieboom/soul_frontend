use crate::{abstract_syntax_tree::{block::Block, conditionals::{For, If, Match, Ternary, While}, expression_groups::ExpressionGroup, function::{FunctionCall, Lambda, StaticMethod, StructConstructor}, literal::Literal, operator::{Binary, Unary}, soul_type::SoulType, spanned::Spanned, statment::Ident}, soul_page_path::SoulPagePath};

/// An expression in the Soul language, wrapped with source location information.
pub type Expression = Spanned<ExpressionKind>;
/// A boxed expression (used to avoid deep recursion in the AST).
pub type BoxExpression = Box<Expression>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    /// An empty expression (placeholder).
    Empty,
    /// A `default` literal or default value.
    Default,
    /// A literal value (number, string, etc.).
    Literal(Literal),
    
    /// Indexing into a collection, e.g., `arr[i]`.
    Index(Index),
    /// A lambda `() => {}`.
    Lambda(Lambda),
    /// A function call, e.g., `foo(x, y)`.
    FunctionCall(FunctionCall),
    /// Constructing a struct, e.g., `Point { x: 1, y: 2 }`.
    StructConstructor(StructConstructor),

    /// Accessing a field on an instance, e.g., `obj.field`.
    FieldAccess(FieldAccess),
    /// Accessing a static field, e.g., `Type.field`.
    StaticFieldAccess(StaticFieldAccess),
    /// Calling a static method, e.g., `Type.method()`.
    StaticMethod(StaticMethod),

    /// Referring to a variable `var`.
    Variable(Ident),
    /// An external expression from another page/module `path.to.something.expression`.
    ExternalExpression(ExternalExpression),

    /// A unary operation (negation, increment, etc.) `!1`.
    Unary(Unary),
    /// A binary operation (addition, multiplication, comparison, etc.) `1 + 2`.
    Binary(Binary),

    /// An `if` expression `if true {Println("is true")}`.
    If(If),
    /// A `for` loop `for i in 1..2 {Println(f"num:{i}")}`.
    For(For),
    /// A `while` loop `while true {Println("loop")}`.
    While(While),
    /// A `match` expression `match booleanVar {true => (), false => }`.
    Match(Match),
    /// A ternary expression `cond ? a : b`.
    Ternary(Ternary),

    /// A dereference, e.g., `*ptr`.
    Deref(BoxExpression),
    /// reference, e.g., `&x`(mut) or `@x`(const).
    Ref{is_mutable: bool, expression: BoxExpression},

    /// A block of statements, returning the last expression `{/*stuff*/}`.
    Block(Block),
    /// Return-like expressions (`return`, `break`, `fall`) `return 1`.
    ReturnLike(ReturnLike),
    /// A grouped expression, e.g., tuples, namedTuples or arrays.
    ExpressionGroup(ExpressionGroup),
}

/// A `return`, `fall`, or `break`-like expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReturnLike {
    pub value: Option<BoxExpression>,
    pub kind: ReturnKind
}
/// The kind of return-like expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ReturnKind {
    /// A returns function.
    Return,
    /// A returns current loop.
    Break,
    /// continue loop.
    Continue,
}

/// An expression from an external page/module.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalExpression {
    /// The path to the external page/module.
    pub path: SoulPagePath,
    /// The expression being accessed.
    pub expr: BoxExpression,
}

/// A static field access on a type, e.g., `Type.field`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StaticFieldAccess {
    /// The type being accessed.
    pub object: SoulType,
    /// The field name.
    pub field: Ident,
}

/// A field access on an instance, e.g., `obj.field`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldAccess {
    /// The object expression.
    pub object: BoxExpression,
    /// The field name.
    pub field: Ident,
}

/// An index operation, e.g., `arr[i]`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Index {
    /// The collection being indexed.
    pub collection: BoxExpression,
    /// The index expression.
    pub index: BoxExpression,
}


