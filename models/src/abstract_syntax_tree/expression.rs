use itertools::Itertools;
use crate::{abstract_syntax_tree::{block::Block, conditionals::{ElseKind, For, If, Match, Ternary, While}, expression_groups::ExpressionGroup, function::{FunctionCall, Lambda, StaticMethod, StructConstructor}, literal::Literal, operator::{Binary, Unary}, soul_type::SoulType, spanned::Spanned, statment::Ident}, error::Span, soul_names::{KeyWord, TypeWrapper}, soul_page_path::SoulPagePath};

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

impl Expression {
    pub fn new_literal(literal: Literal, span: Span) -> Self {
        Self::new(ExpressionKind::Literal(literal), span)
    }
}

impl ExpressionKind {
    
    pub fn display(&self) -> String {

        match self {
            ExpressionKind::Empty => format!("<empty>"),
            ExpressionKind::Default => format!("()"),
            ExpressionKind::Literal(literal) => format!("{}", literal.value_to_string()),
            ExpressionKind::Index(index) => format!("{}[{}]", index.collection.node.display(), index.index.node.display()),
            ExpressionKind::Lambda(lambda) => format!("({}) => {{..}}", lambda.arguments.values.iter().map(|el| el.node.display()).join(", ")),
            ExpressionKind::FunctionCall(function_call) => format!("{}({})", function_call.name, function_call.arguments.values.iter().map(|el| el.node.display()).join(", ")),
            ExpressionKind::StructConstructor(struct_constructor) => format!(
                "{}{{{}}}", 
                struct_constructor.calle.display(), 
                struct_constructor.arguments.values.iter().map(|(name, expr)| format!("{}: {}", name, expr.node.display())).join(", "),
            ),
            ExpressionKind::FieldAccess(field_access) => format!(
                "{}.{}",
                field_access.field,
                field_access.object.node.display()
            ),
            ExpressionKind::StaticFieldAccess(static_field_access) => format!(
                "{}.{}",
                static_field_access.object.display(),
                static_field_access.field,
            ),
            ExpressionKind::StaticMethod(static_method) => format!(
                "{}.{}({})",
                static_method.callee.node.display(),
                static_method.name,
                static_method.arguments.values.iter().map(|el| el.node.display()).join(", "),
            ),
            ExpressionKind::Variable(variable) => variable.clone(),
            ExpressionKind::ExternalExpression(external_expression) => format!(
                "{}::{}",
                external_expression.path.as_str(),
                external_expression.expr.node.display(),
            ),
            ExpressionKind::Unary(unary) => format!(
                "{}{}",
                unary.operator.node.as_str(),
                unary.expression.node.display(),
            ),
            ExpressionKind::Binary(binary) => format!(
                "{}{}{}",
                binary.left.node.display(),
                binary.operator.node.as_str(),
                binary.right.node.display(),
            ),
            ExpressionKind::If(_if) => format!(
                "{} {} {{..}} {}",
                KeyWord::If.as_str(),
                _if.condition.node.display(),
                _if.else_branchs.iter().map(|el| match &el.node {
                    ElseKind::ElseIf(else_if) => format!("{} {} {} {{..}}", KeyWord::Else.as_str(), KeyWord::If.as_str(), else_if.node.condition.node.display()),
                    ElseKind::Else(_) => format!("{} {{..}}", KeyWord::Else.as_str()),
                }).join("")
            ),
            ExpressionKind::For(_for) => format!(
                "{} {}{} {{..}}",
                KeyWord::For.as_str(),
                _for.element.as_ref().map(|el| format!("{} {} ", el.display(), KeyWord::InForLoop.as_str())).unwrap_or(String::default()),
                _for.collection.node.display(),
            ),
            ExpressionKind::While(_while) => format!(
                "{} {} {{..}}",
                KeyWord::While.as_str(),
                _while.condition.as_ref().map(|el| el.node.display()).unwrap_or_default()
            ),
            ExpressionKind::Match(_match) => format!(
                "{} {} {{..}}",
                KeyWord::Match.as_str(),
                _match.condition.node.display(),
            ),
            ExpressionKind::Ternary(ternary) => format!(
                "{} ? {} : {}",
                ternary.condition.node.display(),
                ternary.if_branch.node.display(),
                ternary.else_branch.node.display(),
            ),
            ExpressionKind::Deref(spanned) => format!("{}{}", TypeWrapper::Pointer.as_str(), spanned.node.display()),
            ExpressionKind::Ref { is_mutable, expression } => format!(
                "{}{}",
                if *is_mutable {TypeWrapper::MutRef.as_str()} else {TypeWrapper::ConstRef.as_str()},
                expression.node.display(),
            ),
            ExpressionKind::Block(block) => format!("{} {{..}}", block.modifier.as_str()),
            ExpressionKind::ReturnLike(return_like) => format!(
                "{}{}",
                return_like.kind.as_keyword().as_str(),
                return_like.value.as_ref().map(|el| format!(" {}", el.node.display())).unwrap_or_default(),
            ),
            ExpressionKind::ExpressionGroup(expression_group) => expression_group.display(),
        }
    } 
}

impl ReturnKind {
    pub fn as_keyword(&self) -> KeyWord {
        match self {
            ReturnKind::Break => KeyWord::Break,
            ReturnKind::Return => KeyWord::Return,
            ReturnKind::Continue => KeyWord::Continue,
        }
    }
}
