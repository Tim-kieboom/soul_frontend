use ast::{AstResponse, Literal};
use hir::{ExpressionId, HirTree};
use hir_literal_interpreter::literal_resolve;
use hir_parser::lower_hir;
use soul_utils::{
    compile_options::CompilerOptions, error::SoulError, sementic_level::SementicFault, span::Span, vec_map::VecMap
};
use typed_hir::TypedHir;
use typed_hir_parser::lower_typed_hir;

pub struct HirResponse {
    pub hir: HirTree,
    pub typed: TypedHir,
    pub literal_resolves: VecMap<ExpressionId, Literal>,
}

pub fn to_hir(
    ast: &AstResponse,
    options: &CompilerOptions,
    faults: &mut Vec<SementicFault>,
) -> HirResponse {
    let hir = lower_hir(ast, faults);
    let typed = lower_typed_hir(&hir, faults);

    let literal_resolves = literal_resolve(&hir, &typed);
    if options.debug_view_literal_resolve {
        
        for (id, literal) in literal_resolves.entries() {
            let span = hir.info.spans.expressions[id];
            faults.push(SementicFault::debug(literal_msg(literal, span)));
        }
    }

    HirResponse { hir, typed, literal_resolves }
}

fn literal_msg(literal: &Literal, span: Span) -> SoulError {
    SoulError::new(
        format!("literal resolved to >> {}", literal.value_to_string()),
        soul_utils::error::SoulErrorKind::Empty,
        Some(span),
    )
}