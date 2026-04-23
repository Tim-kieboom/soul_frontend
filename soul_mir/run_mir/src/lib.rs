use ast::{AbtractSyntaxTree};
use mir_parser::{mir::MirTree, mir_lower};
use run_hir::HirResponse;
use soul_utils::{compile_options::CompilerOptions, sementic_level::CompilerContext};

pub struct MirResponse {
    pub tree: MirTree,
}

pub fn to_mir(
    hir_response: &HirResponse,
    ast: &AbtractSyntaxTree,
    _options: &CompilerOptions,
    faults: &mut CompilerContext,
) -> MirResponse {
    MirResponse {
        tree: mir_lower(&hir_response, &ast.modules, faults),
    }
}
