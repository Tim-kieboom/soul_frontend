use inkwell::AddressSpace;

use crate::LlvmBackend;

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn initialize_internal_functions(&mut self) {
        self.declare_exit();
        self.declare_malloc();
        self.declare_free();
        self.declare_arraycmp();
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

        self.internal_functions.exit_function = Some(exit_fn);
    }

    fn declare_malloc(&mut self) {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
        let malloc_fn = self.module.add_function("malloc", malloc_type, None);
        malloc_fn.set_linkage(inkwell::module::Linkage::External);
        self.internal_functions.malloc_function = Some(malloc_fn);
    }

    fn declare_free(&mut self) {
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let free_type = self.context.i8_type().fn_type(&[ptr_type.into()], false);
        let free_fn = self.module.add_function("free", free_type, None);
        free_fn.set_linkage(inkwell::module::Linkage::External);
        self.internal_functions.free_function = Some(free_fn);
    }

    fn declare_arraycmp(&mut self) {
        let bool_type = self.context.bool_type();
        let u32_type = self.context.i32_type();
        let uint_type = self.default_int_type;
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        let memcmp_type = bool_type.fn_type(
            &[
                u32_type.into(),
                ptr_type.into(),
                uint_type.into(),
                ptr_type.into(),
                uint_type.into(),
            ],
            false,
        );
        let arraycmp_fn = self
            .module
            .add_function("__clib_arrayEqual", memcmp_type, None);
        arraycmp_fn.set_linkage(inkwell::module::Linkage::External);
        self.internal_functions.arraycmp_function = Some(arraycmp_fn);
    }
}
