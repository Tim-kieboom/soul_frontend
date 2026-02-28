use ast::{BinaryOperator, Literal, UnaryOperator};
use hir::{IdAlloc, TypeId, impl_soul_ids};
use soul_utils::{
    Ident,
    vec_map::{VecMap, VecMapIndex},
};

impl_soul_ids!(BlockId, LocalId, StatementId, PlaceId, TempId);

/// Mid-level Intermediate Representation (MIR) tree.
///
/// MIR is a lowered, control-flow explicit, expression-flattened IR.
/// It is designed to be:
/// - Easy to lower from HIR
/// - Easy to analyze (CFG, liveness, borrow checking later)
/// - Easy to lower to LLVM IR
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MirTree {
    /// Entry point function (usually `main`)
    pub main: hir::FunctionId,

    /// Temporary values created during lowering (SSA-like registers)
    /// Each temp has a type.
    pub temps: VecMap<TempId, TypeId>,

    /// Assignable memory locations (l-values)
    pub places: VecMap<PlaceId, Place>,

    /// Local variables (including parameters and locals)
    pub locals: VecMap<LocalId, Local>,

    /// Control-flow graph blocks
    pub blocks: VecMap<BlockId, Block>,

    /// All statements in the MIR
    pub statements: VecMap<StatementId, Statement>,

    /// Function metadata
    pub functions: VecMap<hir::FunctionId, Function>,
}

/// Lowered function definition in MIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    pub id: hir::FunctionId,
    pub name: Ident,

    /// Parameters are locals
    pub parameters: Vec<LocalId>,

    /// All locals declared in the function body
    pub locals: Vec<LocalId>,

    /// All blocks belonging to this function
    pub blocks: Vec<BlockId>,

    /// Return type of the function
    pub return_type: TypeId,
}

/// A local variable in MIR.
///
/// Locals represent stack-allocated storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Local {
    pub id: LocalId,
    pub ty: TypeId,
}

/// A basic block in MIR.
///
/// A block contains a linear list of statements and
/// ends with a terminator that controls flow.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub returnable: bool,
    pub terminator: Terminator,
    pub statements: Vec<StatementId>,
}

/// A MIR statement.
///
/// Statements perform side effects or compute values into places.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Statement {
    pub kind: StatementKind,
}

/// The kinds of MIR statements.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    /// Evaluate an operand and discard the result.
    /// Used for standalone expressions.
    Eval(Operand),

    /// Assign the result of an rvalue computation into a place.
    Assign {
        place: PlaceId,
        value: Rvalue,
    },

    StorageStart(Vec<LocalId>),
    StorageDead(LocalId),
}

/// A right-hand-side computation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Rvalue {
    pub kind: RvalueKind,
}

/// Different kinds of MIR rvalues.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RvalueKind {
    /// Move or copy an operand.
    Use(Operand),

    /// Binary operator (e.g. `a + b`)
    Binary {
        left: Operand,
        operator: BinaryOperator,
        right: Operand,
    },

    /// Unary operator (e.g. `-x`, `!x`)
    Unary {
        operator: UnaryOperator,
        value: Operand,
    },

    StackAlloc {
        ty: TypeId,
        len: Operand,
    },
}

/// Block terminators describe control flow edges.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Terminator {
    /// Return from the current function.
    Return(Option<Operand>),

    /// Jump unconditionally to another block.
    Goto(BlockId),

    /// Conditional branch.
    If {
        condition: Operand,
        then: BlockId,
        arm: BlockId,
    },

    /// Function call.
    ///
    /// Control flows to `next` after the call completes.
    Call {
        id: hir::FunctionId,
        arguments: Vec<Operand>,
        return_place: Option<PlaceId>,
        next: BlockId,
    },

    /// Indicates unreachable code (after errors or diverging control flow).
    Unreachable,
}

/// An operand represents a value used by MIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Operand {
    pub ty: TypeId,
    pub kind: OperandKind,
}

/// Kinds of operands.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OperandKind {
    /// A temporary register-like value.
    Temp(TempId),

    /// A local variable.
    Local(LocalId),

    /// A compile-time constant value.
    Comptime(Literal),

    None,
}

/// A memory location that can be assigned to.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Place {
    Temp(TempId),

    /// Dereference of a pointer operand: `*ptr`
    Deref(Operand),

    /// Local variable place.
    Local(LocalId),

    /// Indexed access: `base[index]`
    Index(PlaceId, Operand),

    /// Field access: `base.field`
    Field(PlaceId, hir::FieldId),
}

impl Statement {
    pub fn new(kind: StatementKind) -> Self {
        Self { kind }
    }
}

impl Rvalue {
    pub fn new(kind: RvalueKind) -> Self {
        Self { kind }
    }
}

impl Operand {
    pub fn new(ty: TypeId, kind: OperandKind) -> Self {
        Self { ty, kind }
    }
}
