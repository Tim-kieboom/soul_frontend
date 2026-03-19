use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, VoidType};
use mir_parser::mir::FunctionBody;
use soul_utils::ids::FunctionId;

use crate::LlvmBackend;

impl<'a> LlvmBackend<'a> {
    pub(crate) fn declare_function(&mut self, function_id: FunctionId) {
        let function = &self.mir.tree.functions[function_id];

        let ty: FunctionReturnType<'a> = match self.lower_type(function.return_type) {
            Ok(Some(val)) => val.into(),
            Ok(None) => self.context.void_type().into(),
            Err(err) => {
                self.log_error(err);
                self.context.void_type().into()
            }
        };

        let mut args = Vec::with_capacity(function.parameters.len());
        for arg in &function.parameters {
            let local_type = self.mir.tree.locals[*arg].ty;
            let ty = match self.lower_type(local_type) {
                Ok(Some(val)) => val.into(),
                Ok(None) => self.context.i8_type().into(),
                Err(err) => {
                    self.log_error(err);
                    self.context.i8_type().into()
                }
            };
            args.push(ty);
        }

        let function_type = ty.fn_type(&args, false);
        let llvm_function = self
            .module
            .add_function(function.name.as_str(), function_type, None);

        self.create_block(function_id, llvm_function);
        self.functions.insert(function_id, llvm_function);
    }

    pub(crate) fn lower_function(&mut self, function_id: FunctionId) {
        self.current.function_id = function_id;
        let function = &self.mir.tree.functions[function_id];

        let blocks = match &function.body {
            FunctionBody::External(_) => return,
            FunctionBody::Internal { blocks, .. } => blocks,
        };

        self.allocate_function_locals(function);

        for block_id in blocks {
            let llvm_block = self.blocks[*block_id];
            self.builder.position_at_end(llvm_block);
            self.lower_statement(*block_id);

            if let Err(err) = self.lower_terminator(*block_id) {
                self.log_error(err);
            }
        }
    }
}

enum FunctionReturnType<'a> {
    Basic(BasicTypeEnum<'a>),
    Void(VoidType<'a>),
}
impl<'a> FunctionReturnType<'a> {
    fn fn_type(&self, args: &[BasicMetadataTypeEnum<'a>], varargs: bool) -> FunctionType<'a> {
        match self {
            FunctionReturnType::Void(ty) => ty.fn_type(args, varargs),
            FunctionReturnType::Basic(ty) => ty.fn_type(args, varargs),
        }
    }
}
impl<'a> From<VoidType<'a>> for FunctionReturnType<'a> {
    fn from(value: VoidType<'a>) -> Self {
        Self::Void(value)
    }
}
impl<'a> From<BasicTypeEnum<'a>> for FunctionReturnType<'a> {
    fn from(value: BasicTypeEnum<'a>) -> Self {
        Self::Basic(value)
    }
}
