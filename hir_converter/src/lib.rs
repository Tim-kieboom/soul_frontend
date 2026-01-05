use std::collections::HashMap;

use soul_ast::{
    ParseResonse,
    abstract_syntax_tree::{AbstractSyntaxTree, Ident, Visibility},
    sementic_models::AstMetadata,
};
use soul_hir::{self as hir, HirBodyId, HirId, HirResponse, HirTree};
use soul_utils::{AsIndex, SementicFault, SoulError, SoulErrorKind, SoulResult, Span, VecMap};

mod lower_for_pattern;
mod lower_expression;
mod lower_item;
mod lower_type;

pub fn lower_abstract_syntax_tree(request: &ParseResonse) -> HirResponse {
    let mut faults = vec![];
    let hir = HirLowerer::new(request, &mut faults).lower();
    HirResponse { hir, faults }
}

pub(crate) struct HirLowerer<'a> {
    ast: &'a AbstractSyntaxTree,
    semantic: &'a AstMetadata,

    module: hir::Module,

    scope_depth: u32,
    current_scope_id: hir::ScopeId,
    current_body_id: Option<HirBodyId>,
    faults: &'a mut Vec<SementicFault>,
}
impl<'hir> HirLowerer<'hir> {

    pub fn new(value: &'hir ParseResonse, faults: &'hir mut Vec<SementicFault>) -> Self {
        let root_id = hir::ScopeId::new(0);
        let root = hir::Scope {
            parent: None,
            locals: HashMap::new(),
        };

        let module = hir::Module {
            next_id: HirId::new(0),
            next_scope_id: root_id,
            items: VecMap::new(),
            bodies: VecMap::new(),
            scopes: VecMap::from_vec(vec![(root_id, root)]),
            expressions: VecMap::new(),
        };

        Self {
            current_scope_id: root_id,
            semantic: &value.sementic_info,
            ast: &value.syntax_tree,
            current_body_id: None,
            scope_depth: 0,
            module,
            faults,
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

    pub(crate) fn add_statement(&mut self, statement: hir::Statement) -> SoulResult<HirId> {
        
        let id = self.alloc_id();

        match self.current_body()? {
            soul_hir::Body::Block(block) => {
                block.statements.insert(id, statement);
            }
            soul_hir::Body::Expression(_) => {
                return Err(SoulError::new(
                    format!(
                        "can not add {} to expression",
                        statement.node.get_variant_name()
                    ),
                    SoulErrorKind::InvalidContext,
                    Some(statement.span),
                ));
            }
        }

        Ok(id)
    }

    pub(crate) fn log_error(&mut self, error: SoulError) {
        self.faults.push(SementicFault::error(error));
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

    pub(crate) fn try_get_locale(&self, ident: &Ident) -> SoulResult<(hir::LocalDefId, hir::ScopeId)> {
        
        let name = ident.as_str();
        let scopes = &self.module.scopes;
        let mut current_id = self.current_scope_id;
        let mut current = Some(self.current_scope(ident.span)?);
        while let Some(scope) = current {
            
            if let Some(local) = scope.locals.get(name) {
                return Ok((*local, current_id))
            }
            
            current_id = match scope.parent {
                Some(val) => val,
                None => break,
            };

            current = scopes.get(current_id);
        }
            

        Err(SoulError::new(
            format!("could not find locale of name: \"{}\"", ident.as_str()),
            SoulErrorKind::InvalidIdent,
            Some(ident.span)
        ))
    }

    pub(crate) fn current_scope(&self, span: Span) -> SoulResult<&hir::Scope> {
        self.module
            .scopes
            .get(self.current_scope_id)
            .ok_or(SoulError::new(
                "current scope not found",
                SoulErrorKind::InternalError,
                Some(span),
            ))
    }

    pub(crate) fn current_body(&mut self) -> SoulResult<&mut hir::Body> {
        let body_id = match self.current_body_id {
            Some(val) => val,
            None => {
                return Err(SoulError::new(
                    "trying to get body while in global scope",
                    SoulErrorKind::InternalError,
                    None,
                ));
            }
        };

        self
            .module
            .bodies
            .get_mut(body_id)
            .ok_or(SoulError::new(
                "should have body_id",
                SoulErrorKind::InternalError,
                None,
            ))
    }

    pub(crate) fn push_scope(&mut self) -> hir::ScopeId {
        let parent = self.current_scope_id;
        let scope = self.alloc_scope_id();

        self.module.scopes.insert(
            scope,
            hir::Scope {
                parent: Some(parent),
                locals: HashMap::new(),
            },
        );

        self.scope_depth += 1;
        self.current_scope_id = scope;
        scope
    }

    pub(crate) fn pop_scope(&mut self) {
        let parent = self
            .module
            .scopes
            .get(self.current_scope_id)
            .expect("should have current scope")
            .parent
            .expect("should have parent");

        self.current_scope_id = parent;
        self.scope_depth -= 1;
    }

    pub(crate) fn vis_from_name(&self, ident: &Ident) -> Visibility {
        let first = ident.as_str().chars().next();

        if first.is_some_and(|el| el.is_uppercase()) {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }
}
