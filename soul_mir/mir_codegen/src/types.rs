//! Soul-type-to-LLVM-type mapping and the small constant/int-width helpers
//! that go with it — split out of `lib.rs` since every other module needs
//! these but none of them own any codegen *state* (no `Context`/`Builder`
//! wrapper, just pure functions over inkwell's type/value builders).

use ast_model::{SoulType, Struct, declare_store::DeclareStore};
use inkwell::{
    AddressSpace,
    context::Context,
    types::{BasicMetadataTypeEnum, BasicTypeEnum, IntType},
    values::{BasicValueEnum, IntValue},
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
    function::FunctionCodegen,
    module::ModuleCodegen,
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

impl<'ctx, 'a> ModuleCodegen<'ctx, 'a> {
    pub(crate) fn llvm_type(
        &self,
        module: Option<ModuleId>,
        ty: &SoulType,
        span: Option<Span>,
    ) -> CodegenResult<BasicTypeEnum<'ctx>> {
        llvm_type(
            self.context,
            &self.platform,
            self.declares,
            module,
            ty,
            span,
        )
    }
}
impl<'ctx, 'a> FunctionCodegen<'ctx, 'a> {
    pub(crate) fn llvm_type(
        &self,
        module: Option<ModuleId>,
        ty: &SoulType,
        span: Option<Span>,
    ) -> CodegenResult<BasicTypeEnum<'ctx>> {
        llvm_type(
            self.context,
            &self.platform,
            self.declares,
            module,
            ty,
            span,
        )
    }
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

/// Maps a Soul type to its LLVM representation. Integers/`bool` map to the
/// matching `IntType`; `cstr` and any reference/pointer type map to an
/// (opaque, LLVM-16-style) pointer type; a struct maps to an LLVM struct
/// type with one field per declared field, in declared order (the same
/// order `mir_parser` uses for `Rvalue::Aggregate` operands and
/// `PlaceElem::Field` indices) — everything else (arrays, floats, ...) isn't
/// supported in this codegen slice yet.
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
        other => Err(Fault::error_with_kind(
            CodegenErrorKind::NonPrimitiveType {
                ty: format!("{other:?}").into_boxed_str(),
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
