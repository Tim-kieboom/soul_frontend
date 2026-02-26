use hir::{
    BlockId, ExpressionId, FunctionId, HirTree, HirType, IdAlloc, LocalId, PlaceId, StatementId,
    TypeId, TypesMap,
};
use soul_utils::{
    error::SoulError, sementic_level::SementicFault, span::Span, vec_map::VecMap, vec_set::VecSet,
};

use crate::infer_table::InferTable;

mod expression;
mod handle_type;
mod infer_table;
mod statement;

pub fn infer_types(hir: &HirTree, faults: &mut Vec<SementicFault>) -> HirTypedTable {
    let mut context = HirTypedContext::new(hir, faults);

    for global in &hir.root.globals {
        context.infer_global(global);
    }

    context.resolve_all_types();
    context.finalize_types();
    context.to_type_table()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirTypedTable {
    pub places: VecMap<PlaceId, TypeId>,
    pub locals: VecMap<LocalId, TypeId>,
    pub blocks: VecMap<BlockId, TypeId>,
    pub functions: VecMap<FunctionId, TypeId>,
    pub statements: VecMap<StatementId, TypeId>,
    pub expressions: VecMap<ExpressionId, TypeId>,

    pub types: TypesMap,
    pub auto_copys: VecSet<ExpressionId>,
}
impl HirTypedTable {
    fn remap_types(&mut self, map: &VecMap<TypeId, TypeId>) {
        macro_rules! remap {
            ($field:ident) => {
                for ty in self.$field.values_mut() {
                    if let Some(new) = map.get(*ty) {
                        *ty = *new;
                    }
                }
            };
        }
        remap!(locals);
        remap!(blocks);
        remap!(functions);
        remap!(statements);
        remap!(expressions);
    }
}

struct HirTypedContext<'a> {
    hir: &'a HirTree,
    faults: &'a mut Vec<SementicFault>,

    is_in_unsafe: bool,
    none_type: TypeId,
    type_table: HirTypedTable,
    infer_table: InferTable,
}
impl<'a> HirTypedContext<'a> {
    fn new(hir: &'a HirTree, faults: &'a mut Vec<SementicFault>) -> Self {
        let functions = hir
            .functions
            .entries()
            .map(|(i, function)| (i, function.return_type))
            .collect();

        let types = hir.types.clone();
        let mut this = Self {
            hir,
            faults,
            is_in_unsafe: false,
            none_type: TypeId::error(),
            infer_table: InferTable::new(&hir.types),
            type_table: HirTypedTable {
                functions,
                places: VecMap::const_default(),
                locals: VecMap::const_default(),
                blocks: VecMap::with_capacity(hir.blocks.len()),
                statements: VecMap::with_capacity(hir.meta_data.statements.len()),
                expressions: VecMap::with_capacity(hir.expressions.len()),

                types,
                auto_copys: VecSet::new(),
            },
        };
        this.none_type = this.add_type(HirType::none_type());
        this
    }

    fn statement_span(&self, id: StatementId) -> Span {
        self.hir.spans.statements[id]
    }

    fn expression_span(&self, id: ExpressionId) -> Span {
        self.hir.spans.expressions[id]
    }

    fn block_span(&self, id: BlockId) -> Span {
        self.hir.spans.blocks[id]
    }

    fn add_autocopy(&mut self, id: ExpressionId) {
        self.type_table.auto_copys.insert(id);
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }

    fn to_type_table(self) -> HirTypedTable {
        self.type_table
    }
}
