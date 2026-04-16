use std::collections::HashMap;

use ast::{AstContext, scope::NodeId};
use hir::{
    BlockId, CreatedTypes, ExpressionId, Field, GenericId, HirTree, HirType, LazyTypeId, LocalId,
    StatementId, Struct,
};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdAlloc},
    sementic_level::{CompilerContext, SementicFault},
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

pub fn lower_hir(compiler_context: &mut CompilerContext, ast_context: &AstContext) -> HirTree {
    let root = compiler_context.module_store.get_root_id();
    let mut context = HirContext::new(compiler_context, ast_context);

    context.lower_internal_structs();
    context.lower_module(root);
    context.consume_to_hir()
}

#[derive(Debug)]
struct HirContext<'a> {
    pub tree: HirTree,

    pub scopes: Vec<Scope>,
    pub id_generator: IdAllocalor,
    pub current_body: CurrentBody,
    pub ast_context: &'a AstContext,

    pub context: &'a mut CompilerContext,
    pub node_id_to_local: VecMap<NodeId, LocalId>,
}
impl<'a> HirContext<'a> {
    fn new(context: &'a mut CompilerContext, ast_context: &'a AstContext) -> Self {
        let mut id_generator = IdAllocalor::new(ast_context.function_generators.clone());
        let init_global_function = id_generator.alloc_function();
        let root_id = context.module_store.get_root_id();

        let main = match ast_context.store.main_function {
            Some(val) => val,
            None => {
                context.faults.push(SementicFault::error(SoulError::new(
                    "main function not found",
                    SoulErrorKind::InvalidContext,
                    None,
                )));
                FunctionId::error()
            }
        };

        Self {
            context,
            ast_context,
            id_generator,
            scopes: vec![Scope::default()],
            node_id_to_local: VecMap::new(),
            current_body: CurrentBody::Global,
            tree: HirTree::new(root_id, main, init_global_function),
        }
    }

    fn lower_module(&mut self, module_id: ModuleId) {
        let root = &self.ast_context.modules[module_id];
        
        for global in &root.global.statements {
            if matches!(global.node, ast::StatementKind::Variable(_)) {
                self.lower_global(global);
            }
        }

        for module_id in root.modules.iter().copied() {
            let module = &self.ast_context.modules[module_id];
            for global in &module.global.statements {
                if matches!(global.node, ast::StatementKind::Variable(_)) {
                    self.lower_global(global);
                }
            }
        }

        for global in &root.global.statements {
            if !matches!(global.node, ast::StatementKind::Variable(_)) {
                self.lower_global(global);
            }
        }

        for module_id in root.modules.iter().copied() {
            let module = &self.ast_context.modules[module_id];
            for global in &module.global.statements {
                if !matches!(global.node, ast::StatementKind::Variable(_)) {
                    self.lower_global(global);
                }
            }
        }
    }

    fn lower_internal_structs(&mut self) {
        let struct_id = self.tree.info.types.alloc_struct();
        let generic_id = self.tree.info.types.insert_generic("T".to_string());
        let name = Ident::new(
            "___Array".to_string(),
            Span::default(self.context.module_store.get_root_id()),
        );

        let generic_type = self.add_type(HirType::generic_type(generic_id)).to_lazy();
        let ptr_type = self.add_type(HirType::pointer_type(generic_type)).to_lazy();
        let len_type = self.add_type(HirType::index_type());
        let fields = vec![
            Field {
                struct_id,
                id: self.id_generator.alloc_field(),
                name: "ptr".to_string(),
                ty: ptr_type,
            },
            Field {
                struct_id,
                id: self.id_generator.alloc_field(),
                name: "len".to_string(),
                ty: len_type.to_lazy(),
            },
        ];

        self.tree.info.types.array_struct = struct_id;
        // to insure struct is in compiler
        self.add_type(
            HirType::new(hir::HirTypeKind::Struct(struct_id)).apply_generics(vec![len_type]),
        );
        self.insert_struct(struct_id, Struct { name, fields });
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
    created_type: HashMap<String, CreatedTypes>,
}

#[derive(Debug, Clone, Copy, Default)]
enum CurrentBody {
    #[default]
    Global,
    Block(BlockId),
}
