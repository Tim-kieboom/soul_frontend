use mir_parser::{mir::MirTree, mir_lower};
use run_hir::HirResponse;
use soul_utils::{compile_options::CompilerOptions, sementic_level::SementicFault};

pub struct MirResponse {
    pub tree: MirTree,
}

pub fn to_mir(
    hir_response: &HirResponse,
    _options: &CompilerOptions,
    faults: &mut Vec<SementicFault>,
) -> MirResponse {
    MirResponse {
        tree: mir_lower(&hir_response, faults),
    }
}
