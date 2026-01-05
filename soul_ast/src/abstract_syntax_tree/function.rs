use crate::{
    abstract_syntax_tree::{
        block::Block,
        expression::BoxExpression,
        expression_groups::{NamedTuple, Tuple},
        soul_type::{FunctionType, GenericDeclare, GenericDefine, NamedTupleType, SoulType},
        spanned::Spanned,
        statment::Ident,
    },
    sementic_models::scope::NodeId,
};
use soul_utils::soul_names::TypeModifier;

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
    pub contructor: Option<CtorKind>,
    /// The name of the function.
    pub name: Ident,
    /// Optional callee information for extension methods.
    pub callee: Option<Spanned<FunctionCallee>>,
    /// Generic type parameters.
    pub generics: Vec<GenericDeclare>,
    /// Function parameters.
    pub parameters: NamedTupleType,
    /// Type modifier (const, mut, etc.).
    pub modifier: TypeModifier,
    /// Return type, if specified.
    pub return_type: SoulType,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CtorKind {
    Normal,
    Array,
}

/// A struct constructor call, e.g., `Point { x: 1, y: 2 }`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StructConstructor {
    /// The type being constructed.
    pub calle: SoulType,
    /// Named arguments for the constructor.
    pub arguments: NamedTuple,
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
    pub arguments: Tuple,
    pub candidates: Vec<NodeId>,
}

/// Information about a function's callee (for extension methods).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionCallee {
    /// The extension type this method extends.
    pub extention_type: SoulType,
    /// Optional `this` parameter type.
    pub this: ThisCallee,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ThisCallee {
    /// `&this`
    MutRef,
    /// ``
    Static,
    /// `this`
    Consume,
    /// `@this`
    ConstRef,
}

/// A lambda/anonymous function expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Lambda {
    /// The lambda's signature.
    pub signature: LambdaSignature,
    /// The arguments passed to the lambda.
    pub arguments: Tuple,
    pub body: LamdbaBodyKind,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LamdbaBodyKind {
    Block(Block),
    Expression(BoxExpression),
}
/// The signature of a lambda function.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LambdaSignature {
    /// The function type of the lambda.
    pub ty: FunctionType,
    /// The kind of body (block or expression).
    pub body_kind: LambdaBody,
}

/// The body of a lambda function.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LambdaBody {
    /// A block body with statements.
    Block(Block),
    /// An expression body (single expression).
    Expression(BoxExpression),
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Parameter {
    /// The parameter name.
    pub name: Ident,
    /// The parameter type.
    pub ty: SoulType,
}

/// A static method call on a type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StaticMethod {
    /// The type the method is called on.
    pub callee: Spanned<SoulType>,
    pub function: FunctionCall,
}

impl ThisCallee {
    pub fn display(&self) -> &'static str {
        match self {
            ThisCallee::Static => "",
            ThisCallee::MutRef => "&this",
            ThisCallee::Consume => "this",
            ThisCallee::ConstRef => "@this",
        }
    }
}
