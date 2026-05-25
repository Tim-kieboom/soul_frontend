use crate::{
    BlockId, EnumId, ExpressionId, LocalId, PlaceId, StructId, TypeId, UnionFieldId, UnionId,
    hir_type::LazyTypeId,
};
use ast::{BinaryOperator, Literal, UnaryOperator};
use soul_utils::{Ident, ids::FunctionId};

/// A typed HIR expression.
///
/// Every expression has a unique ID and an associated type.
/// Source spans are stored externally in `SpanMap`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Expression {
    pub id: ExpressionId,
    pub ty: LazyTypeId,
    pub kind: ExpressionKind,
}
impl Expression {
    pub fn is_literal(&self) -> bool {
        matches!(self.kind, ExpressionKind::Literal(_))
    }

    pub fn error(id: ExpressionId) -> Self {
        Self {
            id,
            ty: LazyTypeId::error(),
            kind: ExpressionKind::Error,
        }
    }
}

/// The different kinds of HIR expressions.
///
/// Expressions in HIR are fully typed, name-resolved, and free of
/// syntactic sugar. All expressions are identified by an `ExpressionId`,
/// and their source locations are stored externally in the `SpanMap`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    // --- Values ---
    /// `null` value
    Null,
    Error,

    /// A literal value (integer, float, string, etc.).
    Literal(Literal),
    Array(Array),

    /// A reference to a local variable.
    Local(LocalId),

    /// A reference to a function item.
    Function(FunctionId),

    // --- Memory operations ---
    /// Loads the value from a place.
    ///
    /// This represents reading from a variable, dereference, or indexed location.
    Load(PlaceId),

    /// Creates a reference to a place.
    ///
    /// The `mutable` flag indicates whether this is a mutable (`&`)
    /// or immutable (`@`) reference.
    Ref {
        place: PlaceId,
        mutable: bool,
    },

    /// Dereferences a pointer or reference expression.
    DeRef(ExpressionId),

    // --- Operators ---
    /// A unary operation.
    Unary(Unary),

    /// A binary operation.
    Binary(Binary),

    // --- Control flow ---
    /// An `if` expression.
    ///
    /// The expression evaluates the condition and executes either
    /// the `then_block` or the optional `else_block`.
    If {
        condition: ExpressionId,
        then_block: BlockId,
        else_block: Option<BlockId>,
        ends_with_else: bool,
    },

    /// A `while` loop expression.
    ///
    /// If `cond` is `None`, the loop is infinite.
    While {
        condition: Option<ExpressionId>,
        body: BlockId,
    },

    /// A match expression.
    Match {
        scrutinee: ExpressionId,
        arms: Vec<MatchArm>,
    },

    /// A match-method expression: `expr.Variant{body}` / chained.
    /// Variant names are resolved to indices in THIR/MIR.
    MatchMethod(MatchMethodHir),

    Block(BlockId),

    // --- Calls ---
    /// A function or method call.
    ///
    /// If `callee` is present, this represents a method-style call.
    Call {
        function: FunctionId,
        generics: Vec<TypeId>,
        has_callee: bool,
        arguments: Vec<ExpressionId>,
    },

    /// An external crate function call.
    ExternalCall {
        crate_name: String,
        function_name: String,
        generics: Vec<TypeId>,
        arguments: Vec<ExpressionId>,
    },

    // --- Type operations ---
    /// An explicit type cast.
    Cast {
        value: ExpressionId,
        cast_to: LazyTypeId,
    },

    StructConstructor {
        ty: StructId,
        defaults: bool,
        values: Vec<(Ident, ExpressionId)>,
    },

    EnumVariant {
        enum_id: EnumId,
        variant_name: Ident,
    },

    UnionConstructor {
        union_id: UnionId,
        variant_index: usize,
        variant_field_id: UnionFieldId,
        value: ExpressionId,
    },

    Sizeof(LazyTypeId),

    /// `intrinsic.UnionTag(union_val)` — returns the variant tag of a union value.
    UnionTag(ExpressionId),

    /// `intrinsic.UnionExtract(union_val)` — extracts the active variant's value.
    UnionExtract {
        value: ExpressionId,
    },

    /// `value typeof Type.Variant` — type check, returns `bool`.
    /// When `binding` is `Some`, the variant value is extracted and stored in the local.
    TypeOf(TypeOf),

    /// Pointer offset: given a `*T` and an integer offset, returns a new `*T`
    /// advanced by `offset * sizeof(T)` bytes.
    PtrOffset {
        pointer: ExpressionId,
        offset: ExpressionId,
    },

    /// Stack array element index: given a `[N]T` and an integer index, returns
    /// a `*T` pointing to the element at the given index.
    StackArrayIndex {
        array: ExpressionId,
        index: ExpressionId,
    },

    /// `new(expr)` — heap-allocate and initialize a single value, returns `*T`.
    New(ExpressionId),

    /// `new:[...]` — heap-allocate an array, returns `[*]T`.
    NewArray {
        values: Vec<ExpressionId>,
        ptr_type: TypeId,
    },

    /// `Exit(int)` — exit program with exit  code.
    Exit {
        exit_code: ExpressionId,
    },

    /// `Drop(expr)` — frees heap-allocated memory for pointer/heap-array types.
    Drop {
        value: ExpressionId,
    },

    /// `NewHeapArray(ptr, len)` — wraps a raw pointer and length into a `[*]T`.
    NewHeapArray {
        ptr: ExpressionId,
        len: ExpressionId,
    },

    /// `Alloc(size)` — allocates `size` bytes on the heap, returns `*none`.
    Alloc {
        size: ExpressionId,
    },

    /// `Dealloc(ptr)` — frees heap-allocated memory at `ptr: *none`.
    Dealloc {
        ptr: ExpressionId,
    },

    /// `Realloc(ptr, size)` — reallocates memory to `size` bytes, returns `*none`.
    Realloc {
        ptr: ExpressionId,
        size: ExpressionId,
    },
}

/// An typeof operation, e.g., `value typeof Union.Variant(binding)`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeOf {
    pub value: ExpressionId,
    pub union_id: UnionId,
    pub variant_index: usize,
    pub binding: Option<LocalId>,
}

/// An array literal, e.g., `[1, 2, 3]`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Array {
    pub collection_type: Option<LazyTypeId>,
    pub element_type: Option<LazyTypeId>,
    pub values: Vec<ExpressionId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Unary {
    pub operator: UnaryOperator,
    pub expression: ExpressionId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Binary {
    pub left: ExpressionId,
    pub operator: BinaryOperator,
    pub right: ExpressionId,
}

/// A match arm in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MatchArm {
    pub pattern: MatchPatternHir,
    pub body: BlockId,
}

/// A match pattern in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MatchPatternHir {
    Literal(Literal),
    Wildcard,
    /// Binds the scrutinee to a local variable.
    Binding(LocalId),
    Array(Vec<MatchPatternHir>),
    /// A union constructor pattern: `Type.Variant(binding)`.
    Constructor {
        union_id: UnionId,
        variant_index: usize,
        binding: Option<LocalId>,
    },
}

/// A match-method expression in HIR: `expr.Variant{body}` / chained.
/// Variant names are stored as strings; resolved to indices in THIR/MIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MatchMethodHir {
    pub value: ExpressionId,
    pub arms: Vec<MatchMethodArmHir>,
}

/// A single arm in a HIR match-method expression.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MatchMethodArmHir {
    pub variant_name: String,
    pub binding: Option<LocalId>,
    pub body: BlockId,
}
