use soul_utils::{
    Ident, impl_soul_ids,
    span::{Span, Spanned},
};

use crate::{
    AstStore, NodeId, block::BlockId, literal::Literal, operators::{BinaryOperator, UnaryOperator}, soul_type::SoulType,
};

impl_soul_ids!(ExpressionId);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Expression {
    pub node: ExpressionKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    /// `undefined`
    Undefined(NodeId),
    /// `null`
    Null(NodeId),
    /// none as expression e.g., '()'.
    None(NodeId),
    /// A literal value (number, string, etc.).
    Literal((NodeId, Literal)),
    StringFormat(StringFormat),

    /// Indexing into a collection, e.g., `arr[i]`.
    Index(Index),
    /// access field `object.field`
    FieldAccess(FieldAccess),
    /// A function call, e.g., `foo(x, y)`.
    FunctionCall(FunctionCall),
    /// a constructor/typeCast, e.g. `int.(1)`, `Struct.(1, 2, "foo")`
    Constructor(Constructor),
    /// An struct literal, e.g., `Struct{field: 1, field2: 2}`.
    StructConstructor(StructConstructor),

    /// Referring to a variable `var`.
    Variable(VariableExpression),
    /// An array, e.g., `[1, 2, 3]`, `[for 2 => 1]`.
    Array(AnyArray),
    /// An tuple, e.g., `.(1, "text")`
    Tuple(Vec<ExpressionId>),
    /// An namedTuple, e.g., `.{number: 1, text: "text"}`.
    NamedTuple(Vec<(Ident, ExpressionId)>),

    /// `i32.sizeof // returns 4`
    Sizeof(ExpressionId),
    /// `"text".copy // copys &str into str`
    Copy(ExpressionId),
    /// `i32.pass // returns is null or Err()`
    Pass(ExpressionId),

    /// `new(expr)` — heap-allocate and initialize a single value, returns `*T`.
    New(ExpressionId),
    /// `new[1, 2, 3]`, `new[for N => init]` — heap-allocate an array, returns `[]T`.
    NewArray(AnyArray),

    /// A unary operation (negation, increment, etc.) `-1`.
    Unary(Unary),
    /// A binary operation (addition, multiplication, comparison, etc.) `1 + 2`.
    Binary(Binary),

    /// reference, e.g., `&x` or `&mut x`.
    Ref(Ref),
    /// A dereference, e.g., `*ptr`.
    Deref(Deref),

    /// An `if` expression `if true {Println("is true")} else {Println("is else")}`.
    If(If),
    /// A match expression `match x { 1 => "one", _ => "other" }`.
    Match(Match),
    /// A match-method expression: `expr.Variant{body}` or `expr.Variant{param => body}`.
    /// Chained calls are flattened into multiple arms: `expr.V1{...}.V2{...}`.
    MatchMethod(MatchMethod),
    /// A loop `for true {Println("loop")}` or conditional loop `for true {Println("loop")}` or iterator `for el in [1, 2, 3] {Println(el)}`.
    For(For),

    /// a scope, e.g. `{}`
    Block(BlockId),
    /// `expr typeof Type.Variant` — type check a union value
    /// When `binding` is `Some`, the variant value is extracted and stored in the bound variable.
    /// `binding_id` is the NodeId of the bound variable (set by name resolver).
    TypeOf(TypeOf),

    Break,
    Continue,
    Return(Option<ExpressionId>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StringFormat {
    pub to_string: bool,
    pub trailing: String,
    pub parts: Vec<(String, ExpressionId)>,
}

/// `expr typeof Type.Variant` — type check a union value
/// When `binding` is `Some`, the variant value is extracted and stored in the bound variable.
/// `binding_id` is the NodeId of the bound variable (set by name resolver).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TypeOf {
    pub value: ExpressionId,
    pub kind: TypeofKind,
    pub binding: Option<Binding>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TypeofKind {
    Null,
    NotNull,
    Union {
        type_name: Ident,
        variant_name: Ident,
    },
}

/// A match-method expression `expr.Variant{body}` or chained `expr.V1{...}.V2{...}`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MatchMethod {
    /// The scrutinee expression (left side of the dot).
    pub scrutinee: ExpressionId,
    /// The match arms, one per `.Variant{...}` segment.
    pub arms: Vec<MatchMethodArm>,
    pub optional_map: bool,
}

/// A single arm in a match-method expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MatchMethodArm {
    /// The variant name (e.g., `Err` in `ok.Err{-1}`).
    pub variant: MatchMethodVariant,
    /// Optional binding parameter (e.g., `msg` in `err.Err{msg => body}`).
    pub binding: Option<Binding>,
    /// The body block.
    pub body: BlockId,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MatchMethodVariant {
    Null,
    Else,
    NotNull,
    Name(Ident),
}
impl MatchMethodVariant {
    pub fn as_str(&self) -> &str {
        match self {
            MatchMethodVariant::Null => "null",
            MatchMethodVariant::Else => "else",
            MatchMethodVariant::NotNull => "!null",
            MatchMethodVariant::Name(name) => name.as_str(),
        }
    }
}

/// An struct literal, e.g., `Struct{field: 1, field2: 2}`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StructConstructor {
    pub struct_type: SoulType,
    pub values: Vec<(Ident, ExpressionId)>,
    pub defaults: bool,
}

