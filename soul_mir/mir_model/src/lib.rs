//! MIR data structures. Pure shapes, no lowering logic — see `mir_parser` for the
//! AST-to-MIR construction pass. Mirrors rustc's MIR shape; see
//! `docs/mir-design.md` at the repo root for the rationale behind each piece.
//!
//! Only a subset of these variants is actually *constructed* by the current
//! (smallest-slice) lowering pass; the rest exist so later slices (control flow,
//! calls, structs, borrow checking) are new passes over an already-complete shape
//! rather than a shape migration.

use ast_model::{literal::Literal, operators::BinaryOperatorKind, soul_type::SoulType};
use soul_utils::{
    FunctionId, TypeModifier, collections::vec_map::VecMap, impl_soul_ids, span::Span,
};

impl_soul_ids!(LocalId, BlockId);

/// The type of a MIR local. A plain alias onto the frontend's resolved type for
/// now: M1/M2 only ever see concrete types. Once generics (M3) land this needs to
/// grow a `Param(String)` placeholder variant (see `docs/mir-design.md`'s Generics
/// section) — deliberately not added yet since nothing constructs it today.
pub type MirType = SoulType;

/// A constant value baked into MIR. A plain alias onto the frontend's literal
/// representation; revisit if MIR ever needs a constant shape the AST doesn't
/// (e.g. a post-monomorphization sized-array constant).
pub type ConstValue = Literal;

#[derive(Debug)]
pub struct MirFunction {
    pub name: FunctionId,
    pub locals: VecMap<LocalId, LocalDecl>,
    pub blocks: VecMap<BlockId, BasicBlock>,
    /// `locals[0..arg_count]` are parameters, by convention.
    pub arg_count: usize,
    /// Holds the return value; conventionally `locals[arg_count]`.
    pub return_local: LocalId,
}

#[derive(Debug)]
pub struct LocalDecl {
    pub ty: MirType,
    pub mutability: TypeModifier,
    pub span: Span,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

/// No control flow of their own — always fall through to the next statement (or
/// the block's terminator, for the last one).
#[derive(Debug)]
pub enum Statement {
    Assign(Place, Rvalue),
    /// Marks a local as moved-from without an assignment (e.g. the source operand
    /// of a destructive read). Needed so the borrow checker can flag "use after
    /// move" without inferring move points from `Rvalue` shapes.
    MarkMoved(LocalId),
    /// Explicit drop-flag toggle: false on move-out, true on (re)init. See
    /// `docs/mir-design.md`'s move/drop section for why this is the source of
    /// truth for "is this slot occupied," not the type system.
    SetDropFlag(LocalId, bool),
    /// Lexical marker only — no runtime or CFG effect.
    StorageDead(LocalId),
}

#[derive(Debug)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinaryOperatorKind, Operand, Operand),
    UnaryOp(ast_model::operators::UnaryOperatorKind, Operand),
    Ref { mutable: bool, place: Place },
    Aggregate(AggregateKind, Vec<Operand>),
    Cast(Operand, MirType),
}

#[derive(Debug)]
pub enum AggregateKind {
    Struct,
    Tuple,
    Array,
}

#[derive(Debug)]
pub enum Operand {
    /// `place`'s type is `Copy` or `AutoCopy`; reading it doesn't invalidate the source.
    Copy(Place),
    /// Reading invalidates the source; lowering emits a `MarkMoved` for the
    /// underlying local alongside this.
    Move(Place),
    Constant(ConstValue),
}

#[derive(Debug)]
pub struct Place {
    pub local: LocalId,
    pub projection: Vec<PlaceElem>,
}

impl Place {
    /// A bare local with no projection — the common case.
    pub fn local(local: LocalId) -> Self {
        Self {
            local,
            projection: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum PlaceElem {
    Field(usize),
    Index(LocalId),
    Deref,
}

/// Every block ends in exactly one of these; this is the whole CFG.
#[derive(Debug)]
pub enum Terminator {
    Goto(BlockId),
    /// Covers `if`/match-chain/traditional `match` uniformly.
    SwitchInt {
        discriminant: Operand,
        targets: Vec<(ConstValue, BlockId)>,
        otherwise: BlockId,
    },
    Call {
        func: FunctionId,
        args: Vec<Operand>,
        destination: Place,
        /// `None` = diverges (panics, or return type is `!`).
        target: Option<BlockId>,
    },
    /// `Drop`'s scope-exit call, gated at runtime by the local's drop flag.
    Drop {
        place: Place,
        target: BlockId,
    },
    Return,
    /// Target for a diverging `Call`; also a bodyless infinite `for {}`.
    Unreachable,
}
