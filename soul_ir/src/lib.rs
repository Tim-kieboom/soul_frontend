use std::{cell::RefCell, collections::HashMap};

use hir::{FieldId, TypeId};
use inkwell::{
    basic_block::BasicBlock,
    context::Context,
    module::Module,
    types::{BasicTypeEnum, IntType},
    values::{BasicValueEnum, FunctionValue, PointerValue},
};
use mir_parser::mir::{BlockId, LocalId, TempId};
use run_mir::MirResponse;
use soul_utils::{
    compile_options::CompilerOptions,
    error::{SoulError, SoulResult},
    ids::{FunctionId, IdGenerator},
    impl_soul_ids,
    sementic_level::SementicFault,
    soul_error_internal,
    vec_map::VecMap,
};

mod block;
mod function;
mod ir_type;
mod llvm_builder;
mod local;
mod statement;
mod utils;
mod value;
use typed_hir::{ThirType, TypedHir};
use utils::*;

use crate::llvm_builder::IrBuilder;

pub struct IrRequest<'ctx> {
    pub context: &'ctx Context,
    pub mir: &'ctx MirResponse,
    pub types: &'ctx TypedHir,
}
impl<'ctx> IrRequest<'ctx> {
    pub fn new(mir: &'ctx MirResponse, types: &'ctx TypedHir, context: &'ctx Context) -> Self {
        Self {
            mir,
            types,
            context,
        }
    }
}

pub struct IrResponse<'a> {
    pub module: Module<'a>,
    pub is_fatal: bool,
}

pub fn to_llvm_ir<'f, 'a>(
    request: &'a IrRequest<'a>,
    options: &'a CompilerOptions,
    faults: &'f mut Vec<SementicFault>,
) -> IrResponse<'a> {
    let mut backend = LlvmBackend::new(request, options, faults);

    backend.declare_exit();
    backend.allocate_globals(&GenericSubstitute::new(&[], &[]));

    let entry = request.mir.tree.entry_function;
    backend.get_or_create_function(entry, &vec![]);

    backend.to_ir_reponse()
}

#[derive(Debug, Clone, Copy)]
pub struct IrOperand<'a> {
    pub value: BasicValueEnum<'a>,
    pub info: OperandInfo<'a>,
}
impl<'a> IrOperand<'a> {
    pub fn get_or_convert_pointer(self, builder: &IrBuilder<'a>) -> SoulResult<PointerValue<'a>> {
        if self.value.is_pointer_value() {
            Ok(self.value.into_pointer_value())
        } else {
            let pointee_ty = self.value.get_type();
            let alloca_ptr = builder.build_alloca(pointee_ty, "temp_base_ptr")?;
            builder.store_operand(alloca_ptr, self)?;
            Ok(alloca_ptr)
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct OperandInfo<'a> {
    pub is_unloaded: bool,
    pub type_id: TypeId,
    pub ir_type: BasicTypeEnum<'a>,
}
impl<'a> OperandInfo<'a> {
    fn new_loaded(type_id: TypeId, ir_type: BasicTypeEnum<'a>) -> Self {
        Self {
            is_unloaded: false,
            ir_type,
            type_id,
        }
    }

    fn new_unloaded(type_id: TypeId, ir_type: BasicTypeEnum<'a>) -> Self {
        Self {
            is_unloaded: true,
            ir_type,
            type_id,
        }
    }
}

impl_soul_ids!(FunctionKeyId);

pub struct LlvmBackend<'f, 'a> {
    default_ptr_size: u8,
    default_int_size: u8,
    default_char_size: u8,
    default_c_int_size: u8,
    default_int_type: IntType<'a>,
    default_char_type: IntType<'a>,
    default_c_int_type: IntType<'a>,

    types: TypedHir,
    current: Current,
    module: Module<'a>,
    context: &'a Context,
    mir: &'a MirResponse,
    builder: IrBuilder<'a>,
    options: &'a CompilerOptions,
    exit_function: Option<FunctionValue<'a>>,

    temps: HashMap<(FunctionKeyId, TempId), IrOperand<'a>>,
    locals: HashMap<(FunctionKeyId, LocalId), Local<'a>>,
    blocks: HashMap<(FunctionKeyId, BlockId), BasicBlock<'a>>,

    function_keys: FunctionKeyStore,
    field_indexs: RefCell<VecMap<FieldId, usize>>,
    structs: StructStore<'a>,
    functions: VecMap<FunctionKeyId, FunctionValue<'a>>,

    faults: &'f mut Vec<SementicFault>,
}

#[derive(Debug, Clone, Copy)]
pub enum Local<'a> {
    Runtime(PointerValue<'a>),
    Comptime(IrOperand<'a>),
}

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub fn new(
        request: &'a IrRequest<'a>,
        options: &'a CompilerOptions,
        faults: &'f mut Vec<SementicFault>,
    ) -> Self {
        fn to_int_type<'a>(context: &'a Context, size: u8) -> IntType<'a> {
            match size {
                8 => context.i8_type(),
                16 => context.i16_type(),
                32 => context.i32_type(),
                64 => context.i64_type(),
                _ => unreachable!(),
            }
        }

        let module = request.context.create_module("main");
        let builder = IrBuilder::new(request.context);
        let function_keys = FunctionKeyStore::new();
        let default_char_size = options.target_info().char_bit_size;
        let default_int_size = options.target_info().int_bit_size;
        let default_c_int_size = options.target_info().c_int_bit_size;
        let default_int_type = to_int_type(&request.context, default_int_size);
        let default_char_type = to_int_type(&request.context, default_char_size);
        let default_c_int_type = to_int_type(&request.context, default_c_int_size);

        Self {
            faults,
            module,
            builder,
            options,
            mir: request.mir,
            exit_function: None,
            temps: HashMap::new(),
            blocks: HashMap::new(),
            locals: HashMap::new(),
            functions: VecMap::new(),
            context: request.context,
            structs: StructStore::new(),
            types: request.types.clone(),
            field_indexs: RefCell::new(VecMap::const_default()),
            current: Current::start(function_keys.global_key()),

            function_keys,
            default_int_size,
            default_char_size,
            default_int_type,
            default_char_type,
            default_c_int_size,
            default_c_int_type,
            default_ptr_size: options.target_info().ptr_bit_size,
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

    fn get_or_create_function(
        &mut self,
        function_id: FunctionId,
        type_args: &Vec<TypeId>,
    ) -> FunctionValue<'a> {
        let key = FunctionKey::new(function_id, type_args.clone());
        let key_id = self.function_keys.insert(key);
        let prev = self.current;
        self.current.set_function_key(key_id);

        if let Some(function) = self.functions.get(key_id) {
            return *function;
        }

        let function = &self.mir.tree.functions[function_id];
        let generics = GenericSubstitute::new(&function.generics, type_args);
        let llvm_fn = self.declare_function_instance(function_id, type_args, &generics);

        self.functions.insert(key_id, llvm_fn);
        self.lower_function_instance(function_id, key_id, type_args, &generics);

        self.current = prev;
        if self.is_bodied_function(self.current.function_key()) {
            let block = self.get_block(self.current.block());
            self.builder.position_at_end(block);
        }
        llvm_fn
    }

    fn get_local(&self, id: LocalId) -> Local<'a> {
        match self.locals.get(&(self.current.function_key(), id)) {
            Some(local) => *local,
            _ => {
                let global = self.function_keys.global_key();
                self.locals[&(global, id)]
            }
        }
    }

    fn is_bodied_function(&self, id: FunctionKeyId) -> bool {
        if id == self.function_keys.global_key() {
            return false;
        }

        let key = match self.function_keys.id_to_key(id) {
            Some(val) => val,
            None => panic!("should have {:?}", id),
        };
        let function = &self.mir.tree.functions[key.function_id()];
        function.body.is_internal()
    }

    fn get_block(&self, id: BlockId) -> BasicBlock<'a> {
        self.blocks[&(self.current.function_key(), id)]
    }

