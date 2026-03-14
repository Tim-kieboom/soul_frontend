use hir::{HirType, TypeId};
use hir_typed_context::HirTypedTable;
use inkwell::{
    OptimizationLevel,
    basic_block::BasicBlock,
    builder::{Builder, BuilderError},
    context::Context,
    module::Module,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetData, TargetMachine},
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

mod local;
mod block;
mod function;
mod ir_type;
mod rvalue;
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
    current: Current,
    module: Module<'a>,
    context: &'a Context,
    mir: &'a MirResponse,
    builder: Builder<'a>,
    target_data: TargetData,
    types: &'a HirTypedTable,

    functions: VecMap<FunctionId, FunctionValue<'a>>,
    blocks: VecMap<BlockId, BasicBlock<'a>>,
    locals: VecMap<LocalId, PointerValue<'a>>,
    temps: VecMap<TempId, IrOperand<'a>>,

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
            types: request.types,
            context: request.context,
            current: Current::start(),
            target_data: get_target_data(),
            temps: VecMap::const_default(),
            blocks: VecMap::const_default(),
            locals: VecMap::const_default(),
            functions: VecMap::const_default(),
        }
    }

    fn get_type(&self, ty: TypeId) -> SoulResult<&HirType> {
        self.types
            .types
            .get_type(ty)
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

fn get_target_data() -> TargetData {
    Target::initialize_native(&InitializationConfig::default()).unwrap();

    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).unwrap();

    let target_machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap();

    target_machine.get_target_data()
}

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
