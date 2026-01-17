use soul_utils::{
    Ident, soul_import_path::SoulImportPath, soul_names::KeyWord, span::{Span, Spanned}
};

use crate::{
    ast::{Array, Binary, BinaryOperator, Block, Literal, Unary, UnaryOperator},
    scope::NodeId,
};

/// An expression in the Soul language, wrapped with source location information.
pub type Expression = Spanned<ExpressionKind>;
/// A boxed expression (used to avoid deep recursion in the AST).
pub type BoxExpression = Box<Expression>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    /// A `default` literal or default value e.g., '()'.
    Default(Option<NodeId>),
    /// A literal value (number, string, etc.).
    Literal((Option<NodeId>, Literal)),

    /// Indexing into a collection, e.g., `arr[i]`.
    Index(Index),
    /// A function call, e.g., `foo(x, y)`.
    FunctionCall(FunctionCall),

    /// Referring to a variable `var`.
    Variable {
        id: Option<NodeId>,
        ident: Ident,
        resolved: Option<NodeId>,
    },
    /// An external expression from another page/module `path::to::something.expression`.
    ExternalExpression(ExternalExpression),

    /// A unary operation (negation, increment, etc.) `!1`.
    Unary(Unary),
    /// A binary operation (addition, multiplication, comparison, etc.) `1 + 2`.
    Binary(Binary),
    Array(Array),
    /// An `if` expression `if true {Println("is true")} else {Println("is else")}`.
    If(If),
    /// A conditional loop `while true {Println("loop")}`.
    While(While),
    /// A dereference, e.g., `*ptr`.
    Deref{id: Option<NodeId>, inner: BoxExpression},
    /// reference, e.g., `&x`(mut) or `@x`(const).
    Ref {
        id: Option<NodeId>,
        is_mutable: bool,
        expression: BoxExpression,
    },

    /// A block of statements, returning the last expression `{/*stuff*/}`.
    Block(Block),
    /// Return-like expressions (`return`, `break`) `return 1`.
    ReturnLike(ReturnLike),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Index {
    pub id: Option<NodeId>,
    /// The collection being indexed.
    pub collection: BoxExpression,
    /// The index expression.
    pub index: BoxExpression,
}

/// A function call expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionCall {
    /// The name of the function being called.
    pub name: Ident,
    /// Optional callee expression (for method calls).
    pub callee: Option<BoxExpression>,
    /// Function arguments.
    pub arguments: Vec<Expression>,
    pub id: Option<NodeId>,
    pub resolved: Option<NodeId>,
}

/// An expression from an external page/module.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalExpression {
    pub id: Option<NodeId>,
    /// The path to the external page/module.
    pub path: SoulImportPath,
    /// The expression being accessed.
    pub expr: BoxExpression,
}

/// An `if` statement or expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct If {
    pub id: Option<NodeId>,
    /// The condition expression.
    pub condition: BoxExpression,
    /// The block to execute if the condition is true.
    pub block: Block,
    /// Optional `else if` and `else` branches.
    pub else_branchs: Option<IfArm>,
}

pub type IfArm = Box<Spanned<ElseKind>>;

/// The kind of else branch in an `if` statement.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ElseKind {
    /// An `else if` branch (another conditional).
    ElseIf(Box<Spanned<If>>),
    /// An `else` branch (unconditional).
    Else(Spanned<Block>),
}

/// A `while` loop statement.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct While {
    pub id: Option<NodeId>,
    /// Optional condition expression. If `None`, the loop runs indefinitely.
    pub condition: Option<BoxExpression>,
    /// The loop body block.
    pub block: Block,
}

/// A `return`, `fall`, or `break`-like expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReturnLike {
    pub id: Option<NodeId>,
    pub value: Option<BoxExpression>,
    pub kind: ReturnKind,
}
/// The kind of return-like expression.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ReturnKind {
    /// A returns function.
    Return,
    /// A returns current loop.
    Break,
    /// continue loop.
    Continue,
}

impl ReturnKind {
    pub fn from_keyword(keyword: KeyWord) -> Option<Self> {
        match keyword {
            KeyWord::Break => Some(ReturnKind::Break),
            KeyWord::Return => Some(ReturnKind::Return),
            KeyWord::Continue => Some(ReturnKind::Continue),
            _ => None,
        }
    }
}

pub trait IfArmHelper {
    fn new_arm(kind: ElseKind, span: Span) -> Self;
    fn try_next_mut(&mut self) -> Option<&mut Option<IfArm>>;
}
impl IfArmHelper for IfArm {
    fn new_arm(kind: ElseKind, span: Span) -> Self {
        Self::new(Spanned::new(kind, span))
    }
    fn try_next_mut(&mut self) -> Option<&mut Option<IfArm>> {
        match &mut self.node {
            ElseKind::ElseIf(spanned) => Some(&mut spanned.node.else_branchs),
            ElseKind::Else(_) => None,
        }
    }
}

pub trait ExpressionHelpers {
    fn from_function_call(function_call: Spanned<FunctionCall>) -> Expression;
    fn from_array(array: Spanned<Array>) -> Expression;

    fn new_block(block: Block, span: Span) -> Expression;
    fn new_literal(literal: Literal, span: Span) -> Expression;
    fn new_unary(op: UnaryOperator, rvalue: Expression, span: Span) -> Expression;
    fn new_index(collection: Expression, index: Expression, span: Span) -> Expression;
    fn new_binary(lvalue: Expression, op: BinaryOperator, rvalue: Expression, span: Span) -> Expression;
}
impl ExpressionHelpers for Expression {
    
    fn new_block(block: Block, span: Span) -> Expression {
        Expression::new(ExpressionKind::Block(block), span)
    }
 
    fn from_array(array: Spanned<Array>) -> Expression {
        let (array, meta_data) = array.consume();
        Expression::with_meta_data(ExpressionKind::Array(array), meta_data)
    }
    
    fn new_unary(op: UnaryOperator, rvalue: Expression, span: Span) -> Expression {
        let unary = Unary{
            id: None,
            operator: op,
            expression: Box::new(rvalue),
        };
        Expression::new(ExpressionKind::Unary(unary), span)
    }
    
    fn new_binary(lvalue: Expression, op: BinaryOperator, rvalue: Expression, span: Span) -> Expression {
        let binary = Binary {
            id: None,
            left: Box::new(lvalue),
            operator: op,
            right: Box::new(rvalue),
        };
        Expression::new(ExpressionKind::Binary(binary), span)
    }
    
    fn new_literal(literal: Literal, span: Span) -> Expression {
        Expression::new(ExpressionKind::Literal((None, literal)), span)
    }
    
    fn from_function_call(function_call: Spanned<FunctionCall>) -> Expression {
        let (node, meta_data) = function_call.consume();
        Expression::with_meta_data(ExpressionKind::FunctionCall(node), meta_data)
    }
    
    fn new_index(collection: Expression, index: Expression, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::Index(Index {
                id: None,  
                collection: Box::new(collection), 
                index: Box::new(index),
            }), 
            span,
        )
    }
}
