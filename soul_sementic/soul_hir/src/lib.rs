use std::collections::HashMap;

use hir_model::{self as hir, ExpressionId, HirResponse};
use parser_models::{
    ParseResponse,
    scope::{NodeId, NodeIdGenerator},
};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    sementic_level::SementicFault,
    span::Span,
    vec_map::{AsIndex, VecMap},
};

mod lower_condition;
mod lower_expression;
mod lower_statement;
mod lower_type;


pub fn lower_to_hir(request: &ParseResponse) -> HirResponse {
    HirLowerer::lower_ast(request)
}

struct HirLowerer {
    id_generator: NodeIdGenerator,
    module: hir::Module,

    current_scope: hir::ScopeId,
    current_item: Option<NodeId>,

    faults: Vec<SementicFault>,
}

impl HirLowerer {
    pub(crate) fn lower_ast(request: &ParseResponse) -> HirResponse {
        let ParseResponse { tree, meta_data } = request;
        let mut this = Self::new(meta_data.last_node_id);

        for statement in &tree.root.statements {
            this.lower_globals_statements(statement);
        }

        HirResponse {
            hir: hir::HirTree { root: this.module },
            faults: this.faults,
        }
    }

    fn new(last_node_id: NodeId) -> Self {
        let mut scopes = VecMap::new();
        scopes.insert(hir::ScopeId::new(0), hir::Scope::new_global());

        Self {
            module: hir::Module {
                next_scope_id: hir::ScopeId::new(1),
                expressions: VecMap::new(),
                bodies: VecMap::new(),
                items: VecMap::new(),
                scopes,
            },
            faults: vec![],
            current_item: None,
            current_scope: hir::ScopeId::new(0),
            id_generator: NodeIdGenerator::from_last(last_node_id),
        }
    }

    fn push_block(&mut self, body: hir::Block) {
        self.module
            .bodies
            .insert(body.id, hir::Body::Block(body))
    }

    fn push_item(&mut self, id: NodeId, entry: hir::Item)  {
        self.module.items.insert(id, entry)
    }

    fn push_expression(&mut self, id: ExpressionId, entry: hir::Expression)  {
        self.module.expressions.insert(id, entry)
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
            self.log_error(SoulError::new(
                "AbstractSyntaxTree Node does not have NodeId",
                SoulErrorKind::InternalError,
                Some(span),
            ));
        }

        id
    }
}
