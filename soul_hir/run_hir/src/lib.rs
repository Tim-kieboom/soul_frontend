use ast::AstResponse;
use hir::HirTree;
use hir_parser::lower_hir;
use soul_utils::{
    compile_options::CompilerOptions, sementic_level::SementicFault
};

pub struct HirResponse {
    pub hir: HirTree,
}

pub fn to_hir(
    ast: &AstResponse,
    _options: &CompilerOptions,
    faults: &mut Vec<SementicFault>,
) -> HirResponse {
    let hir = lower_hir(ast, faults);
    // let types = infer_hir_types(&hir, faults);

    // let literal_resolves = literal_resolve(&hir, &types);
    // if options.debug_view_literal_resolve {
        
    //     for (id, literal) in literal_resolves.entries() {
    //         let span = hir.spans.expressions[id];
    //         faults.push(SementicFault::debug(literal_msg(literal, span)));
    //     }
    // }

    HirResponse { hir }
}

// fn literal_msg(literal: &Literal, span: Span) -> SoulError {
//     SoulError::new(
//         format!("literal resolved to >> {}", literal.value_to_string()),
//         soul_utils::error::SoulErrorKind::Empty,
//         Some(span),
//     )
// }