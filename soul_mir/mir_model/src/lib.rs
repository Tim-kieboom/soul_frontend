//! MIR data structures. Pure shapes, no lowering logic — see `mir_parser` for the
//! AST-to-MIR construction pass. Mirrors rustc's MIR shape; see
//! `docs/mir-design.md` at the repo root for the rationale behind each piece.
//!
//! Only a subset of these variants is actually *constructed* by the current
//! (smallest-slice) lowering pass; the rest exist so later slices (control flow,
//! calls, structs, borrow checking) are new passes over an already-complete shape
//! rather than a shape migration.

use ast_model::{Literal, SoulType, operators::BinaryOperatorKind};
use soul_utils::{
    FunctionId, TypeModifier, collections::vec_map::VecMap, impl_soul_ids, span::Span,
};

impl_soul_ids!(LocalId, BlockId);

/// The type of a MIR local. A plain alias onto the frontend's resolved type for
/// now: M1/M2 only ever see concrete types. Once generics (M3) land this needs to
/// grow a `Param(String)` placeholder variant (see `docs/mir-design.md`'s Generics
/// section) — deliberately not added yet since nothing constructs it today.
pub type Type = SoulType;

/// A constant value baked into MIR. A plain alias onto the frontend's literal
/// representation; revisit if MIR ever needs a constant shape the AST doesn't
/// (e.g. a post-monomorphization sized-array constant).
pub type ConstValue = Literal;

pub struct MirProgram {
    pub functions: VecMap<FunctionId, Function>,
    /// `extern "C"` declarations — a signature with no body to lower at all,
    /// not a `Function` missing its blocks. Kept in a separate map rather
    /// than folded into `functions` so codegen can tell "declare only, no
    /// body to emit" apart from "should have a body" at the type level,
    /// instead of via a sentinel-empty `blocks`/`locals`.
    pub externs: VecMap<FunctionId, ExternFunction>,
}
impl MirProgram {
    pub const fn empty() -> Self {
        Self {
            functions: VecMap::const_default(),
            externs: VecMap::const_default(),
        }
    }
}

/// An `extern "C"` function declaration: just enough to declare it to LLVM
/// and lower calls to it — no body, no locals, no blocks. Every param/return
/// type is passed through as whatever `SoulType` the signature declared;
/// unlike `Function`'s body-lowering, there's no lowering logic here that
/// could break on a type it doesn't understand, so nothing is rejected at
/// this stage — codegen is the sole judge of which types it can actually
/// represent (see `mir_codegen`'s `NonPrimitiveType`/`UnsupportedPrimitiveType`).
#[derive(Debug, serde::Serialize)]
pub struct ExternFunction {
    pub id: FunctionId,
    pub params: Vec<Type>,
    /// `None` for a `none`(void)-returning extern function — same convention
    /// as `Function::return_local`.
    pub return_type: Option<Type>,
}

#[derive(Debug, serde::Serialize)]
pub struct Function {
    pub id: FunctionId,
    pub locals: VecMap<LocalId, LocalDecl>,
    pub blocks: VecMap<BlockId, BasicBlock>,
    /// `locals[0..arg_count]` are parameters, by convention.
    pub arg_count: usize,
    /// Holds the return value; conventionally `locals[arg_count]`. `None` for a
    /// `none`(void)-returning function — there's no value to hold, so no local
    /// is allocated for one rather than allocating a phantom, never-touched
    /// `none`-typed local just to fill this field.
    pub return_local: Option<LocalId>,
}

#[derive(Debug, serde::Serialize)]
pub struct LocalDecl {
    pub ty: Type,
    pub mutability: TypeModifier,
    pub span: Span,
}

#[derive(Debug, serde::Serialize)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

/// No control flow of their own — always fall through to the next statement (or
/// the block's terminator, for the last one).
#[derive(Debug, serde::Serialize)]
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

#[derive(Debug, serde::Serialize)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinaryOperatorKind, Operand, Operand),
    /// `Add`/`Sub`/`Mul` that traps on overflow instead of wrapping. Produces
    /// a `(T, bool)` tuple (result, overflowed) — mirrors rustc's own
    /// `CheckedBinaryOp` shape: lowering assigns this into a tuple-typed
    /// temp, then emits a `Terminator::Assert` on the `bool` half (field
    /// `1`) before using the result (field `0`), so the overflow *check* is
    /// an ordinary MIR `Assert` rather than something `mir_codegen` has to
    /// special-case.
    CheckedBinaryOp(BinaryOperatorKind, Operand, Operand),
    UnaryOp(ast_model::operators::UnaryOperatorKind, Operand),
    Ref {
        mutable: bool,
        place: Place,
    },
    Aggregate(AggregateKind, Vec<Operand>),
    Cast(Operand, Type),
    /// The runtime length of a slice-typed place (`[&]T`/`[&mut]T`'s own
    /// `len` field) — always `uint`-typed. Used by bounds-check lowering to
    /// compare against an index before an `Assert`, same as rustc's `Len`.
    Len(Place),
}

#[derive(Debug, serde::Serialize)]
pub enum AggregateKind {
    Struct,
    Tuple,
    Array,
}

#[derive(Debug, serde::Serialize)]
pub enum Operand {
    /// `place`'s type is `Copy` or `AutoCopy`; reading it doesn't invalidate the source.
    Copy(Place),
    /// Reading invalidates the source; lowering emits a `MarkMoved` for the
    /// underlying local alongside this.
    Move(Place),
    Constant(ConstValue),
}

#[derive(Debug, Clone, serde::Serialize)]
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

#[derive(Debug, Clone, serde::Serialize)]
pub enum PlaceElem {
    Field(usize),
    Index(LocalId),
    Deref,
}

/// Every block ends in exactly one of these; this is the whole CFG.
#[derive(Debug, serde::Serialize)]
pub enum Terminator {
    Goto(BlockId),
    /// Covers `if`/match-chain/traditional `match` uniformly.
    SwitchInt {
        discriminant: Operand,
        targets: Vec<(ConstValue, BlockId)>,
        otherwise: BlockId,
    },
    Call {
        id: FunctionId,
        arguments: Vec<Operand>,
        /// `None` when the callee returns `none`, or when the caller discards
        /// a non-`none` result (a bare `f(x);` statement) — either way there's
        /// nothing to write the result into.
        destination: Option<Place>,
        /// `None` = diverges (panics, or return type is `!`).
        target: Option<BlockId>,
    },
    /// `Drop`'s scope-exit call, gated at runtime by the local's drop flag.
    Drop {
        place: Place,
        target: BlockId,
    },
    /// `assert(cond)` / `panic(msg)`. Mirrors rustc's `Assert` terminator
    /// (minus `unwind`, since this compiler has no unwinding model): if
    /// `cond == expected`, execution continues at `target`; otherwise it
    /// panics with `msg`. An unconditional `panic(msg)` is `cond:
    /// Constant(Bool(false)), expected: true` — always takes the panic path,
    /// so `target` is allocated (the shape requires a `BlockId`) but never
    /// actually reachable, and lowering doesn't insert a real block for it.
    Assert {
        cond: Operand,
        expected: bool,
        msg: Operand,
        target: BlockId,
    },
    Return,
    /// Target for a diverging `Call`; also a bodyless infinite `for {}`.
    Unreachable,
}
