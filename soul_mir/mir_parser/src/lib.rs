
use hir::{HirTree, IdAlloc};
use hir_typed_context::HirTypedTable;
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    sementic_level::SementicFault,
    soul_error_internal,
    vec_map::VecMap,
};

pub(crate) use utils::*;
mod utils;
mod id_generators;
pub mod mir;
mod parse;
use crate::{id_generators::IdGenerators, mir::MirTree};

pub fn mir_lower(hir: &HirTree, types: &HirTypedTable, faults: &mut Vec<SementicFault>) -> MirTree {
    let mut context = MirContext::new(hir, types, faults);

    for function in hir.functions.keys() {
        context.lower_function(function)
    }

    context.to_mir_tree()
}

struct MirContext<'a> {
    tree: MirTree,

    current: CurrentContext,
    id_generators: IdGenerators,
    local_remap: VecMap<hir::LocalId, mir::LocalId>,

    hir: &'a HirTree,
    types: &'a HirTypedTable,
    faults: &'a mut Vec<SementicFault>,
}

struct CurrentContext {
    scope: Vec<mir::LocalId>,
    function: hir::FunctionId,
    block: Option<mir::BlockId>,
}
impl CurrentContext {
    pub fn new(function: hir::FunctionId) -> Self {
        Self {
            scope: vec![],
            function,
            block: None,
        }
    }
}

impl<'a> MirContext<'a> {
    fn new(hir: &'a HirTree, types: &'a HirTypedTable, faults: &'a mut Vec<SementicFault>) -> Self {
        let main = hir
            .functions
            .values()
            .find(|func| func.name.as_str() == "main")
            .map(|func| func.id)
            .unwrap_or_else(|| {
                faults.push(SementicFault::error(SoulError::new(
                    "'main' function not found",
                    SoulErrorKind::InvalidContext,
                    None,
                )));
                hir::FunctionId::error()
            });

        let mut blocks = VecMap::const_default();
        blocks.insert(
            mir::BlockId::error(),
            mir::Block {
                id: mir::BlockId::error(),
                returnable: false,
                statements: vec![],
                terminator: mir::Terminator::Unreachable,
            },
        );

        let tree = MirTree {
            main,

            blocks,
            temps: VecMap::const_default(),
            places: VecMap::const_default(),
            locals: VecMap::const_default(),
            functions: VecMap::const_default(),
            statements: VecMap::const_default(),
        };

        Self {
            hir,
            tree,
            types,
            faults,
            id_generators: IdGenerators::new(),
            local_remap: VecMap::const_default(),
            current: CurrentContext::new(main),
        }
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }

    fn new_function_block(&mut self) -> mir::BlockId {
        let id = self.id_generators.alloc_block();
        let block = mir::Block {
            id,
            returnable: true,
            statements: vec![],
            terminator: mir::Terminator::Unreachable,
        };

        self.tree.blocks.insert(id, block);
        id
    }

    fn new_block(&mut self) -> mir::BlockId {
        let id = self.id_generators.alloc_block();
        let block = mir::Block {
            id,
            returnable: false,
            statements: vec![],
            terminator: mir::Terminator::Unreachable,
        };

        self.tree
            .functions
            .get_mut(self.current.function)
            .expect("should have id")
            .blocks
            .push(id);

        self.tree.blocks.insert(id, block);
        id
    }

    fn new_local(&mut self, local: hir::LocalId, ty: hir::TypeId) -> mir::LocalId {
        let id = self.id_generators.alloc_local();

        self.local_remap.insert(local, id);
        self.tree.locals.insert(id, mir::Local { id, ty });
        self.tree.functions[self.current.function].locals.push(id);

        self.current.scope.push(id);
        id
    }

    fn new_temp(&mut self, ty: hir::TypeId) -> mir::TempId {
        let id = self.id_generators.alloc_temp();
        self.tree.temps.insert(id, ty);
        id
    }

    fn new_place(&mut self, place: mir::Place) -> mir::PlaceId {
        let id = self.id_generators.alloc_place();
        self.tree.places.insert(id, place);
        id
    }

    fn push_statement(&mut self, statement: mir::Statement) -> mir::StatementId {
        let block_id = self.expect_current_block();

        let id = self.id_generators.alloc_statement();
        self.tree.statements.insert(id, statement);
        self.tree.blocks[block_id].statements.push(id);
        id
    }

    fn insert_terminator(&mut self, block: mir::BlockId, terminator: mir::Terminator) {
        self.tree.blocks[block].terminator = terminator;
    }

    fn get_type(&mut self, ty: hir::TypeId) -> &hir::HirType {
        const ERROR: hir::HirType = hir::HirType::error_type();

        match self.types.types.get_type(ty) {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    format!("type {:?} not found in typeTable", ty),
                    None
                ));
                &ERROR
            }
        }
    }

    fn expect_current_block(&mut self) -> mir::BlockId {
        match self.current.block {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "expected current_block to be Some(_)",
                    None
                ));
                mir::BlockId::error()
            }
        }
    }

    fn to_mir_tree(self) -> MirTree {
        self.tree
    }
}
