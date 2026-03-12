use ast::AstResponse;
use hir::HirTree;
use hir_literal_interpreter::try_literal_resolve_expression;
use hir_parser::hir_lower;
use hir_typed_context::{HirTypedTable, infer_hir_types};
use soul_utils::{
    compile_options::CompilerOptions, error::SoulError, sementic_level::SementicFault,
};

pub struct HirResponse {
    pub tree: HirTree,
    pub types: HirTypedTable,
}

pub fn to_hir(
    ast: &AstResponse,
    options: &CompilerOptions,
    faults: &mut Vec<SementicFault>,
) -> HirResponse {
    let hir = hir_lower(ast, faults);
    let types = infer_hir_types(&hir, faults);

    if options.debug_view_literal_resolve {
        for value_id in hir.expressions.keys() {
            let span = hir.spans.expressions[value_id];
            let literal = try_literal_resolve_expression(&hir, &types, value_id);
            if let Some(literal) = literal {
                let msg = SoulError::new(
                    format!("literal resolved to >> {}", literal.value_to_string()),
                    soul_utils::error::SoulErrorKind::Empty,
                    Some(span),
                );
                faults.push(SementicFault::debug(msg));
            }
        }
    }

    HirResponse { types, tree: hir }
}
