use mir_parser::{mir::MirTree, mir_lower};
use run_hir::HirResponse;
use soul_utils::{compile_options::CompilerOptions, sementic_level::SementicFault};

pub fn to_mir(hir: &HirResponse, _options: &CompilerOptions, faults: &mut Vec<SementicFault>) -> MirTree {
    mir_lower(&hir.tree, &hir.types, faults)
}
