use ast::{AstResponse, Literal};
use hir::{ExpressionId, HirTree};
use hir_literal_interpreter::literal_resolve;
use hir_parser::hir_lower;
use hir_typed_context::{HirTypedTable, infer_hir_types};
use soul_utils::{
    compile_options::CompilerOptions, error::SoulError, sementic_level::SementicFault, span::Span, vec_map::VecMap
};

pub struct HirResponse {
    pub hir: HirTree,
    pub types: HirTypedTable,
    pub literal_resolves: VecMap<ExpressionId, Literal>,
}

pub fn to_hir(
    ast: &AstResponse,
    options: &CompilerOptions,
    faults: &mut Vec<SementicFault>,
) -> HirResponse {
    let hir = hir_lower(ast, faults);
    let types = infer_hir_types(&hir, faults);

    let literal_resolves = literal_resolve(&hir, &types);
    if options.debug_view_literal_resolve {
        
        for (id, literal) in literal_resolves.entries() {
            let span = hir.spans.expressions[id];
            faults.push(SementicFault::debug(literal_msg(literal, span)));
        }
    }

    HirResponse { types, hir, literal_resolves }
}

fn literal_msg(literal: &Literal, span: Span) -> SoulError {
    SoulError::new(
        format!("literal resolved to >> {}", literal.value_to_string()),
        soul_utils::error::SoulErrorKind::Empty,
        Some(span),
    )
}