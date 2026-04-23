use std::collections::HashMap;

use ast::{AbtractSyntaxTree, scope::{NodeId}};
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

pub fn lower_hir(compiler_context: &mut CompilerContext, ast_context: &AbtractSyntaxTree, root: ModuleId) -> HirTree {
    let mut context = HirContext::new(compiler_context, ast_context, root);

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

    pub context: &'a mut CompilerContext,
    pub node_id_to_local: VecMap<NodeId, LocalId>,
    pub root_id: ModuleId,
}
impl<'a> HirContext<'a> {
    fn new(context: &'a mut CompilerContext, ast_context: &'a AbtractSyntaxTree, root_id: ModuleId) -> Self {
        let mut id_generator = IdAllocalor::new(ast_context.function_generators.clone());
        let init_global_function = id_generator.alloc_function();

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

        let root = &ast_context.modules[root_id];
        let mut tree = HirTree::new(root, main, init_global_function);
        Self::init_submodules(&mut tree, ast_context, root_id);

        Self {
            context,
            ast_context,
            id_generator,
            scopes: vec![Scope::default()],
            node_id_to_local: VecMap::new(),
            current: Current {
                module: root_id,
                body: CurrentBody::Global,
            },
            tree,
            root_id,
        }
    }

    fn init_submodules(tree: &mut HirTree, ast_context: &AbtractSyntaxTree, module_id: ModuleId) {
        let ast_module = &ast_context.modules[module_id];
        let sub_modules: Vec<ModuleId> = ast_module.modules.entries().collect();
        for sub_module_id in sub_modules.iter().cloned() {
            let sub_ast_module = &ast_context.modules[sub_module_id];
            let sub_sub_modules: Vec<ModuleId> = sub_ast_module.modules.entries().collect();
            tree.insert_module(sub_module_id, sub_sub_modules);
            Self::init_submodules(tree, ast_context, sub_module_id);
        }
    }

    fn lower_module(&mut self, module_id: ModuleId) {
        let ast_module = &self.ast_context.modules[module_id];

        for statement in &ast_module.global.statements {

            match &statement.node {
                ast::StatementKind::Struct(object) => self.add_struct(object),
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
    }

    fn lower_struct(&mut self, object: &ast::Struct) {
        
        let Some(scope) = self.scopes.last() else {
            self.log_error(soul_error_internal!(format!("self.scopes.last() not found"), Some(object.name.span)));
            return
        };

        let Some(CreatedTypes::Struct(struct_id)) = scope.created_type.get(object.name.as_str()).copied() else {
            self.log_error(soul_error_internal!(format!("{:?} not found", object.name.as_str()), Some(object.name.span)));
            return
        };


        let mut fields = vec![];
        for field in &object.fields {
            
            let ty = self.lower_type(&field.ty, field.name.span);
            let id = self.id_generator.alloc_field();
            
            let hir_field = hir::Field {
                id,
                ty,
                struct_id,
                name: field.name.clone(),
            };
            
            fields.push(hir_field.clone());
            self.tree.nodes.fields.insert(id, hir_field);
        }
        
        match self.tree.info.types.id_to_struct_mut(struct_id) {
            Some(obj) => {
                obj.fields = fields
            }
            None => (),
        }
    }

    fn lower_internal_structs(&mut self) {
        let struct_id = self.tree.info.types.alloc_struct();
        let name = Ident::new(
            "___Array".to_string(),
            Span::default(self.root_id),
        );

        let none_type = self.add_type(HirType::none_type()).to_lazy();
        let ptr_type = self.add_type(HirType::pointer_type(none_type)).to_lazy();
        let len_type = self.add_type(HirType::index_type());
        let fields = vec![
            Field {
                struct_id,
                id: self.id_generator.alloc_field(),
                name: Ident::new("ptr".to_string(), Span::error()),
                ty: ptr_type,
            },
            Field {
                struct_id,
                id: self.id_generator.alloc_field(),
                name: Ident::new("len".to_string(), Span::error()),
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