/// A `match` expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Match {
    /// The expression to match on.
    pub scrutinee: ExpressionId,
    /// The match arms.
    pub arms: Vec<MatchArm>,
}

/// A single arm in a match expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MatchArm {
    /// The pattern to match against.
    pub pattern: MatchPattern,
    /// The block to execute if this arm matches.
    pub body: BlockId,
}

/// A pattern in a match arm.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MatchPattern {
    /// A literal value pattern.
    Literal(Literal),
    /// A wildcard (default) pattern.
    Wildcard,
    /// optional is null `null => ()`.
    Null,
    /// optional is not null `!null(binding) => ()`.
    NotNull(Binding),
    /// A binding pattern: `name` binds the scrutinee to a variable.
    Binding(Binding),
    /// `pattern if condition => ()`
    If{
        pattern: Box<MatchPattern>,
        if_condition: ExpressionId,
    },
    /// An array pattern: [elem1, elem2, ...]
    Array(Vec<MatchPattern>),
    /// A union constructor pattern: `Type.Variant(binding)`.
    Constructor(MatchContructor),
    /// A tuple pattern: `(a, b, ..)`.
    Tuple(TupleMatchPattern),
    /// A named tuple / record pattern: `{field1, field2: alias, ..}`.
    NamedTuple(NamedTupleMatchPattern),
    /// A struct constructor pattern: `Struct{a, b: alias, ..}`.
    ConstructorStruct(ConstructorStructPattern),
    /// A rest pattern: `..` matches remaining elements/fields.
    Rest,
}

/// A tuple pattern: `(a, b, ..)`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TupleMatchPattern {
    pub elements: Vec<MatchPattern>,
    pub rest: bool,
}

/// A named tuple / record pattern: `{field1, field2: alias, ..}`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedTupleMatchPattern {
    pub fields: Vec<NamedMatchPattern>,
    pub rest: bool,
}

/// A single field in a named tuple / constructor pattern.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedMatchPattern {
    /// The field name being matched.
    pub field: Ident,
    /// Optional binding: `None` = wildcard/ignore, `Some` = bind value to this name.
    pub binding: Option<Binding>,
}

/// A struct constructor pattern: `Struct{a, b: alias, ..}`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConstructorStructPattern {
    pub type_name: Ident,
    pub fields: Vec<NamedMatchPattern>,
    pub rest: bool,
}

/// A union constructor pattern: `Type.Variant(binding)`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MatchContructor {
    pub type_name: Ident,
    pub variant_name: Ident,
    pub binding: Option<Binding>,
}

/// A binding pattern: `name` binds the scrutinee to a variable.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Binding {
    pub id: NodeId,
    pub ident: Ident,
}

/// An `if` statement or expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct If {
    pub condition: ExpressionId,
    pub block: BlockId,
    pub branch: Option<Box<IfBranch>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum IfBranch {
    If(If),
    Else(BlockId),
}

