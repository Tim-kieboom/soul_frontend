use hir::{GenericId, TypeId};
use mir_parser::mir;
use soul_utils::{
    ids::{FunctionId, IdAlloc},
    vec_map::VecMap,
};

use crate::FunctionKeyId;

#[derive(Debug, Clone, Copy)]
pub struct Current {
    function_key: FunctionKeyId,
    block: mir::BlockId,
}
impl Current {
    pub fn start(global: FunctionKeyId) -> Self {
        Self {
            function_key: global,
            block: mir::BlockId::error(),
        }
    }

    pub fn function_key(&self) -> FunctionKeyId {
        self.function_key
    }

    pub fn set_function_key(&mut self, id: FunctionKeyId) {
        self.function_key = id
    }

    pub fn block(&self) -> mir::BlockId {
        self.block
    }

    pub fn set_block(&mut self, id: mir::BlockId) {
        self.block = id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionKey {
    id: FunctionId,
    type_args: Vec<TypeId>,
}
impl FunctionKey {
    pub fn new(id: FunctionId, type_args: Vec<TypeId>) -> Self {
        Self { id, type_args }
    }

    pub fn function_id(&self) -> FunctionId {
        self.id
    }

    pub fn type_args(&self) -> &Vec<TypeId> {
        &self.type_args
    }
}

pub struct GenericSubstitute {
    store: VecMap<GenericId, TypeId>,
}
impl GenericSubstitute {
    pub fn new(generics: &[GenericId], type_args: &[TypeId]) -> Self {
        let mut substitude = Self {
            store: VecMap::const_default(),
        };

        for (generic, ty) in generics.iter().zip(type_args.iter()) {
            substitude.insert(*generic, *ty);
        }

        substitude
    }

    pub fn insert(&mut self, id: GenericId, ty: TypeId) {
        self.store.insert(id, ty);
    }

    pub fn resolve(&self, id: GenericId) -> Option<TypeId> {
        self.store.get(id).copied()
    }
}
