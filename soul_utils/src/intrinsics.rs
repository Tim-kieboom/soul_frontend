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
        }
    }
}
