use ast::{
    AstResponse, Block, DeclareStore, Function, SoulType, Statement, meta_data::AstMetadata, scope::{NodeId, ScopeModuleEntry, ScopeValue}
};
use soul_utils::{
    Ident,
    error::SoulError,
    ids::{FunctionId, IdGenerator},
    sementic_level::SementicFault,
    span::{Span, Spanned},
};

mod check_name;
mod collect;
mod resolve;

pub fn name_resolve(request: &mut AstResponse, faults: &mut Vec<SementicFault>) {
    let mut resolver = NameResolver::new(
        &mut request.meta_data,
        faults,
        &mut request.store,
        &mut request.function_generators,
        request.source_file.clone(),
    );

    let root = &mut request.tree.root;

    resolver.collect_declarations(root);
    resolver.resolve_import_functions(root);
    
    resolver.resolve_names(root);
}

struct NameResolver<'a> {
    node_generator: IdGenerator<NodeId>,
    function_generator: &'a mut IdGenerator<FunctionId>,
    info: &'a mut AstMetadata,
    store: &'a mut DeclareStore,
    faults: &'a mut Vec<SementicFault>,
    current_function: Option<FunctionId>,
    source_file: Option<std::path::PathBuf>,
    imported_functions: Vec<(Function, String)>,
}
impl<'a> NameResolver<'a> {
    fn new(
        info: &'a mut AstMetadata,
        faults: &'a mut Vec<SementicFault>,
        store: &'a mut DeclareStore,
        function_generator: &'a mut IdGenerator<FunctionId>,
        source_file: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            info,
            store,
            faults,
            function_generator,
            current_function: None,
            node_generator: IdGenerator::new(),
            source_file,
            imported_functions: Vec::new(),
        }
    }

    fn log_error(&mut self, error: SoulError) {
        self.faults.push(SementicFault::error(error));
    }

    fn check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValue::Variable)
    }

    fn lookup_module(&mut self, name: &str) -> Option<ast::scope::ScopeModuleEntry> {
        self.info.scopes.lookup_module(name)
    }

    fn lookup_module_function(&self, module_name: &str, function_name: &str) -> Option<FunctionId> {
        let entry_key = format!("{}::{}", module_name, function_name);
        self.info.scopes.lookup_function(&Ident::new_dummy(&entry_key))
    }

    fn flat_check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValue::Variable)
    }

    pub fn add_imported_function(&mut self, func: Function, import_name: String) {
        self.imported_functions.push((func, import_name));
    }

    pub fn take_imported_functions(&mut self) -> Vec<(Function, String)> {
        std::mem::take(&mut self.imported_functions)
    }

    fn resolve_import_functions(&mut self, root: &mut Block) {
        let mut imported_modules = Vec::new();

        let import_funcs = self.take_imported_functions();
        for (mut function, name) in import_funcs {
            let signature = &mut function.signature.node;
            let func_name = signature.name.as_str().to_string();
            let import_name = name.clone();
            
            if !imported_modules.contains(&import_name) {
                imported_modules.push(import_name.clone());
            }
            
            
            if signature.external.is_some() {
                signature.methode_type = SoulType::none(Span::default());
                signature.name = Ident::new_dummy(&func_name);
            } else {
                let qualified_name = format!("{}::{}", import_name, func_name);
                signature.name = Ident::new_dummy(&qualified_name);
            }
            
            self.collect_function(&mut function);
            
            let stmt = Statement::from_function(Spanned::new(function, Span::default()));
            root.statements.push(stmt);
        }

        if let Some(scope) = self.info.scopes.current_scope_mut() {
            for module_name in imported_modules {
                scope.insert_module(&module_name, ScopeModuleEntry {
                    module_name: module_name.clone(),
                    import_kind: ast::ImportKind::All,
                });
            }
        }
    }
}