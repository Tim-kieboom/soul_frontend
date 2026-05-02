use ast::{BinaryOperator, ExternLanguage, UnaryOperator};
use hir::{ComplexLiteral, FieldId, GenericId, StructId, TypeId};
use soul_utils::{Ident, ids::FunctionId, impl_soul_ids, span::ModuleId, vec_map::VecMap};

impl_soul_ids!(GlobalId, BlockId, LocalId, StatementId, PlaceId, TempId);

/// Mid-level Intermediate Representation (MIR) tree.
///
/// MIR is a lowered, control-flow explicit, expression-flattened IR.
/// It is designed to be:
/// - Easy to lower from HIR
/// - Easy to analyze (CFG, liveness, borrow checking later)
/// - Easy to lower to LLVM IR
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MirTree {
    pub root_module: ModuleId,
    pub entry_function: Option<FunctionId>,
    pub public_functions: Vec<FunctionId>,
    pub init_global_function: FunctionId,

    pub modules: VecMap<ModuleId, Module>,

    pub globals: VecMap<GlobalId, Global>,

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
    pub functions: VecMap<FunctionId, Function>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ModuleNodeId {
    BlockId(BlockId),
    GlobalId(GlobalId),
    FunctionId(FunctionId),
    StatementId(StatementId),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub modules: Vec<ModuleId>,
    pub parent: Option<ModuleId>,
    pub nodes: Vec<ModuleNodeId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Local {
    Runtime {
        id: LocalId,
        ty: TypeId,
    },
    Comptime {
        id: LocalId,
        ty: TypeId,
        value: ComplexLiteral,
    },
}
impl Local {
    pub fn id(&self) -> LocalId {
        match self {
            Local::Runtime { id, .. } => *id,
            Local::Comptime { id, .. } => *id,
        }
    }

    pub fn ty(&self) -> TypeId {
        match self {
            Local::Runtime { ty, .. } => *ty,
            Local::Comptime { ty, .. } => *ty,
        }
    }

    pub fn is_runtime(&self) -> bool {
        matches!(self, Local::Runtime { .. })
    }

    pub fn is_comptime(&self) -> bool {
        matches!(self, Local::Comptime { .. })
    }
}

/// Lowered function definition in MIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    pub id: FunctionId,
    pub name: Ident,

    pub body: FunctionBody,

    /// Parameters are locals
    pub parameters: Vec<LocalId>,
    pub generics: Vec<GenericId>,
    pub owner_type: TypeId,

    /// Return type of the function
    pub return_type: TypeId,

    pub from_module: ModuleId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FunctionBody {
    External(ExternLanguage),
    Internal {
        entry_block: BlockId,
        /// All locals declared in the function body
        locals: Vec<LocalId>,
        /// All blocks belonging to this function
        blocks: Vec<BlockId>,
    },
}
impl FunctionBody {
    pub fn is_internal(&self) -> bool {
        matches!(self, FunctionBody::Internal { .. })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Global {
    pub id: GlobalId,
    pub local: LocalId,
    pub ty: TypeId,
    pub literal: Option<ComplexLiteral>,
}
impl Global {
    pub fn is_comptime(&self) -> bool {
        self.literal.is_some()
    }
    pub fn is_runtime(&self) -> bool {
        self.literal.is_none()
    }
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

    /// Function call.
    ///
    /// Control flows to `next` after the call completes.
    /// has to return Value
    Call {
        id: FunctionId,
        arguments: Vec<Operand>,
        type_args: Vec<TypeId>,
        return_place: Option<PlaceId>,
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
    Place(Place),
    /// Move or copy an operand.
    Operand(Operand),
    CastUse {
        value: Operand,
        cast_to: TypeId,
    },

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

    Aggregate {
        struct_type: StructId,
        body: AggregateBody,
    },

    StackAlloc(TypeId),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AggregateBody {
    Runtime(Vec<Operand>),
    Comptime(Vec<(ComplexLiteral, TypeId)>),
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

    Exit,

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
    Comptime(ComplexLiteral),

    /// Ref Place (e.g. `&a` or `@a`)
    Ref {
        place: PlaceId,
        mutable: bool,
    },

    Sizeof(TypeId),
    Nullptr,
    None,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Place {
    pub kind: PlaceKind,
    pub ty: TypeId,
}
impl Place {
    pub fn new(kind: PlaceKind, ty: TypeId) -> Self {
        Self { kind, ty }
    }
}

/// A memory location that can be assigned to.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlaceKind {
    Temp(TempId),

    /// Dereference of a pointer operand: `*ptr`
    Deref(Operand),

    /// Local variable place.
    Local(LocalId),

    Field {
        base: PlaceId,
        field_id: FieldId,
        struct_type: StructId,
    },
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

    pub fn is_literal(&self) -> bool {
        matches!(self.kind, OperandKind::Comptime(_))
    }
}
