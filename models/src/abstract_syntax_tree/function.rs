use crate::{abstract_syntax_tree::{block::Block, expression::{BoxExpression}, expression_groups::{NamedTuple, Tuple}, soul_type::{FunctionType, GenericParameter, NamedTupleType, SoulType, TypeGeneric}, spanned::Spanned, statment::Ident}, scope::scope::ScopeId, soul_names::TypeModifier};

/// A function definition with a signature and body block.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Function {
    /// The function's signature (name, parameters, return type, etc.).
    pub signature: FunctionSignature,
    /// The function's body block.
    pub block: Block,
}

/// A function signature describing a function's interface.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionSignature {
    /// The name of the function.
    pub name: Ident,
    /// Optional callee information for extension methods.
    pub callee: Option<Spanned<FunctionCallee>>,
    /// Generic type parameters.
    pub generics: Vec<GenericParameter>,
    /// Function parameters.
    pub parameters: NamedTupleType,
    /// Type modifier (const, mut, etc.).
    pub modifier: TypeModifier,
    /// Return type, if specified.
    pub return_type: SoulType,
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
    pub generics: Vec<TypeGeneric>,
    /// Function arguments.
    pub arguments: Tuple,
}

/// Information about a function's callee (for extension methods).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionCallee {
    /// The extension type this method extends.
    pub extention_type: SoulType,
    /// Optional `this` parameter type.
    pub this: ThisCallee,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// The scope identifier for the lambda's closure.
    pub scope_id: ScopeId,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LamdbaBodyKind {
    Block(Block),
    Expression(BoxExpression),
}
/// The signature of a lambda function.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LambdaSignature {
    /// The name of the lambda (if any).
    pub name: Ident,
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
    /// The method name.
    pub name: Ident,
    /// Generic type arguments.
    pub generics: Vec<TypeGeneric>,
    /// Method arguments.
    pub arguments: Tuple,
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