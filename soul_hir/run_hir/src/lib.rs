use ast::{AstResponse};
use hir::{ComplexLiteral, ExpressionId, HirTree};
use hir_literal_interpreter::{literal_resolve};
use hir_parser::lower_hir;
use soul_utils::{
    compile_options::CompilerOptions, error::SoulError, sementic_level::SementicFault, span::Span,
    vec_map::VecMap,
};
use typed_hir::TypedHir;
use typed_hir_parser::lower_typed_hir;

pub struct HirResponse {
    pub hir: HirTree,
    pub typed: TypedHir,
    pub literal_resolves: VecMap<ExpressionId, ComplexLiteral>,
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
            faults.push(SementicFault::debug(literal_msg(literal, &hir, span)));
        }
    }

    HirResponse {
        hir,
        typed,
        literal_resolves,
    }
}

fn literal_msg(literal: &ComplexLiteral, hir: &HirTree, span: Span) -> SoulError {
    
    let mut literal_str = String::new();
    literal_display(literal, hir, &mut literal_str);

    SoulError::new(
        format!("literal resolved to >> {literal_str}"),
        soul_utils::error::SoulErrorKind::Empty,
        Some(span),
    )
}

pub fn literal_display(literal: &ComplexLiteral, hir: &HirTree, sb: &mut String) {
    use std::fmt::Write;

    match literal {
        ComplexLiteral::Basic(literal) => sb.push_str(&literal.value_to_string()),
        ComplexLiteral::Struct { struct_id, values, struct_type:_, all_fields_const:_ } => {
            
            let object = hir.info.types.id_to_struct(*struct_id);
            match object {
                Some(obj) => sb.push_str(obj.name.as_str()),
                None => write!(sb, "{:?}", struct_id).expect("no fmt error"),
            }

            sb.push('{');
            let last_index = values.len().saturating_sub(1);
            for (i, (value, _ty)) in values.iter().enumerate() {
                let field = match object {
                    Some(obj) => obj.fields.get(i),
                    None => None,
                };
                
                match field {
                    Some(val) => sb.push_str(&val.name),
                    None => write!(sb, "_{}", i).expect("no fmt error"),
                }

                sb.push_str(": ");
                literal_display(value, hir, sb);
                if i != last_index {
                    sb.push_str(", ");
                }
            }
            sb.push('}');
        },
    }
}
