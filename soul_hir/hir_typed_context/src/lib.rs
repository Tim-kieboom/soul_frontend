use hir::{
    BlockId, ExpressionId, FunctionId, HirTree, HirType, IdAlloc, LocalId, StatementId, TypeId,
    TypesMap, UnifyResult,
};
use soul_utils::{
    error::SoulError, sementic_level::SementicFault, span::Span, vec_map::VecMap, vec_set::VecSet,
};

use crate::infer_table::InferTable;

mod expression;
mod infer_table;
mod statement;

pub fn infer_types(hir: &HirTree, faults: &mut Vec<SementicFault>) -> HirTypedTable {
    let mut context = HirTypedContext::new(hir, faults);

    for global in &hir.root.globals {
        context.infer_global(global);
    }

    context.to_type_table()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirTypedTable {
    pub locals: VecMap<LocalId, TypeId>,
    pub blocks: VecMap<BlockId, TypeId>,
    pub functions: VecMap<FunctionId, TypeId>,
    pub statements: VecMap<StatementId, TypeId>,
    pub expressions: VecMap<ExpressionId, TypeId>,

    pub types: TypesMap,
    pub auto_copys: VecSet<ExpressionId>,
}

struct HirTypedContext<'a> {
    hir: &'a HirTree,
    faults: &'a mut Vec<SementicFault>,

    is_in_unsafe: bool,
    none_type: TypeId,
    type_table: HirTypedTable,
    infer_table: InferTable<'a>,
}
impl<'a> HirTypedContext<'a> {
    fn new(hir: &'a HirTree, faults: &'a mut Vec<SementicFault>) -> Self {
        let functions = hir.functions.entries().map(|(i, function)| (i, function.return_type)).collect();

        let mut this = Self {
            hir,
            faults,
            is_in_unsafe: false,
            none_type: TypeId::error(),
            infer_table: InferTable::new(&hir.types),
            type_table: HirTypedTable {
                functions,
                locals: VecMap::const_default(),
                blocks: VecMap::with_capacity(hir.blocks.len()),
                statements: VecMap::with_capacity(hir.meta_data.statements.len()),
                expressions: VecMap::with_capacity(hir.expressions.len()),

                types: hir.types.clone(),
                auto_copys: VecSet::new(),
            },
        };
        this.none_type = this.add_type(HirType::none_type());
        this
    }

    fn unify(&mut self, value: ExpressionId, expect: TypeId, got: TypeId, span: Span) {
        match self.infer_table.unify_type_type(expect, got, span) {
            Ok(UnifyResult::Ok) => (),
            Ok(UnifyResult::NeedsAutoCopy) => self.add_autocopy(value),
            Err(err) => self.log_error(err),
        }
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

    fn type_block(&mut self, id: BlockId, ty: TypeId) {
        self.type_table.blocks.insert(id, ty);
    }

    fn type_function(&mut self, id: FunctionId, ty: TypeId) {
        self.type_table.functions.insert(id, ty);
    }

    fn type_statement(&mut self, id: StatementId, ty: TypeId) {
        self.type_table.statements.insert(id, ty);
    }

    fn type_expression(&mut self, id: ExpressionId, ty: TypeId) {
        self.type_table.expressions.insert(id, ty);
    }

    fn type_local(&mut self, id: LocalId, ty: TypeId) {
        self.type_table.locals.insert(id, ty);
    }

    fn add_type(&mut self, ty: HirType) -> TypeId {
        self.type_table.types.insert(ty)
    }

    fn add_autocopy(&mut self, id: ExpressionId) {
        self.type_table.auto_copys.insert(id);
    }

    fn get_type(&self, ty: TypeId) -> &HirType {
        self.type_table
            .types
            .get_type(ty)
            .expect("TypeId should always have a type")
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }

    fn to_type_table(self) -> HirTypedTable {
        self.type_table
    }
}
