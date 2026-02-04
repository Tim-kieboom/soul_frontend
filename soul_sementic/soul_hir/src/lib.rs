use std::collections::HashMap;

use hir_model::{self as hir, ExpressionId, HirResponse};
use parser_models::{
    ParseResponse,
    scope::NodeId,
};
#[cfg(debug_assertions)]
use soul_utils::soul_error_internal;
use soul_utils::{
    error::SoulError,
    sementic_level::SementicFault,
    span::Span,
    vec_map::{VecMapIndex, VecMap},
};

mod lower_condition;
mod lower_expression;
mod lower_statement;
mod lower_type;


pub fn lower_to_hir(request: &ParseResponse) -> HirResponse {
    HirLowerer::lower_ast(request)
}

struct HirLowerer {
    module: hir::Module,

    current_scope: hir::ScopeId,
    current_item: Option<NodeId>,

    faults: Vec<SementicFault>,
}

impl HirLowerer {
    pub(crate) fn lower_ast(request: &ParseResponse) -> HirResponse {
        let ParseResponse { tree, .. } = request;
        let mut this = Self::new();

        for statement in &tree.root.statements {
            this.lower_globals_statements(statement);
        }

        HirResponse {
            hir: hir::HirTree { root: this.module },
            faults: this.faults,
        }
    }

    fn new() -> Self {
        let mut scopes = VecMap::new();
        scopes.insert(hir::ScopeId::new_index(0), hir::Scope::new_global());

        Self {
            module: hir::Module {
                next_scope_id: hir::ScopeId::new_index(1),
                expressions: VecMap::new(),
                functions: VecMap::new(),
                bodies: VecMap::new(),
                items: VecMap::new(),
                scopes,
            },
            faults: vec![],
            current_item: None,
            current_scope: hir::ScopeId::new_index(0),
        }
    }

    fn push_block(&mut self, body: hir::Block, span: Span) {
        self.module
            .bodies
            .insert(body.id, hir::Body::Block(body, span));
    }

    fn push_item(&mut self, id: NodeId, entry: hir::Item)  {
        self.module.items.insert(id, entry);
    }

    fn push_expression(&mut self, id: ExpressionId, entry: hir::Expression)  {
        self.module.expressions.insert(id, entry);
    }

    fn push_scope(&mut self) -> hir::ScopeId {
        let scope_id = self.module.next_scope_id;
        self.module.next_scope_id.increment();

        self.module.scopes.insert(
            scope_id,
            hir::Scope {
                parent: Some(self.current_scope),
                locals: HashMap::new(),
            },
        );

        let prev = self.current_scope;
        self.current_scope = scope_id;
        prev
    }

    fn pop_scope(&mut self, prev: hir::ScopeId) {
        self.current_scope = prev;
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }

    /// logs error if Option<NodeId> == None, just return same value
    fn expect_node_id(&mut self, id: Option<NodeId>, span: Span) -> Option<NodeId> {

        if id.is_none() {

            #[cfg(debug_assertions)]
            self.log_error(
                soul_error_internal!("AbstractSyntaxTree Node does not have NodeId", Some(span))
            );
        }

        id
    }
}
