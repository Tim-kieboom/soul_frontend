mod id_generator;
use ast::{AstResponse, DeclareStore};
use hir::{BlockId, GenericId, HirTree, LocalId, TypeId};
use id_generator::IdAllocalors;
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdAlloc, IdGenerator},
    sementic_level::SementicFault,
    soul_error_internal,
    span::Span,
    vec_map::VecMapIndex,
};
use std::collections::HashMap;

mod expression;
mod insert;
mod place;
mod resolve_import;
mod statement;
mod r#type;

pub fn hir_lower(response: &AstResponse, faults: &mut Vec<SementicFault>) -> HirTree {
    let mut context = HirContext::new(
        response.function_generators.clone(),
        &response.store,
        faults,
    );

    for global in &response.tree.root.statements {
        context.lower_global(global);
    }

    context.hir
}

#[derive(Debug, Default)]
struct Scope {
    locals: HashMap<String, LocalId>,
    generics: HashMap<String, GenericId>,
    functions: HashMap<String, FunctionId>,
}

#[derive(Debug)]
struct HirContext<'a> {
    pub ast_store: &'a DeclareStore,

    pub hir: HirTree,

    pub scopes: Vec<Scope>,
    pub current_body: CurrentBody,

    pub id_generator: IdAllocalors,
    pub faults: &'a mut Vec<SementicFault>,
}

#[derive(Debug, Clone, Copy, Default)]
enum CurrentBody {
    #[default]
    Global,
    Block(BlockId),
}

impl<'a> HirContext<'a> {
    fn new(
        function_generator: IdGenerator<FunctionId>,
        ast_store: &'a DeclareStore,
        faults: &'a mut Vec<SementicFault>,
    ) -> Self {
        let mut id_generator = IdAllocalors::new(function_generator);
        let init_global_function = id_generator.alloc_function();
        let root_id = id_generator.alloc_module();

        let main = match ast_store.main_function {
            Some(val) => val,
            None => {
                faults.push(SementicFault::error(SoulError::new(
                    "main function not found",
                    SoulErrorKind::InvalidContext,
                    None,
                )));
                FunctionId::error()
            }
        };

        Self {
            faults,
            ast_store,
            id_generator,
            scopes: vec![Scope::default()],
            current_body: CurrentBody::Global,
            hir: HirTree::new(root_id, main, init_global_function),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn find_local(&mut self, name: &Ident) -> Option<LocalId> {
        for store in self.scopes.iter().rev() {
            if let Some(id) = store.locals.get(name.as_str()).copied() {
                return Some(id);
            }
        }

        None
    }

    fn insert_generic(&mut self, name: &Ident, id: GenericId) {
        let scope = match self.scopes.last_mut() {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "tryed to insert_generic in global scope",
                    Some(name.span)
                ));
                return;
            }
        };

        scope.generics.insert(name.node.clone(), id);
        self.hir.types.insert_generic(name.node.clone(), id);
    }

    fn insert_parameter(&mut self, name: &Ident, local: LocalId, ty: TypeId) {
        self.inner_insert_local(
            name,
            local,
            hir::LocalInfo {
                ty,
                kind: hir::LocalKind::Parameter,
            },
        );
    }

    fn insert_variable(&mut self, name: &Ident, local: LocalId, ty: TypeId) {
        self.inner_insert_local(
            name,
            local,
            hir::LocalInfo {
                ty,
                kind: hir::LocalKind::Variable,
            },
        );
    }

    fn insert_temp(&mut self, name: &Ident, local: LocalId, ty: TypeId) {
        self.inner_insert_local(
            name,
            local,
            hir::LocalInfo {
                ty,
                kind: hir::LocalKind::Temp,
            },
        );
    }

    fn inner_insert_local(&mut self, name: &Ident, local: LocalId, info: hir::LocalInfo) {
        let scope = match self.scopes.last_mut() {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "tryed to insert_local in global scope",
                    Some(name.span)
                ));
                return;
            }
        };

        self.hir.spans.locals.insert(local, name.span);
        scope.locals.insert(name.to_string(), local);
        self.hir.locals.insert(local, info);
    }

    fn insert_block(&mut self, id: BlockId, block: hir::Block, span: Span) {
        self.hir.blocks.insert(id, block);
        self.hir.spans.blocks.insert(id, span);
    }

    fn insert_in_block(&mut self, id: BlockId, statement: hir::Statement) {
        self.hir.blocks[id].statements.push(statement);
    }

    fn insert_block_terminator(&mut self, id: BlockId, terminator: hir::ExpressionId) {
        self.hir.blocks[id].terminator = Some(terminator)
    }

    fn insert_function(&mut self, name: &Ident, function: FunctionId) {
        let scope = match self.scopes.last_mut() {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "tryed to insert_local in global scope",
                    Some(name.span)
                ));
                return;
            }
        };

        self.hir.spans.functions.insert(function, name.span);
        scope.functions.insert(name.to_string(), function);
    }

    fn pop_scope(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }
}

fn create_local_name(id: LocalId) -> String {
    format!("__local_{}", id.index())
}
