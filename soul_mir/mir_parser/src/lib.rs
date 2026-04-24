use ast::AstModuleStore;
use hir::{ComplexLiteral, TypeId};
use hir_literal_interpreter::ToComplex;
use run_hir::HirResponse;
use soul_utils::{
    crate_store::CrateContext,
    error::SoulError,
    ids::{FunctionId, IdAlloc},
    sementic_level::SementicFault,
    soul_error_internal,
    span::{ModuleId, Span},
    vec_map::VecMap,
};

use typed_hir::ThirType;
pub(crate) use utils::*;
mod global;
mod id_generators;
pub mod mir;
mod parse;
mod utils;
use crate::{id_generators::IdGenerators, mir::MirTree};

pub fn mir_lower(
    hir_reponse: &HirResponse,
    ast_modules: &AstModuleStore,
    context: &mut CrateContext,
    root: ModuleId,
) -> MirTree {
    let mut context = MirContext::new(hir_reponse, ast_modules, context, root);

    for module_id in hir_reponse.hir.nodes.modules.keys() {
        context.lower_module(module_id);
    }

    context.lower_main_call();
    context.to_mir_tree()
}

struct MirContext<'a> {
    tree: MirTree,
    main: Option<FunctionId>,
    error_type: ThirType,
    current: CurrentContext,
    id_generators: IdGenerators,
    place_typed: VecMap<mir::PlaceId, TypeId>,
    local_remap: VecMap<hir::LocalId, mir::LocalId>,
    temp_remap: VecMap<hir::LocalId, mir::TempId>,

    hir_response: &'a HirResponse,
    context: &'a mut CrateContext,
    ast_modules: &'a AstModuleStore,
    root: ModuleId,
}

struct CurrentContext {
    module: ModuleId,
    scope: Vec<mir::LocalId>,
    function: FunctionId,
    block: Option<mir::BlockId>,

    loop_finish: Option<mir::BlockId>,
    loop_continue: Option<mir::BlockId>,
}
impl CurrentContext {
    pub fn new(function: FunctionId, module: ModuleId) -> Self {
        Self {
            module,
            function,
            block: None,
            scope: vec![],
            loop_finish: None,
            loop_continue: None,
        }
    }
}

impl<'a> MirContext<'a> {
    fn new(
        hir_reponse: &'a HirResponse,
        ast_modules: &'a AstModuleStore,
        context: &'a mut CrateContext,
        root: ModuleId,
    ) -> Self {
        let init_global_function = hir_reponse.hir.init_globals;
        let main = hir_reponse.hir.main;

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
            public_functions: vec![],
            init_global_function,
            root_module: root,

            temps: VecMap::const_default(),
            places: VecMap::const_default(),
            locals: VecMap::const_default(),
            modules: VecMap::const_default(),
            globals: VecMap::const_default(),
            functions: VecMap::const_default(),
            statements: VecMap::const_default(),
        };

        let mut this = Self {
            main,
            tree,
            context,
            hir_response: hir_reponse,
            error_type: ThirType {
                kind: typed_hir::ThirTypeKind::Error,
                generics: vec![],
                modifier: None,
            },
            ast_modules,
            id_generators: IdGenerators::new(),
            temp_remap: VecMap::const_default(),
            place_typed: VecMap::const_default(),
            local_remap: VecMap::const_default(),
            current: CurrentContext::new(init_global_function, root),
            root,
        };

        this.build_init_global_function();
        this
    }

    fn lower_module(&mut self, id: ModuleId) {
        let is_end = &mut false;

        let parent_module = self.current.module;
        self.current.module = id;

        let Some(module) = self.hir_response.hir.nodes.modules.get(id) else {
            self.log_error(soul_error_internal!(format!("{:?} not found", id), None));
            return;
        };

        let mut nodes = Vec::with_capacity(module.globals.len());
        for global in &module.globals {
            if let Some(id) = self.lower_global(global, module.is_public, is_end) {
                nodes.push(id);
            }

            if *is_end {
                break;
            }
        }

        let ast_module = &self.ast_modules[id];
        self.tree.modules.insert(
            id,
            mir::Module {
                id,
                nodes,
                parent: ast_module.parent,
                name: ast_module.name.clone(),
                modules: module.modules.clone(),
            },
        );

        self.current.module = parent_module;
    }

    fn lower_main_call(&mut self) {
        if self.main.is_none() || self.main == Some(FunctionId::error()) {
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
        self.context.faults.push(SementicFault::error(err));
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
        self.tree.locals.insert(id, mir::Local::Runtime { id, ty });

        self.current.scope.push(id);
        id
    }

    fn new_local(
        &mut self,
        local: hir::LocalId,
        ty: hir::TypeId,
        comptime: Option<ComplexLiteral>,
    ) -> mir::LocalId {
        let id = self.id_generators.alloc_local();

        self.local_remap.insert(local, id);

        let immutable = !self.id_to_type(ty).is_mutable();
        let local_kind = match comptime {
            Some(value) if immutable => mir::Local::Comptime { id, ty, value },
            _ => mir::Local::Runtime { id, ty },
        };

        self.tree.locals.insert(id, local_kind);

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
        self.tree.locals.insert(id, mir::Local::Runtime { id, ty });

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

    fn id_to_type(&mut self, ty: hir::TypeId) -> &ThirType {
        match self.hir_response.typed.types_map.id_to_type(ty) {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    format!("type {:?} not found in typeTable", ty),
                    None
                ));
                &self.error_type
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

    fn place_type(&self, id: hir::PlaceId) -> hir::TypeId {
        self.hir_response
            .typed
            .types_table
            .places
            .get(id)
            .copied()
            .unwrap_or(hir::TypeId::error())
    }

    fn local_type(&self, id: hir::LocalId) -> hir::TypeId {
        self.hir_response
            .typed
            .types_table
            .locals
            .get(id)
            .copied()
            .unwrap_or(hir::TypeId::error())
    }

    fn function_span(&self, id: FunctionId) -> Span {
        self.hir_response.hir.info.spans.functions[id]
    }

    fn function_type(&self, id: FunctionId) -> hir::TypeId {
        self.hir_response
            .typed
            .types_table
            .functions
            .get(id)
            .copied()
            .unwrap_or(hir::TypeId::error())
    }

    fn expression_span(&mut self, id: hir::ExpressionId) -> Span {
        self.hir_response.hir.info.spans.expressions[id]
    }

    fn expression_type(&self, id: hir::ExpressionId) -> hir::TypeId {
        self.hir_response
            .typed
            .types_table
            .expressions
            .get(id)
            .copied()
            .unwrap_or(hir::TypeId::error())
    }

    fn sizeof_type(&self, id: hir::ExpressionId) -> hir::TypeId {
        self.hir_response
            .typed
            .types_table
            .sizeofs
            .get(id)
            .copied()
            .unwrap_or(hir::TypeId::error())
    }

    fn get_expression_literal(&self, id: hir::ExpressionId) -> Option<ComplexLiteral> {
        match &self.hir_response.hir.nodes.expressions[id].kind {
            hir::ExpressionKind::Literal(literal) => Some(literal.clone().to_complex()),
            _ => self.hir_response.literal_resolves.get(id).cloned(),
        }
    }

    fn statement_span(&self, id: hir::StatementId) -> Span {
        self.hir_response.hir.info.spans.statements[id]
    }

    fn to_mir_tree(self) -> MirTree {
        self.tree
    }
}