/// A loop `for true {Println("loop")}` or conditional loop `for true {Println("loop")}` or iterator `for el in [1, 2, 3] {Println(el)}`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct For {
    pub block: BlockId,
    pub condition: ForCondition,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ForCondition {
    Loop,
    While(ExpressionId),
    Foreach {
        index: Option<Binding>,
        element_kind: ForElementKind,
        collection: ExpressionId,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ForElementKind {
    Single([Binding; 1]), // Single is array to be able to use ForElementKind::iter
    Tuple(Vec<Binding>),
}

/// reference, e.g., `&x`(mut) or `@x`(const).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Ref {
    pub id: NodeId,

    pub is_mutable: bool,
    pub value: ExpressionId,
}

/// A dereference, e.g., `*ptr`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Deref {
    pub id: NodeId,
    pub value: ExpressionId,
}

/// A unary operation expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Unary {
    pub id: NodeId,

    /// The unary operator.
    pub operator: UnaryOperator,
    /// The operand expression.
    pub value: ExpressionId,
}

/// A binary operation expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Binary {
    pub id: NodeId,

    /// The left-hand side expression.
    pub left: ExpressionId,
    /// The binary operator.
    pub operator: BinaryOperator,
    /// The right-hand side expression.
    pub right: ExpressionId,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AnyArray {
    Array(Array),
    ArrayFiller(ArrayFiller),
}

/// An array literal, e.g., `[1, 2, 3]`, `List.[1, 2, 3]`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Array {
    pub id: NodeId,

    pub values: Vec<ExpressionId>,
    pub element_type: Option<SoulType>,
    pub collection_type: Option<SoulType>,
}

/// An array filler, e.g., `[for 3 => 0] //creates [0, 0, 0]`, `int.[for 1 => 1]`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArrayFiller {
    pub id: NodeId,

    pub amount: ExpressionId,
    pub element: ExpressionId,
    pub for_index: Option<Binding>,
    pub element_type: Option<SoulType>,
    pub collection_type: Option<SoulType>,
}

/// a contructor/typeCast, e.g. `int.(1)`, `Struct.(1, 2, "foo")`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Constructor {
    pub id: NodeId,

    pub ty: SoulType,
    pub arguments: Vec<Argument>,
}

