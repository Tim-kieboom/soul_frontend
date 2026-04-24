use ast::AbtractSyntaxTree;
use mir_parser::{mir::MirTree, mir_lower};
use run_hir::HirResponse;
use soul_utils::{compile_options::CompilerOptions, crate_store::CrateContext, span::ModuleId};

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
