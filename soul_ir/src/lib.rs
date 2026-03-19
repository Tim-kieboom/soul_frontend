use hir::{HirType, TypeId};
use hir_typed_context::HirTypedTable;
use inkwell::{
    basic_block::BasicBlock,
    builder::{Builder, BuilderError},
    context::Context,
    module::Module,
    types::IntType,
    values::{BasicValueEnum, FunctionValue, PointerValue},
};
use mir_parser::mir::{BlockId, LocalId, TempId};
use run_mir::MirResponse;
use soul_utils::{
    compile_options::CompilerOptions,
    error::{SoulError, SoulResult},
    ids::{FunctionId, IdAlloc},
    sementic_level::SementicFault,
    soul_error_internal,
    vec_map::VecMap,
};

mod block;
mod function;
mod ir_type;
mod local;
mod value;
mod statement;

pub struct IrRequest<'ctx> {
    pub context: &'ctx Context,
    pub mir: &'ctx MirResponse,
    pub types: &'ctx HirTypedTable,
}
impl<'ctx> IrRequest<'ctx> {
    pub fn new(mir: &'ctx MirResponse, types: &'ctx HirTypedTable, context: &'ctx Context) -> Self {
        Self {
            mir,
            types,
            context,
        }
    }
}

pub struct IrResponse<'a> {
    pub module: Module<'a>,
    pub no_errors: bool,
}

pub fn to_llvm_ir<'a>(
    request: &'a IrRequest<'a>,
    _options: &CompilerOptions,
    faults: &'a mut Vec<SementicFault>,
) -> IrResponse<'a> {
    let mut backend = LlvmBackend::new(request, faults);

    backend.declare_exit();
    let mir = &request.mir.tree;
    for function_id in mir.functions.keys() {
        backend.declare_function(function_id);
    }

    backend.allocate_globals();
    for function_id in mir.functions.keys() {
        backend.lower_function(function_id);
    }

    backend.to_ir_reponse()
}

#[derive(Debug, Clone, Copy)]
pub struct IrOperand<'a> {
    pub value: BasicValueEnum<'a>,
    pub is_signed_interger: bool,
}

pub struct LlvmBackend<'a> {
    default_int_size: u8,
    default_char_size: u8,
    default_int_type: IntType<'a>,
    default_char_type: IntType<'a>,

    current: Current,
    module: Module<'a>,
    context: &'a Context,
    mir: &'a MirResponse,
    builder: Builder<'a>,
    types: &'a HirTypedTable,
    exit_function: Option<FunctionValue<'a>>,

    temps: VecMap<TempId, IrOperand<'a>>,
    blocks: VecMap<BlockId, BasicBlock<'a>>,
    locals: VecMap<LocalId, PointerValue<'a>>,
    functions: VecMap<FunctionId, FunctionValue<'a>>,

    faults: &'a mut Vec<SementicFault>,
}
impl<'a> LlvmBackend<'a> {
    pub fn new(request: &'a IrRequest<'a>, faults: &'a mut Vec<SementicFault>) -> Self {
        let module = request.context.create_module("main");
        let builder = request.context.create_builder();

        Self {
            faults,
            module,
            builder,
            mir: request.mir,
            exit_function: None,
            types: request.types,
            context: request.context,
            current: Current::start(),
            temps: VecMap::const_default(),
            blocks: VecMap::const_default(),
            locals: VecMap::const_default(),
            functions: VecMap::const_default(),

            default_int_size: 32,
            default_char_size: 8,
            default_char_type: request.context.i8_type(),
            default_int_type: request.context.i32_type(),
        }
    }

    fn declare_exit(&mut self) {
        let void_type = self.context.void_type();
        let i32_type = self.context.i32_type();
        let exit_type = void_type.fn_type(&[i32_type.into()], false);
        let exit_fn = self.module.add_function("exit", exit_type, None);

        exit_fn.set_linkage(inkwell::module::Linkage::External);

        // Use raw enum ID 39 for noreturn (LLVM 16)
        let noreturn_attr = self.context.create_enum_attribute(39, 0);
        exit_fn.add_attribute(inkwell::attributes::AttributeLoc::Function, noreturn_attr);

        self.exit_function = Some(exit_fn);
    }

    fn get_type(&self, ty: TypeId) -> SoulResult<&HirType> {
        self.types
            .types
            .id_to_type(ty)
            .ok_or(soul_error_internal!(format!("{:?} not found", ty), None))
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }

    fn to_ir_reponse(self) -> IrResponse<'a> {
        IrResponse {
            module: self.module,
            no_errors: self.faults.is_empty(),
        }
    }
}

/// From [`BuilderError`] to [`SoulError`] of [`soul_utils::error::SoulErrorKind::LlvmError`]
fn build_error(value: BuilderError) -> SoulError {
    SoulError::new(
        value.to_string(),
        soul_utils::error::SoulErrorKind::LlvmError,
        None,
    )
}

pub struct Current {
    function_id: FunctionId,
}
impl Current {
    pub fn start() -> Self {
        Self {
            function_id: FunctionId::error(),
        }
    }
}
