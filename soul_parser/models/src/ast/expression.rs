use soul_utils::{
    Ident, soul_import_path::SoulImportPath, soul_names::KeyWord, span::{Span, Spanned}
};

use crate::{
    ast::{Array, Binary, BinaryOperator, Block, ExpressionGroup, GenericDefine, Literal, NamedTuple, SoulType, Unary, UnaryOperator},
    scope::NodeId,
};

/// An expression in the Soul language, wrapped with source location information.
pub type Expression = Spanned<ExpressionKind>;
/// A boxed expression (used to avoid deep recursion in the AST).
pub type BoxExpression = Box<Expression>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    /// An empty expression (placeholder).
    Empty,
    /// A `default` literal or default value e.g., '()'.
    Default,
    /// A literal value (number, string, etc.).
    Literal(Literal),

    /// Indexing into a collection, e.g., `arr[i]`.
    Index(Index),
    /// A function call, e.g., `foo(x, y)`.
    FunctionCall(FunctionCall),

    FieldAccess(FieldAccess),

    /// Referring to a variable `var`.
    Variable {
        ident: Ident,
        resolved: Option<NodeId>,
    },
    /// An external expression from another page/module `path::to::something.expression`.
    ExternalExpression(ExternalExpression),

    /// A unary operation (negation, increment, etc.) `!1`.
    Unary(Unary),
    /// A binary operation (addition, multiplication, comparison, etc.) `1 + 2`.
    Binary(Binary),

    /// used for type as parent in FieldAccess `int.MAX_VALUE`
    TypeNamespace(SoulType),

    StructConstructor(StructConstructor),

    /// An `if` expression `if true {Println("is true")} else {Println("is else")}`.
    If(If),
    /// A conditional loop `while true {Println("loop")}`.
    While(While),
    /// A dereference, e.g., `*ptr`.
    Deref(BoxExpression),
    /// reference, e.g., `&x`(mut) or `@x`(const).
    Ref {
        is_mutable: bool,
        expression: BoxExpression,
    },

    /// A block of statements, returning the last expression `{/*stuff*/}`.
    Block(Block),
    /// Return-like expressions (`return`, `break`) `return 1`.
    ReturnLike(ReturnLike),
    /// A grouped expression, e.g., tuples, namedTuples or arrays.
    ExpressionGroup(ExpressionGroup),
}


#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StructConstructor {
    pub ty: SoulType,
    pub named_tuple: NamedTuple,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Index {
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
    /// Generic type arguments.
    pub generics: Vec<GenericDefine>,
    /// Function arguments.
    pub arguments: Vec<Expression>,
    pub node_id: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldAccess {
    pub parent: BoxExpression,
    pub member: Ident,
}

/// An expression from an external page/module.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalExpression {
    /// The path to the external page/module.
    pub path: SoulImportPath,
    /// The expression being accessed.
    pub expr: BoxExpression,
}

/// An `if` statement or expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct If {
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
    /// Optional condition expression. If `None`, the loop runs indefinitely.
    pub condition: Option<BoxExpression>,
    /// The loop body block.
    pub block: Block,
}

/// A `return`, `fall`, or `break`-like expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReturnLike {
    pub value: Option<BoxExpression>,
    pub kind: ReturnKind,
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
    fn from_named_tuple(named_tuple: Spanned<NamedTuple>) -> Expression;
    fn from_array(array: Spanned<Array>) -> Expression;
    fn new_block(block: Block, span: Span) -> Expression;
    fn new_unary(op: UnaryOperator, rvalue: Expression, span: Span) -> Expression;
    fn new_binary(lvalue: Expression, op: BinaryOperator, rvalue: Expression, span: Span) -> Expression;
    fn new_literal(literal: Literal, span: Span) -> Expression;
    fn from_function_call(function_call: Spanned<FunctionCall>) -> Expression;
}
impl ExpressionHelpers for Expression {
    fn from_named_tuple(named_tuple: Spanned<NamedTuple>) -> Expression {
        
        Expression::with_atribute(
            ExpressionKind::ExpressionGroup(
                ExpressionGroup::NamedTuple(named_tuple.node)
            ), 
            named_tuple.span, 
            named_tuple.attributes,
        )
    }
    
    
    fn new_block(block: Block, span: Span) -> Expression {
        Expression::new(ExpressionKind::Block(block), span)
    }
    
    fn from_array(array: Spanned<Array>) -> Expression {
        Expression::with_atribute(
            ExpressionKind::ExpressionGroup(
                ExpressionGroup::Array(Box::new(array.node))
            ), 
            array.span, 
            array.attributes,
        )
    }
    
    fn new_unary(op: UnaryOperator, rvalue: Expression, span: Span) -> Expression {
        let unary = Unary{
            operator: op,
            expression: Box::new(rvalue),
        };
        Expression::new(ExpressionKind::Unary(unary), span)
    }
    
    fn new_binary(lvalue: Expression, op: BinaryOperator, rvalue: Expression, span: Span) -> Expression {
        let binary = Binary {
            left: Box::new(lvalue),
            operator: op,
            right: Box::new(rvalue),
        };
        Expression::new(ExpressionKind::Binary(binary), span)
    }
    
    fn new_literal(literal: Literal, span: Span) -> Expression {
        Expression::new(ExpressionKind::Literal(literal), span)
    }
    
    fn from_function_call(function_call: Spanned<FunctionCall>) -> Expression {
        Expression::with_atribute(ExpressionKind::FunctionCall(function_call.node), function_call.span, function_call.attributes)
    }
    
}
