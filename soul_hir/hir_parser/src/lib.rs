use std::collections::HashMap;

use ast::{AstResponse, DeclareStore};
use hir::{
    BlockId, CreatedTypes, ExpressionId, GenericId, HirTree, LazyTypeId, LocalId, StatementId,
};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdAlloc},
    sementic_level::SementicFault,
    soul_error_internal,
    span::{ItemMetaData, Span},
    vec_map::VecMapIndex,
};

use crate::id_allocator::IdAllocalor;

mod expression;
mod id_allocator;
mod place;
mod statement;
mod r#type;

pub fn lower_hir(response: &AstResponse, faults: &mut Vec<SementicFault>) -> HirTree {
    let mut context = HirContext::new(response, faults);

    for global in &response.tree.root.statements {
        context.lower_global(global);
    }

    context.to_hir()
}

#[derive(Debug)]
struct HirContext<'a> {
    pub tree: HirTree,

    pub scopes: Vec<Scope>,
    pub id_generator: IdAllocalor,
    pub current_body: CurrentBody,
    pub ast_store: &'a DeclareStore,

    pub faults: &'a mut Vec<SementicFault>,
}
impl<'a> HirContext<'a> {
    fn new(response: &'a AstResponse, faults: &'a mut Vec<SementicFault>) -> Self {
        let mut id_generator = IdAllocalor::new(response.function_generators.clone());
        let init_global_function = id_generator.alloc_function();
        let root_id = id_generator.alloc_module();

        let main = match response.store.main_function {
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
            id_generator,
            ast_store: &response.store,
            scopes: vec![Scope::default()],
            current_body: CurrentBody::Global,
            tree: HirTree::new(root_id, main, init_global_function),
        }
    }

    fn alloc_statement(&mut self, meta_data: &ItemMetaData, span: Span) -> StatementId {
        let id = self.id_generator.alloc_statement();
        self.tree.info.spans.statements.insert(id, span);
        self.tree
            .info
            .meta_data
            .statements
            .insert(id, meta_data.clone());
        id
    }

    pub(crate) fn alloc_expression(&mut self, span: Span) -> ExpressionId {
        let id = self.id_generator.alloc_expression();
        self.tree.info.spans.expressions.insert(id, span);
        id
    }

    fn insert_parameter(&mut self, name: &Ident, local: LocalId, ty: LazyTypeId) {
        self.inner_insert_local(
            name,
            local,
            hir::LocalInfo {
                ty,
                kind: hir::LocalKind::Parameter,
                span: self.tree.info.spans.locals.get(local).copied(),
            },
        );
    }

    fn insert_variable(
        &mut self,
        name: &Ident,
        local: LocalId,
        ty: LazyTypeId,
        value: Option<ExpressionId>,
    ) {
        self.inner_insert_local(
            name,
            local,
            hir::LocalInfo {
                ty,
                kind: hir::LocalKind::Variable(value),
                span: self.tree.info.spans.locals.get(local).copied(),
            },
        );
    }

    fn insert_temp(&mut self, name: &Ident, local: LocalId, ty: LazyTypeId, value: ExpressionId) {
        self.inner_insert_local(
            name,
            local,
            hir::LocalInfo {
                ty,
                kind: hir::LocalKind::Temp(value),
                span: self.tree.info.spans.locals.get(local).copied(),
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

        self.tree.info.spans.locals.insert(local, name.span);
        scope.locals.insert(name.to_string(), local);
        self.tree.nodes.locals.insert(local, info);
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }

    fn to_hir(self) -> HirTree {
        self.tree
    }
}

fn create_local_name(id: LocalId) -> String {
    format!("___{}", id.index())
}

#[derive(Debug, Default)]
struct Scope {
    locals: HashMap<String, LocalId>,
    generics: HashMap<String, GenericId>,
    functions: HashMap<String, FunctionId>,
    created_type: HashMap<String, CreatedTypes>,
}

#[derive(Debug, Clone, Copy, Default)]
enum CurrentBody {
    #[default]
    Global,
    Block(BlockId),
}
