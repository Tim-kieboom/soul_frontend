use std::fmt::Write;

use hir::TypeId;
use inkwell::{
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, VoidType},
    values::FunctionValue,
};
use mir_parser::mir::{self, FunctionBody};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::FunctionId,
    vec_map::VecMapIndex,
};
use typed_hir::{ThirType, ThirTypeKind, display_thir::DisplayThirType};

use crate::{FunctionKeyId, GenericSubstitute, LlvmBackend};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn declare_function_instance(
        &mut self,
        function_id: FunctionId,
        type_args: &Vec<TypeId>,
        generics: &GenericSubstitute,
    ) -> FunctionValue<'a> {
        let function = &self.mir.tree.functions[function_id];

        let return_type: FunctionReturnType<'a> =
            match self.lower_type(function.return_type, generics) {
                Ok(Some(val)) => val.into(),
                Ok(None) => self.context.void_type().into(),
                Err(err) => {
                    self.log_error(err);
                    self.context.void_type().into()
                }
            };

        let mut args = vec![];
        for param in &function.parameters {
            let ty = self.mir.tree.locals[*param].ty();
            let arg_type = match self.lower_type(ty, generics) {
                Ok(Some(val)) => val.into(),
                Ok(None) => self.context.i8_type().into(),
                Err(err) => {
                    self.log_error(err);
                    self.context.i8_type().into()
                }
            };

            args.push(arg_type);
        }

        let function_type = return_type.fn_type(&args, false);

        let name = if function.body.is_internal() {
            &self.mangle(&function.name, function.owner_type, function_id, type_args)
        } else {
            self.check_non_mangle(function);
            function.name.as_str()
        };
        let llvm_function = self.module.add_function(name, function_type, None);

        self.create_block(function_id, llvm_function);
        llvm_function
    }

    pub(crate) fn lower_function_instance(
        &mut self,
        function_id: FunctionId,
        function_key: FunctionKeyId,
        type_args: &Vec<TypeId>,
        generics: &GenericSubstitute,
    ) {
        self.current.set_function_key(function_key);
        let function = &self.mir.tree.functions[function_id];
        let blocks = match &function.body {
            FunctionBody::External(_) => return,
            FunctionBody::Internal { blocks, .. } => blocks,
        };

        self.allocate_function_locals(function, type_args, generics);

        for block_id in blocks {
            let llvm_block = self.get_block(*block_id);
            self.current.set_block(*block_id);
            self.builder.position_at_end(llvm_block);

            self.lower_block(*block_id, generics);
            if let Err(err) = self.lower_terminator(*block_id, generics) {
                self.log_error(err);
            }
        }
    }

    pub(crate) fn mangle(
        &mut self,
        name: &Ident,
        owner: TypeId,
        id: FunctionId,
        type_args: &Vec<TypeId>,
    ) -> String {
        const SEPARATOR: &str = "_";
        if name.as_str() == "main" {
            return name.to_string();
        }

        let mut sb = String::new();
        sb.write_fmt(format_args!("F{}___", id.index()))
            .expect("no fmt error");

        sb.push_str(name.as_str());
        let owner_type = match self.get_type(owner) {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                &ThirType {
                    kind: ThirTypeKind::None,
                    generics: vec![],
                    modifier: None,
                }
            }
        };

        if owner_type.kind != ThirTypeKind::None {
            sb.push_str("___t_");
            owner_type
                .write_display_no_spaces(&self.types.types_map, &mut sb)
                .expect("expect not fmt error");
        }
        if !type_args.is_empty() {
            sb.push_str("___g");
        }
        for ty in type_args {
            sb.push_str(SEPARATOR);
            match self.get_type(*ty) {
                Ok(ty) => ty
                    .write_display_no_spaces(&self.types.types_map, &mut sb)
                    .expect("expect not fmt error"),
                Err(err) => {
                    self.log_error(err);
                    sb.push_str("error");
                }
            };
        }

        sb
    }

    fn check_non_mangle(&mut self, function: &mir::Function) {
        let id = match self.non_mangels.get(&function.name.node) {
            Some(val) => val,
            None => {
                self.non_mangels
                    .insert(function.name.to_string(), function.id);
                return;
            }
        };

        if *id != function.id {
            self.log_error(SoulError::new(
                format!(
                    "function: '{}' is not mangeled and name already exists somewhere in crate",
                    function.name.as_str(),
                ),
                SoulErrorKind::AlreadyFoundInScope,
                Some(function.name.span),
            ));
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
