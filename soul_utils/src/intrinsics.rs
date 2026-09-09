use crate::define_str_enum;

define_str_enum!(
    /// Compiler-provided `intrinsic.*` functions.
    ///
    /// Callable as `intrinsic.<path>(...)`, e.g. `intrinsic.array.toRaw(arr)`
    /// or `intrinsic.fieldIndex(t, index)`. Namespaced paths use a dotted
    /// string (`"array.toRaw"`); unnamespaced ones use a bare name (`"typeinfo"`).
    pub enum IntrinsicFunction {
        /// `intrinsic.array.toRaw<T>(arr: []T) -> *T` (unsafe)
        ArrayToRaw => "array.toRaw",
        /// `intrinsic.ptr.toSlice<T>(ptr: *T, len: uint) -> []T` (unsafe)
        PtrToSlice => "ptr.toSlice",
        /// `intrinsic.ptr.offset<T>(ptr: *T, index: int) -> *T` (unsafe)
        PtrOffset => "ptr.offset",
        /// `intrinsic.typeinfo(t: typeid) -> TypeInfo`
        TypeInfo => "typeinfo",
        /// `intrinsic.fieldIndex(t: typeid, index: uint) -> FieldInfo`
        FieldIndex => "fieldIndex",
        /// `intrinsic.fieldCount(t: typeid) -> uint`
        FieldCount => "fieldCount",
        /// `assert(cond: bool)` — panics with a default message if `cond` is
        /// `false`. Unlike every other intrinsic, callable *bare* — no
        /// `intrinsic.` prefix — since that's the only way real Soul code
        /// (and every other language's `assert`) ever calls it. See the
        /// resolver's `resolve_function_call` for where that's carved out.
        Assert => "assert",
        /// `panic(msg: str)` — unconditionally aborts the program with `msg`.
        /// Also callable bare, same reasoning as `Assert`.
        Panic => "panic",
    }
);

impl IntrinsicFunction {
    /// Number of arguments this intrinsic expects.
    pub const fn arity(&self) -> usize {
        match self {
            Self::ArrayToRaw => 1,
            Self::PtrToSlice => 2,
            Self::PtrOffset => 2,
            Self::TypeInfo => 1,
            Self::FieldIndex => 2,
            Self::FieldCount => 1,
            Self::Assert => 1,
            Self::Panic => 1,
        }
    }

    /// Whether this intrinsic may only be called inside an `unsafe` block.
    ///
    /// Not enforced yet — `unsafe` blocks have no dedicated AST representation
    /// in this compiler. Kept as metadata so the check is a one-line addition
    /// once they do.
    pub const fn requires_unsafe(&self) -> bool {
        match self {
            Self::ArrayToRaw | Self::PtrToSlice | Self::PtrOffset => true,
            Self::TypeInfo | Self::FieldIndex | Self::FieldCount => false,
            Self::Assert | Self::Panic => false,
        }
    }

    /// Whether this intrinsic is callable bare (no `intrinsic.` prefix) — see
    /// the resolver's `resolve_function_call`.
    pub const fn callable_bare(&self) -> bool {
        matches!(self, Self::Assert | Self::Panic)
    }
}