    fn get_temp(&self, id: TempId) -> SoulResult<IrOperand<'a>> {
        self.temps
            .get(&(self.current.function_key(), id))
            .copied()
            .ok_or(soul_error_internal!(
                format!("{:?} not found current: {:?}", id, self.current),
                None
            ))
    }

    fn push_global(&mut self, id: LocalId, value: PointerValue<'a>) {
        self.locals
            .insert((self.function_keys.global_key(), id), Local::Runtime(value));
    }

    fn push_local(&mut self, id: LocalId, value: Local<'a>) {
        self.locals.insert((self.current.function_key(), id), value);
    }

    fn push_block(&mut self, id: BlockId, value: BasicBlock<'a>) {
        self.blocks.insert((self.current.function_key(), id), value);
    }

    fn push_temp(&mut self, id: TempId, value: IrOperand<'a>) {
        self.temps.insert((self.current.function_key(), id), value);
    }

    fn get_type(&self, ty: TypeId) -> SoulResult<&ThirType> {
        self.types
            .types_map
            .id_to_type(ty)
            .ok_or(soul_error_internal!(format!("{:?} not found", ty), None))
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }

    fn to_ir_reponse(self) -> IrResponse<'a> {
        IrResponse {
            module: self.module,
            is_fatal: self
                .faults
                .iter()
                .find(|fault| fault.is_fatal(self.options.fatal_level()))
                .is_some(),
        }
    }
}

pub struct FunctionKeyStore {
    to_id: HashMap<FunctionKey, FunctionKeyId>,
    to_key: VecMap<FunctionKeyId, FunctionKey>,
    alloc: IdGenerator<FunctionKeyId>,
    global_key: FunctionKeyId,
}
impl FunctionKeyStore {
    pub fn new() -> Self {
        let mut alloc = IdGenerator::new();
        Self {
            to_id: HashMap::new(),
            to_key: VecMap::new(),
            global_key: alloc.alloc(),
            alloc,
        }
    }

    pub fn global_key(&self) -> FunctionKeyId {
        self.global_key
    }

    pub fn insert(&mut self, key: FunctionKey) -> FunctionKeyId {
        if let Some(id) = self.to_id.get(&key).copied() {
            return id;
        }

        let id = self.alloc.alloc();
        self.to_key.insert(id, key.clone());
        self.to_id.insert(key, id);
        id
    }

    pub fn id_to_key(&self, id: FunctionKeyId) -> Option<&FunctionKey> {
        self.to_key.get(id)
    }

    pub fn key_to_id(&self, key: &FunctionKey) -> Option<FunctionKeyId> {
        self.to_id.get(key).copied()
    }
}
