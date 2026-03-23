use hir::{HirTree, RefTypeId, TypeId};
use hir_typed_context::HirTypedTable;
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdAlloc},
    sementic_level::SementicFault,
    soul_error_internal,
    vec_map::VecMap,
};

pub(crate) use utils::*;
mod global;
mod id_generators;
pub mod mir;
mod parse;
mod utils;
use crate::{id_generators::IdGenerators, mir::MirTree};

pub fn mir_lower(hir: &HirTree, types: &HirTypedTable, faults: &mut Vec<SementicFault>) -> MirTree {
    let mut context = MirContext::new(hir, types, faults);
    let is_end = &mut false;

    for global in &hir.root.globals {
        context.lower_global(global, is_end);
        if *is_end {
            break;
        }
    }

    context.lower_main_call();
    context.to_mir_tree()
}

struct MirContext<'a> {
    tree: MirTree,
    main: FunctionId,

    current: CurrentContext,
    id_generators: IdGenerators,
    place_typed: VecMap<mir::PlaceId, TypeId>,
    local_remap: VecMap<hir::LocalId, mir::LocalId>,
    temp_remap: VecMap<hir::LocalId, mir::TempId>,

    hir: &'a HirTree,
    types: &'a HirTypedTable,
    faults: &'a mut Vec<SementicFault>,
}

struct CurrentContext {
    scope: Vec<mir::LocalId>,
    function: FunctionId,
    block: Option<mir::BlockId>,

    loop_finish: Option<mir::BlockId>,
    loop_continue: Option<mir::BlockId>,
}
impl CurrentContext {
    pub fn new(function: FunctionId) -> Self {
        Self {
            function,
            block: None,
            scope: vec![],
            loop_finish: None,
            loop_continue: None,
        }
    }
}

impl<'a> MirContext<'a> {
    fn new(hir: &'a HirTree, types: &'a HirTypedTable, faults: &'a mut Vec<SementicFault>) -> Self {
        let init_global_function = hir.init_global_function;
        let main = hir
            .functions
            .values()
            .find(|func| func.name.as_str() == "main")
            .map(|f| f.id)
            .unwrap_or(FunctionId::error());

        if main == FunctionId::error() {
            faults.push(SementicFault::error(SoulError::new(
                "'main' function not found",
                SoulErrorKind::InvalidContext,
                None,
            )));
        }

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
            blocks,
            entry_function: main,
            init_global_function,
            temps: VecMap::const_default(),
            places: VecMap::const_default(),
            locals: VecMap::const_default(),
            globals: VecMap::const_default(),
            functions: VecMap::const_default(),
            statements: VecMap::const_default(),
        };

        let mut this = Self {
            hir,
            main,
            tree,
            types,
            faults,
            id_generators: IdGenerators::new(),
            temp_remap: VecMap::const_default(),
            place_typed: VecMap::const_default(),
            local_remap: VecMap::const_default(),
            current: CurrentContext::new(init_global_function),
        };

        this.build_init_global_function();
        this
    }

    fn lower_main_call(&mut self) {
        if self.main == FunctionId::error() {
            return;
        }

        self.lower_main_function();

        self.current.function = self.tree.init_global_function;
        self.current.block = Some(self.expect_init_global_block());

        let end_block = match self.current.block {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "self.current.block should be Some(_)",
                    None
                ));
                mir::BlockId::error()
            }
        };
        self.insert_terminator(end_block, mir::Terminator::Return(None));
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

        let function = self
            .tree
            .functions
            .get_mut(self.current.function)
            .expect("should have id");

        match &mut function.body {
            mir::FunctionBody::External(_) => panic!("should be internal function"),
            mir::FunctionBody::Internal { blocks, .. } => {
                blocks.push(id);
            }
        };

        self.tree.blocks.insert(id, block);
        id
    }

    fn new_parameter(&mut self, local: hir::LocalId, ty: hir::TypeId) -> mir::LocalId {
        let id = self.id_generators.alloc_local();

        self.local_remap.insert(local, id);
        self.tree.locals.insert(id, mir::Local { id, ty });

        self.current.scope.push(id);
        id
    }

    fn new_local(&mut self, local: hir::LocalId, ty: hir::TypeId) -> mir::LocalId {
        let id = self.id_generators.alloc_local();

        self.local_remap.insert(local, id);
        self.tree.locals.insert(id, mir::Local { id, ty });

        self.current.scope.push(id);

        match &mut self.tree.functions[self.current.function].body {
            mir::FunctionBody::External(_) => return id,
            mir::FunctionBody::Internal { locals, .. } => locals.push(id),
        };

        id
    }

    fn new_local_global(&mut self, local: hir::LocalId, ty: hir::TypeId) -> mir::LocalId {
        let id = self.id_generators.alloc_local();

        self.local_remap.insert(local, id);
        self.tree.locals.insert(id, mir::Local { id, ty });

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
        self.push_statement_from(statement, block_id)
    }

    fn push_statement_from(
        &mut self,
        statement: mir::Statement,
        block_id: mir::BlockId,
    ) -> mir::StatementId {
        let id = self.id_generators.alloc_statement();
        self.tree.statements.insert(id, statement);
        self.tree.blocks[block_id].statements.push(id);
        id
    }

    fn insert_terminator(&mut self, block: mir::BlockId, terminator: mir::Terminator) {
        self.tree.blocks[block].terminator = terminator;
    }

    fn id_to_type(&mut self, ty: hir::TypeId) -> &hir::HirType {
        const ERROR: hir::HirType = hir::HirType::error_type();

        match self.types.types.id_to_type(ty) {
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

    fn ref_to_id(&mut self, ref_id: RefTypeId) -> TypeId {
        match self.types.types.ref_to_id(ref_id) {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    format!("type {:?} not found in typeTable", ref_id),
                    None
                ));
                TypeId::error()
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

    fn expect_init_global_block(&mut self) -> mir::BlockId {
        let start = self.tree.init_global_function;
        let block = self.tree.functions.get(start).map(|func| match &func.body {
            mir::FunctionBody::External(_) => panic!("should be internal function"),
            mir::FunctionBody::Internal { blocks, .. } => blocks.get(0),
        });

        match block {
            Some(Some(val)) => *val,
            _ => {
                self.log_error(soul_error_internal!(
                    "expected _init_globals function to have block",
                    None
                ));
                mir::BlockId::error()
            }
        }
    }

    fn expression_ty(&self, id: hir::ExpressionId) -> hir::TypeId {
        self.types
            .expressions
            .get(id)
            .copied()
            .unwrap_or(hir::TypeId::error())
    }

    fn to_mir_tree(self) -> MirTree {
        self.tree
    }
}
