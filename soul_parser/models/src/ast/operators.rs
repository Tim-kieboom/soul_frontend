use crate::{ast::{BoxExpression, Expression}, scope::NodeId};
use soul_utils::span::Spanned;

/// A unary operator wrapped with source location information.
pub type UnaryOperator = Spanned<UnaryOperatorKind>;
/// A binary operator wrapped with source location information.
pub type BinaryOperator = Spanned<BinaryOperatorKind>;

/// A unary operation expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Unary {
    pub id: Option<NodeId>,
    /// The unary operator.
    pub operator: UnaryOperator,
    /// The operand expression.
    pub expression: BoxExpression,
}

/// A binary operation expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Binary {
    pub id: Option<NodeId>,
    /// The left-hand side expression.
    pub left: BoxExpression,
    /// The binary operator.
    pub operator: BinaryOperator,
    /// The right-hand side expression.
    pub right: BoxExpression,
}

/// The kind of unary operator.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum UnaryOperatorKind {
    Invalid,
    /// `-`
    Neg,                            
    /// `!`
    Not,                            
    /// `*`
    DeRef,                          
    /// `&`
    MutRef,                         
    /// `@`
    ConstRef,                       
    /// `++`
    Increment { before_var: bool }, 
    /// `--`
    Decrement { before_var: bool }, 
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BinaryOperatorKind {
    Invalid,
    /// `+`
    Add,  
    /// `-`
    Sub,  
    /// `*`
    Mul,  
    /// `/`
    Div,  
    /// `log`
    Log,  
    /// `**`
    Pow,  
    /// `</`
    Root, 
    /// `%`
    Mod,  

    /// `&`
    BitAnd, 
    /// `|`
    BitOr,  
    /// `^`
    BitXor, 

    /// `&&`    
    LogAnd, 
    /// `||`
    LogOr,  
    /// `==`
    Eq,     
    /// `!=`
    NotEq,  
    /// `<`
    Lt,     
    /// `>`
    Gt,     
    /// `<=`
    Le,     
    /// `>=`
    Ge,     

    /// Range operator (`..`).
    Range,
    /// Type check operator (`typeof`).
    TypeOf,
}

impl Binary {
    pub fn new(left: Expression, operator: BinaryOperator, right: Expression) -> Self {
        Self {
            id: None,
            left: Box::new(left),
            operator,
            right: Box::new(right),
        }
    }
}

impl UnaryOperatorKind {
    pub const fn as_str(&self) -> &str {
        match self {
            UnaryOperatorKind::Invalid => "<invalid>",
            UnaryOperatorKind::Neg => "-",
            UnaryOperatorKind::Not => "!",
            UnaryOperatorKind::Increment { .. } => "++",
            UnaryOperatorKind::Decrement { .. } => "--",
            UnaryOperatorKind::DeRef => "*",
            UnaryOperatorKind::MutRef => "&",
            UnaryOperatorKind::ConstRef => "@",
        }
    }
}

impl BinaryOperatorKind {
    pub const fn as_str(&self) -> &str {
        match self {
            BinaryOperatorKind::Invalid => "<invalid>",
            BinaryOperatorKind::Add => "+",
            BinaryOperatorKind::Sub => "-",
            BinaryOperatorKind::Mul => "*",
            BinaryOperatorKind::Div => "/",
            BinaryOperatorKind::Log => "log",
            BinaryOperatorKind::Pow => "**",
            BinaryOperatorKind::Root => "</",
            BinaryOperatorKind::Mod => "%",
            BinaryOperatorKind::BitAnd => "&",
            BinaryOperatorKind::BitOr => "|",
            BinaryOperatorKind::BitXor => "^",
            BinaryOperatorKind::LogAnd => "&&",
            BinaryOperatorKind::LogOr => "||",
            BinaryOperatorKind::Eq => "==",
            BinaryOperatorKind::NotEq => "!=",
            BinaryOperatorKind::Lt => "<=",
            BinaryOperatorKind::Gt => ">=",
            BinaryOperatorKind::Le => "<",
            BinaryOperatorKind::Ge => ">",
            BinaryOperatorKind::Range => "..",
            BinaryOperatorKind::TypeOf => "typeof",
        }
    }
}