/// Referring to a variable `var`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VariableExpression {
    pub id: NodeId,
    pub name: Ident,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Index {
    pub id: NodeId,
    pub index: ExpressionId,
    pub collection: ExpressionId,
    pub optional_map: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldAccess {
    pub id: NodeId,
    pub field: Ident,
    pub optional_map: bool,
    pub object: ExpressionId,
    pub is_enum_variant: bool,
}

/// A function call expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionCall {
    pub id: NodeId,
    pub optional_map: bool,

    /// The name of the function being called.
    pub name: Ident,
    pub generics: Vec<SoulType>,
    /// Optional callee expression (for method calls).
    pub callee: Option<FunctionCallee>,
    /// Function arguments.
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionCallee {
    pub value: ExpressionId,
    pub optional_map: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Argument {
    pub name: Option<Ident>,
    pub value: ExpressionId,
}

impl ForElementKind {
    pub fn iter(&self) -> impl Iterator<Item = &Binding> {
        match self {
            ForElementKind::Tuple(items) => items.iter(),
            ForElementKind::Single(value) => value.iter(),
        }
    }
}

impl Expression {
    pub const fn error() -> Self {
        Self {
            node: ExpressionKind::Null(NodeId::ERROR),
            span: Span::error(),
        }
    }

    pub const fn new(node: ExpressionKind, span: Span) -> Self {
        Self { node, span }
    }

    pub fn new_id(store: &mut AstStore, node: ExpressionKind, span: Span) -> ExpressionId {
        let this = Self { node, span };
        store.insert_expression(this)
    }

    pub const fn new_block(block: BlockId, span: Span) -> Expression {
        Expression::new(ExpressionKind::Block(block), span)
    }

    pub const fn new_literal(id: NodeId, literal: Literal, span: Span) -> Expression {
        Expression::new(ExpressionKind::Literal((id, literal)), span)
    }

    pub fn new_variable(id: NodeId, name: Ident) -> Expression {
        let span = name.span();
        Expression::new(
            ExpressionKind::Variable(VariableExpression {
                id,
                name,
            }),
            span,
        )
    }

    pub fn new_binary(
        id: NodeId,
        left: ExpressionId,
        operator: BinaryOperator,
        right: ExpressionId,
        span: Span,
    ) -> Expression {
        let binary = Binary {
            id,
            left,
            operator,
            right,
        };
        Expression::new(ExpressionKind::Binary(binary), span)
    }

    pub fn from_for(array: Spanned<For>) -> Expression {
        let Spanned { value, span } = array;
        Expression::new(ExpressionKind::For(value), span)
    }

    pub fn from_array(array: Spanned<Array>) -> Expression {
        let Spanned { value, span } = array;
        Expression::new(ExpressionKind::Array(AnyArray::Array(value)), span)
    }

    pub fn from_array_filler(array: Spanned<ArrayFiller>) -> Expression {
        let Spanned { value, span } = array;
        Expression::new(ExpressionKind::Array(AnyArray::ArrayFiller(value)), span)
    }

    pub fn from_any_array(array: Spanned<AnyArray>) -> Expression {
        let Spanned { value, span } = array;
        Expression::new(ExpressionKind::Array(value), span)
    }

    pub fn from_function_call(call: Spanned<FunctionCall>) -> Expression {
        let Spanned { value, span } = call;
        Self {
            node: ExpressionKind::FunctionCall(value),
            span,
        }
    }

    pub fn from_struct_contructor(ctor: Spanned<StructConstructor>) -> Expression {
        let Spanned { value, span } = ctor;
        Self {
            node: ExpressionKind::StructConstructor(value),
            span,
        }
    }

    pub fn new_unary(id: NodeId, op: UnaryOperator, value: ExpressionId, span: Span) -> Expression {
        let unary = Unary {
            id,
            value,
            operator: op,
        };
        Expression::new(ExpressionKind::Unary(unary), span)
    }

    pub fn new_ref(id: NodeId, is_mutable: bool, value: ExpressionId, new_span: Span) -> Expression {
        let new_ref = ExpressionKind::Ref(Ref {
            id,
            value,
            is_mutable,
        });
        Expression::new(new_ref, new_span)
    }

    pub fn new_deref(id: NodeId, value: ExpressionId, new_span: Span) -> Expression {
        let deref = ExpressionKind::Deref(Deref { value, id });
        Expression::new(deref, new_span)
    }

    pub fn new_index(id: NodeId, collection: ExpressionId, index: ExpressionId, span: Span, optional_map: bool) -> Expression {
        Expression::new(
            ExpressionKind::Index(Index {
                id,
                index,
                collection,
                optional_map,
            }),
            span,
        )
    }

    pub fn new_field(id: NodeId, store: &AstStore, object: ExpressionId, field: Ident, optional_map: bool) -> Expression {
        let span = store.expressions[object].span.combine(field.span());
        Expression::new(
            ExpressionKind::FieldAccess(FieldAccess {
                id,
                object,
                field,
                optional_map,
                is_enum_variant: false,
            }),
            span,
        )
    }
}

impl AnyArray {
    pub fn from_array(arr: Spanned<Array>) -> Spanned<Self> {
        let Spanned { value, span } = arr;
        Spanned {
            value: AnyArray::Array(value),
            span: span,
        }
    }

    pub fn from_array_filler(arr: Spanned<ArrayFiller>) -> Spanned<Self> {
        let Spanned { value, span } = arr;
        Spanned {
            value: AnyArray::ArrayFiller(value),
            span: span,
        }
    }
}
impl Array {
    pub fn new(id: NodeId, collection_type: Option<SoulType>) -> Self {
        Self {
            id,
            values: vec![],
            element_type: None,
            collection_type,
        }
    }
}

impl Binding {
    pub fn from_text(id: NodeId, text: impl Into<String>, span: Span) -> Self {
        Self {
            id,
            ident: Ident::new(text, span),
        }
    }

    pub fn new(id: NodeId, ident: Ident) -> Self {
        Self { ident, id }
    }
}
