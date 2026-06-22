use std::path::PathBuf;

use ast_model::{AstModuleStore, AstStore, AstTree, CustomType, EntryKind, NodeId, ScopeInfo, block::BlockId, expression::ExpressionId, scope::ScopeId, statements::StatementId};
use soul_utils::{CrateContext, FunctionId, collections::{module_store::ModuleStore, soul_import_path::SoulImportPath, vec_map::VecMap}, ids::IdGenerator, span::ModuleId};

mod resolve;
mod collect;
mod utils;

pub fn name_resolve<'a>(
    source_folder: PathBuf,
    module_store: &mut ModuleStore,
    ast: &mut AstTree,
) {
    let root = ast.root;
    let mut resolver = NameResolver::new(ast.root, source_folder, module_store, ast);
    resolver.collect_module(root);
    resolver.resolve_module(root);
}


struct NameResolver<'a> {
    current: Current,
    nodes: NodeStore,
    store: &'a AstStore,
    modules: &'a mut ModuleStore,
    context: &'a mut CrateContext,
    scope_info: &'a mut ScopeInfo,
    ast_modules: &'a mut AstModuleStore,
    node_generator: IdGenerator<NodeId>,
    scope_ids: VecMap<BlockId, ScopeId>,
    module_childern: Vec<SoulImportPath>,
}

struct Current {
    in_global: bool,
    module: ModuleId,
    in_if_condition: bool,
    source_folder: PathBuf,
    resolving_default: bool,
    path_stack: Vec<PathBuf>,
    function: Option<FunctionId>,
}

#[derive(Debug, Default)]
struct NodeStore {
    blocks: VecMap<BlockId, NodeId>,
    statements: VecMap<StatementId, NodeId>,
    expressions: VecMap<ExpressionId, NodeId>,
}

impl<'a> NameResolver<'a> {
    pub fn new(
        module: ModuleId, 
        source_folder: PathBuf, 
        modules: &'a mut ModuleStore,
        ast: &'a mut AstTree,
    ) -> Self {
        Self {
            modules,
            store: &ast.store,
            context: &mut ast.context,
            ast_modules: &mut ast.modules,
            scope_info: &mut ast.scope_info,
            nodes: NodeStore::default(),
            current: Current { 
                module, 
                source_folder, 
                in_global: true, 
                function: None,
                path_stack: vec![], 
                in_if_condition: false, 
                resolving_default: false, 
            },
            scope_ids: VecMap::new(),
            module_childern: Vec::new(),
            node_generator: IdGenerator::new(),
        }
    }

    fn header_insert_custom_type(&mut self, id: StatementId, custom: CustomType) -> Option<EntryKind<CustomType>> {
        let is_public = self.store.statements.get(id)
            .map(|s| s.is_public())
            .unwrap_or(false);
        
        let header = &mut self.ast_modules.get_mut(self.current.module)?.header;
        let entry = match header.get_mut(custom.name().as_str()) {
            Some(val) => val,
            None => header.entry(custom.name().to_string()).or_default(),
        };

        entry.struct_type.replace(EntryKind {
            value: custom,
            is_public,
        })
    }

    fn into_module_childern(self) -> Vec<SoulImportPath> {
        self.module_childern
    }
}

impl Current {
    fn current_path(&self) -> PathBuf {
        let mut result = self.source_folder.clone();
        for component in &self.path_stack {
            result.push(component);
        }
        result
    }

    fn push_current_path(&mut self, path: PathBuf) {
        self.path_stack.push(path);
    }

    fn pop_current_path(&mut self) {
        self.path_stack.pop();
    }
}
