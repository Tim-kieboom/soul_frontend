//! Soul-type-to-LLVM-type mapping and the small constant/int-width helpers
//! that go with it — split out of `lib.rs` since every other module needs
//! these but none of them own any codegen *state* (no `Context`/`Builder`
//! wrapper, just pure functions over inkwell's type/value builders).

use ast_model::{ArrayKind, SoulType, Struct, TupleKind, declare_store::DeclareStore};
use inkwell::{
    AddressSpace,
    context::Context,
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FloatType, IntType},
    values::{BasicValueEnum, FloatValue, IntValue},
};
use mir_model::ConstValue;
use soul_utils::{
    compiler_options::PlatformInfo,
    fault::Fault,
    soul_names::PrimitiveTypes,
    span::{ModuleId, Span},
};

use crate::{
    err,
    fault::{CodegenErrorKind, CodegenResult},
};

pub(crate) fn param_metadata<'ctx>(
    types: &[BasicTypeEnum<'ctx>],
) -> Vec<BasicMetadataTypeEnum<'ctx>> {
    types.iter().map(|ty| (*ty).into()).collect()
}

pub(crate) fn const_int<'ctx>(
    ty: IntType<'ctx>,
    value: &ConstValue,
) -> CodegenResult<IntValue<'ctx>> {
    Ok(match value {
        ConstValue::Bool(b) => ty.const_int(u64::from(*b), false),
        ConstValue::Int(n) => ty.const_int(*n as u64, true),
        ConstValue::Uint(n) => ty.const_int(*n as u64, false),
        other => {
            return Err(err(CodegenErrorKind::UnsupportedConstant {
                value: format!("{other:?}").into_boxed_str(),
            }));
        }
    })
}

pub(crate) fn is_signed(prim: PrimitiveTypes) -> bool {
    use PrimitiveTypes::*;
    matches!(
        prim,
        CInt | UntypedInt | Int | Int8 | Int16 | Int32 | Int64 | Int128
    )
}

/// Builds a float constant — the `const_int` of the float world. `Int`/
/// `Uint` literals are accepted too (truncated to the destination's own
/// precision) since an untyped int literal can unify with a float-typed
/// context (`x: f64 = 5`) the same way it already does at the resolver
/// level.
pub(crate) fn const_float<'ctx>(
    ty: FloatType<'ctx>,
    value: &ConstValue,
) -> CodegenResult<FloatValue<'ctx>> {
    Ok(match value {
        ConstValue::Float(f) => ty.const_float(*f),
        ConstValue::Int(n) => ty.const_float(*n as f64),
        ConstValue::Uint(n) => ty.const_float(*n as f64),
        other => {
            return Err(err(CodegenErrorKind::UnsupportedConstant {
                value: format!("{other:?}").into_boxed_str(),
            }));
        }
    })
}

/// Resolves a struct-typed `SoulType::Stub`'s bare name back to its `Struct`
/// declaration — mirrors `mir_parser`'s own `resolve_struct`, since codegen
/// gets handed the exact same unresolved `SoulType` MIR lowering already
/// accepted. `None` for anything that isn't a `Stub`, or a `Stub` that
/// doesn't name an in-scope struct.
pub(crate) fn resolve_struct<'d>(
    declares: &'d DeclareStore,
    module: Option<ModuleId>,
    ty: &SoulType,
) -> Option<&'d Struct> {
    let SoulType::Stub(stub) = ty else {
        return None;
    };
    declares.get_struct_by_name(&stub.name, module?)
}

