
use hir::{HirTree, TypeId};
use hir_typed_context::HirTypedTable;
use soul_utils::{
    Ident, error::{SoulError, SoulErrorKind}, ids::{FunctionId, IdAlloc}, sementic_level::SementicFault, soul_error_internal, span::Span, vec_map::VecMap
};

pub(crate) use utils::*;
mod utils;
mod id_generators;
pub mod mir;
mod parse;
use crate::{id_generators::IdGenerators, mir::MirTree};

pub fn mir_lower(hir: &HirTree, types: &HirTypedTable, faults: &mut Vec<SementicFault>) -> MirTree {
    let mut context = MirContext::new(hir, types, faults);
    let is_end = &mut false;

    for global in &hir.root.globals {
        context.lower_global(global, is_end);
        if *is_end {
            break
        }
    }

    context.insert_main_call();
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
}
impl CurrentContext {
    pub fn new(function: FunctionId) -> Self {
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
            start_function: hir.start_function,
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
            current: CurrentContext::new(hir.start_function),
        };

        this.build_start_function();
        this
    }

    fn insert_main_call(&mut self) {
        if self.main == FunctionId::error() {
            return
        }

        self.current.function = self.tree.start_function;
        self.current.block = Some(self.expect_start_block());

        let span = self.hir.spans.functions[self.main];
        self.lower_call(self.main, &None, &vec![], self.types.none_type, span);
        let end_block = match self.current.block {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!("self.current.block should be Some(_)", None));
                mir::BlockId::error()
            }
        };
        self.insert_terminator(end_block, mir::Terminator::Exit);
    }

    fn build_start_function(&mut self) {
        let block = self.new_function_block();
        let start = mir::Function {
            id: self.tree.start_function,
            name: Ident::new("_start".to_string(), Span::default_const()),
            parameters: vec![],
            locals: vec![],
            blocks: vec![block],
            return_type: TypeId::error(),
        };

        self.tree.blocks.insert(block, mir::Block{
            id: block,
            returnable: false,
            terminator: mir::Terminator::Unreachable,
            statements: vec![],
        });
        self.tree.functions.insert(self.tree.start_function, start);
    }

    fn lower_global(&mut self, global: &hir::Global, is_end: &mut bool) {
        match global {
            hir::Global::Function(function,_) => self.lower_function(*function),
            
            hir::Global::Variable(variable,_) 
            | hir::Global::InternalVariable(variable,_) => {
                let local = if variable.is_temp {
                    mir::Place::Temp(self.new_temp(self.types.locals[variable.local]))
                } else {
                    mir::Place::Local(self.lower_global_variable(variable))
                };
                let value = match variable.value {
                    Some(val) => val,
                    None => return,
                };
                
                self.current.function = self.tree.start_function;
                self.current.block = Some(self.expect_start_block());

                let place = self.new_place(local);
                let value = self.lower_operand(value).pass(is_end);
                self.push_statement(mir::Statement::new(mir::StatementKind::Assign { 
                    place, 
                    value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
                }));
            }
            
            hir::Global::InternalAssign(assign,_) => {
                self.current.function = self.tree.start_function;
                self.current.block = Some(self.expect_start_block());

                let place = self.lower_place(&assign.place).pass(is_end);
                let value = self.lower_operand(assign.value).pass(is_end);
                self.push_statement(mir::Statement::new(mir::StatementKind::Assign { 
                    place, 
                    value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
                }));
            }
        }
    }

    fn lower_global_variable(&mut self, variable: &hir::Variable) -> mir::LocalId {
        let ty = self.types.locals[variable.local];
        let local = self.new_local(variable.local, ty);

        let value_id = match variable.value {
            Some(val) => val,
            None => {
                #[cfg(debug_assertions)]
                self.log_error(soul_error_internal!("global variables should have Some(_) value", None));
                return local
            }
        };

        let id = self.id_generators.alloc_global();
        let literal = if let hir::ExpressionKind::Literal(literal) = &self.hir.expressions[value_id].kind {
            Some(literal.clone())
        } else {
            None
        };

        let global = mir::Global {
            id,
            ty,
            local,
            literal,
        };
        self.tree.globals.insert(id, global);
        local
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
        self.push_statement_from(statement, block_id)
    }

    fn push_statement_from(&mut self, statement: mir::Statement, block_id: mir::BlockId) -> mir::StatementId {
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

    fn expect_start_block(&mut self) -> mir::BlockId {
        
        let start = self.tree.start_function;
        let block = self.tree.functions.get(start).map(|block| block.blocks.get(0));

        match block {
            Some(Some(val)) => *val,
            _ => {
                self.log_error(soul_error_internal!(
                    "expected _start function to have block",
                    None
                ));
                mir::BlockId::error()
            }
        }
    }

    pub(crate) fn expression_ty(&self, id: hir::ExpressionId) -> hir::TypeId {
        self.types.expressions.get(id).copied().unwrap_or(hir::TypeId::error())
    }

    fn to_mir_tree(self) -> MirTree {
        self.tree
    }
}
