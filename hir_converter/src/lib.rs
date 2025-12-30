use std::collections::HashMap;

use soul_ast::{ParseResonse, abstract_syntax_tree::AbstractSyntaxTree, sementic_models::ASTSemanticInfo};
use soul_hir::{self as hir, HirId, HirResponse, HirTree};
use soul_ast::abstract_syntax_tree as ast;

pub mod lower_item;

pub fn lower_abstract_syntax_tree(request: &ParseResonse) -> HirResponse {
    let hir = HirLowerer::new(request).lower();
    HirResponse { hir, faults: vec![] }
}

pub(crate) struct HirLowerer<'hir> {
    scope_depth: u32,
    module: hir::Module,
    ast: &'hir AbstractSyntaxTree,
    current_scope: Option<hir::ScopeId>,
    sementic_info: &'hir ASTSemanticInfo,
}
impl<'hir> HirLowerer<'hir> {

    pub fn new(value: &'hir ParseResonse) -> Self {
        let root_id = hir::ScopeId::new(0);
        let module = hir::Module {
            next_id: HirId::new(0),
            next_scope_id: root_id,
            items: HashMap::new(),
            bodies: HashMap::new(),
            scopes: HashMap::new(),
            expressions: HashMap::new(),
        };

        Self {
            current_scope: Some(root_id),
            sementic_info: &value.sementic_info,
            ast: &value.syntax_tree,
            scope_depth: 0,
            module,
        }
    }

    pub fn lower(mut self) -> HirTree {
        for statement in &self.ast.root.statements {
            self.lower_global_statment(statement);
        }

        HirTree { root: self.module }
    }

    pub(crate) fn add_item(&mut self, item: hir::Item) -> HirId {
        let id = item.get_id();
        self.module.items.insert(id, item);
        id
    }

    pub(crate) fn add_expression(&mut self, expression: hir::Expression) -> HirId {
        let id = self.alloc_id();
        self.module.expressions.insert(id, expression);
        id
    }

    pub(crate) fn alloc_id(&mut self) -> HirId {
        let id = self.module.next_id.clone();
        self.module.next_id.increment();
        id
    }

    pub(crate) fn alloc_scope_id(&mut self) -> hir::ScopeId {
        let id = self.module.next_scope_id.clone();
        self.module.next_scope_id.increment();
        id
    }
    
    pub(crate) fn create_root_scope() -> hir::Scope {
        todo!("impl create_root_scope")
    }
}