use ast::AbtractSyntaxTree;
use mir_parser::{mir::MirTree, mir_lower};
use run_hir::HirResponse;
use soul_utils::{compile_options::CompilerOptions, crate_store::{CrateContext, CrateExports}, span::ModuleId, vec_map::VecMap};

pub struct MirResponse {
    pub tree: MirTree,
    pub root: ModuleId,
}

pub fn to_mir(
    hir_response: &HirResponse,
    ast: &AbtractSyntaxTree,
    _options: &CompilerOptions,
    context: &mut CrateContext,
    root: ModuleId,
) -> MirResponse {
    MirResponse {
        tree: mir_lower(&hir_response, &ast.modules, context, root),
        root,
    }
}

pub fn extract_exports(mir: &MirResponse) -> CrateExports {
    let mut exports = CrateExports::default();
    
    let root = mir.root;
    
    for func_id in &mir.tree.public_functions {
        if let Some(func) = mir.tree.functions.get(*func_id) {
            let func_name = func.name.as_str().to_string();
            let module_id = func.from_module;
            
            let module_path = get_module_path(module_id, &mir.tree.modules, root);
            let full_name = if module_path.is_empty() {
                func_name
            } else {
                format!("{}.{}", module_path, func_name)
            };
            
            exports.functions.insert(full_name, *func_id);
        }
    }
    
    exports
}

fn get_module_path(
    module_id: ModuleId, 
    mir_modules: &VecMap<ModuleId, mir_parser::mir::Module>, 
    root: ModuleId
) -> String {
    if module_id == root {
        return String::new();
    }
    
    let mut path = String::new();
    let mut current = module_id;
    
    while let Some(module) = mir_modules.get(current) {
        if current != root {
            if !path.is_empty() {
                path.insert(0, '.');
            }
            path.insert_str(0, &module.name);
        }
        
        if current == root {
            break;
        }
        current = module.parent.unwrap_or(root);
    }
    
    path
}