/// Narrows a value to an `IntValue`, faulting (not panicking) if it's
/// actually a pointer — the guard every int-only operator/branch-condition
/// site goes through.
pub(crate) fn expect_int(value: BasicValueEnum<'_>) -> CodegenResult<IntValue<'_>> {
    match value {
        BasicValueEnum::IntValue(v) => Ok(v),
        _ => Err(err(CodegenErrorKind::ExpectedIntOperand)),
    }
}

/// Narrows a value to a `FloatValue`, faulting if it's actually an int or a
/// pointer — the float-side counterpart to `expect_int`.
pub(crate) fn expect_float(value: BasicValueEnum<'_>) -> CodegenResult<FloatValue<'_>> {
    match value {
        BasicValueEnum::FloatValue(v) => Ok(v),
        _ => Err(err(CodegenErrorKind::ExpectedFloatOperand)),
    }
}

/// Maps a Soul type to its LLVM representation. Integers/`bool` map to the
/// matching `IntType`; `f32`/`f64` map to the matching `FloatType` (`f16`
/// isn't supported yet — falls through to `UnsupportedPrimitiveType`); `cstr`
/// and any reference/pointer type map to an (opaque, LLVM-16-style) pointer
/// type; a struct maps to an LLVM struct type with one field per declared
/// field, in declared order (the same order `mir_parser` uses for
/// `Rvalue::Aggregate` operands and `PlaceElem::Field` indices).
pub(crate) fn llvm_type<'ctx>(
    context: &'ctx Context,
    platform: &PlatformInfo,
    declares: &DeclareStore,
    module: Option<ModuleId>,
    ty: &SoulType,
    span: Option<Span>,
) -> CodegenResult<BasicTypeEnum<'ctx>> {
    use PrimitiveTypes::*;
    match ty {
        SoulType::Primitive(prim) => Ok(match prim {
            Boolean => context.bool_type().into(),
            Int8 | Uint8 => context.i8_type().into(),
            Int16 | Uint16 | Char16 => context.i16_type().into(),
            Int32 | Uint32 | Char | Char32 => context.i32_type().into(),
            Int64 | Uint64 | Char64 => context.i64_type().into(),
            Int128 | Uint128 => context.i128_type().into(),
            // Platform-sized (pointer-width).
            Int | Uint | UntypedInt | UntypedUint => {
                context.custom_width_int_type(platform.pointer_bits).into()
            }
            // C's `int`/`unsigned int` — always 32 bits here, regardless of pointer width.
            CInt | CUint => context.custom_width_int_type(platform.c_int_bits).into(),
            Char8 => context.i8_type().into(),
            CStr => context.ptr_type(AddressSpace::default()).into(),
            Float32 => context.f32_type().into(),
            // Untyped float literals default to `f64` the same way an
            // untyped int literal defaults to `int` elsewhere in this file
            // — matches the resolver's own `UntypedFloat` widening.
            Float64 | UntypedFloat => context.f64_type().into(),
            other => {
                return Err(Fault::error_with_kind(
                    CodegenErrorKind::UnsupportedPrimitiveType {
                        ty: format!("{other:?}").into_boxed_str(),
                    },
                    span,
                ));
            }
        }),
        SoulType::Reference(_) | SoulType::Pointer(_) => {
            Ok(context.ptr_type(AddressSpace::default()).into())
        }
        SoulType::Stub(_) => stub_type(context, platform, declares, module, ty, span),
        SoulType::Array(array) => array_type(context, platform, declares, module, array, span),
        SoulType::TupleKind(TupleKind::Tuple(types)) => {
            let field_types = types
                .iter()
                .map(|ty| llvm_type(context, platform, declares, module, ty, span))
                .collect::<CodegenResult<Vec<_>>>()?;
            Ok(context.struct_type(&field_types, false).into())
        }
        other => Err(Fault::error_with_kind(
            CodegenErrorKind::NonPrimitiveType {
                ty: format!("{other:?}").into_boxed_str(),
            },
            span,
        )),
    }
}

/// `[N]T` maps to a real fixed-size LLVM array (a value type, `N` elements
/// inline) — the only array kind that can be *constructed* as a value in
/// this slice (array literals). `[&]T`/`[&mut]T` map to a fat pointer: a
/// two-field `{ptr, len}` struct, `len` at pointer width to match `int`'s
/// own width (see `Int`/`Uint` above) — decided up front so bounds checking
/// can land later without changing the representation. `[_]T`/`[]T`
/// (wildcard-sized stack arrays, heap arrays) aren't supported yet.
fn array_type<'ctx>(
    context: &'ctx Context,
    platform: &PlatformInfo,
    declares: &DeclareStore,
    module: Option<ModuleId>,
    array: &ast_model::ArrayType,
    span: Option<Span>,
) -> CodegenResult<BasicTypeEnum<'ctx>> {
    match array.kind {
        ArrayKind::StackArray(len) => {
            let element_ty = llvm_type(context, platform, declares, module, &array.of_type, span)?;
            Ok(element_ty.array_type(len as u32).into())
        }
        ArrayKind::MutSlice | ArrayKind::ConstSlice => {
            let ptr_ty = context.ptr_type(AddressSpace::default());
            let len_ty = context.custom_width_int_type(platform.pointer_bits);
            Ok(context
                .struct_type(&[ptr_ty.into(), len_ty.into()], false)
                .into())
        }
        ArrayKind::StackArrayWildcard | ArrayKind::HeapArray => Err(Fault::error_with_kind(
            CodegenErrorKind::NonPrimitiveType {
                ty: format!("{:?}", SoulType::Array(array.clone())).into_boxed_str(),
            },
            span,
        )),
    }
}

fn stub_type<'ctx>(
    context: &'ctx Context,
    platform: &PlatformInfo,
    declares: &DeclareStore,
    module: Option<ModuleId>,
    ty: &SoulType,
    span: Option<Span>,
) -> Result<BasicTypeEnum<'ctx>, Fault<CodegenErrorKind>> {
    let struct_ = resolve_struct(declares, module, ty).ok_or_else(|| {
        Fault::error_with_kind(
            CodegenErrorKind::NonPrimitiveType {
                ty: format!("{ty:?}").into_boxed_str(),
            },
            span,
        )
    })?;
    let field_types = struct_
        .fields
        .iter()
        .map(|field| {
            let field_ty = field.value.ty.as_ref().ok_or_else(|| {
                Fault::error_with_kind(
                    CodegenErrorKind::NonPrimitiveType {
                        ty: format!("{ty:?}").into_boxed_str(),
                    },
                    span,
                )
            })?;
            llvm_type(context, platform, declares, module, field_ty, span)
        })
        .collect::<CodegenResult<Vec<_>>>()?;
    Ok(context.struct_type(&field_types, false).into())
}
