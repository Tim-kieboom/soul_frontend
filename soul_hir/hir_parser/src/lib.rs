use std::collections::HashMap;
use std::path::PathBuf;

use ast::{AbtractSyntaxTree, Visibility, scope::NodeId};
use hir::{
    BlockId, CustomTypeId, ExpressionId, GenericId, HirTree, LazyTypeId, LocalId, StatementId,
};
use soul_utils::{
    Ident,
    crate_store::{CrateContext, CrateExports},
    error::{SoulError, SoulErrorKind},
    ids::FunctionId,
    sementic_level::SementicFault,
    soul_error_internal,
    span::{ItemMetaData, ModuleId, Span},
    vec_map::{VecMap, VecMapIndex},
};

use crate::id_allocator::IdAllocalor;

mod expression;
mod id_allocator;
mod place;
mod statement;
mod r#type;

pub fn lower_hir(
    faults: &mut CrateContext,
    ast_context: &AbtractSyntaxTree,
    crate_exports: &CrateExports,
    root: ModuleId,
    source_folder: PathBuf,
) -> HirTree {
    let mut context = HirContext::new(faults, ast_context, crate_exports, root, source_folder);

    context.lower_internal_structs();
    context.lower_module(root);
    context.consume_to_hir()
}

#[derive(Debug)]
struct Current {
    pub module: ModuleId,
    pub body: CurrentBody,
}

#[derive(Debug)]
struct HirContext<'a> {
    pub tree: HirTree,

    pub current: Current,
    pub scopes: Vec<Scope>,
    pub id_generator: IdAllocalor,
    pub ast_context: &'a AbtractSyntaxTree,
    pub _crate_exports: &'a CrateExports,

    pub context: &'a mut CrateContext,
    pub node_id_to_local: VecMap<NodeId, LocalId>,
    pub root_id: ModuleId,
    pub source_folder: PathBuf,
}
impl<'a> HirContext<'a> {
    fn new(
        context: &'a mut CrateContext,
        ast_context: &'a AbtractSyntaxTree,
        crate_exports: &'a CrateExports,
        root_id: ModuleId,
        source_folder: PathBuf,
    ) -> Self {
        let mut id_generator = IdAllocalor::new(ast_context.function_generators.clone());
        let init_global_function = id_generator.alloc_function();

        let main = ast_context.store.main_function;
        if !context.is_lib && main.is_none() {
            context.faults.push(SementicFault::error(SoulError::new(
                "main function not found",
                SoulErrorKind::InvalidContext,
                None,
            )));
        }

        let root = &ast_context.modules[root_id];
        let mut tree = HirTree::new(root, main, init_global_function);
        Self::init_submodules(&mut tree, ast_context, root_id);

        Self {
            context,
            ast_context,
            _crate_exports: crate_exports,
            id_generator,
            scopes: vec![Scope::default()],
            node_id_to_local: VecMap::new(),
            current: Current {
                module: root_id,
                body: CurrentBody::Global,
            },
            tree,
            root_id,
            source_folder,
        }
    }

    fn init_submodules(tree: &mut HirTree, ast_context: &AbtractSyntaxTree, module_id: ModuleId) {
        let ast_module = &ast_context.modules[module_id];
        let sub_modules: Vec<ModuleId> = ast_module.modules.entries().collect();
        for sub_module_id in sub_modules.iter().cloned() {
            let sub_ast_module = &ast_context.modules[sub_module_id];
            let sub_sub_modules: Vec<ModuleId> = sub_ast_module.modules.entries().collect();
            let is_public = matches!(ast_module.visibility, Visibility::Public);
            tree.insert_module(sub_module_id, is_public, sub_sub_modules);
            Self::init_submodules(tree, ast_context, sub_module_id);
        }
    }

    fn lower_module(&mut self, module_id: ModuleId) {
        let ast_module = &self.ast_context.modules[module_id];

        let prev = self.current.module;
        self.current.module = module_id;

        for statement in &ast_module.global.statements {
            match &statement.node {
                ast::StatementKind::Struct(object) => self.add_struct(object),
                ast::StatementKind::Enum(object) => self.add_enum(object),
                _ => (),
            }
        }

        for global in &ast_module.global.statements {
            if matches!(global.node, ast::StatementKind::Variable(_)) {
                self.lower_global(module_id, global);
            }
        }

        for sub_module_id in ast_module.modules.entries() {
            self.lower_module(sub_module_id);
        }

        for global in &ast_module.global.statements {
            if !matches!(global.node, ast::StatementKind::Variable(_)) {
                self.lower_global(module_id, global);
            }
        }
        self.current.module = prev;
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
        self.context.faults.push(SementicFault::error(err));
    }

    fn consume_to_hir(self) -> HirTree {
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
    custom_types: HashMap<String, CustomTypeId>,
}

#[derive(Debug, Clone, Copy, Default)]
enum CurrentBody {
    #[default]
    Global,
    Block(BlockId),
}
