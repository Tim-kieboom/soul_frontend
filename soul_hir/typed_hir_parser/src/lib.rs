use hir::{
    BlockId, ExpressionId, Field, FieldId, GenericId, HirTree, HirType, HirTypeKind, InferType,
    InferTypeId, InferTypesMap, LazyTypeId, LocalId, Place, PlaceId, StatementId, TypeId, TypesMap,
    Variable,
};
use soul_utils::{
    compile_options::CompilerOptions,
    error::SoulError,
    ids::{FunctionId, IdAlloc},
    sementic_level::SementicFault,
    soul_error_internal,
    soul_names::{PrimitiveTypes, TypeModifier},
    span::Span,
    vec_map::VecMap,
    vec_set::VecSet,
};
use typed_hir::{LazyFieldInfo, TypedHir};

use crate::infer_table::InferTable;

mod expression;
mod finalize;
mod handle_type;
mod infer_table;
mod place;
mod statement;
mod type_helpers;
pub use type_helpers::UnifyPrimitiveCast;

pub fn lower_typed_hir<'a>(
    hir: &'a HirTree,
    options: &'a CompilerOptions,
    faults: &'a mut Vec<SementicFault>,
) -> TypedHir {
    let mut context = TypedHirContext::new(hir, options, faults);

    for (struct_id, object) in hir.info.types.structs_entries() {
        for (i, field) in object.fields.iter().enumerate() {
            let struct_type = HirType::new(HirTypeKind::Struct(struct_id));
            let base = context.add_type(struct_type);
            context.type_field(field, base, i);
        }
    }

    for function in context.hir.nodes.functions.values() {
        context.functions.insert(function.id, function.return_type);
    }

    for global in &context.hir.root.globals {
        context.infer_global(global);
    }

    context.finalize()
}

struct TypedHirContext<'a> {
    types: TypesMap,
    hir: &'a HirTree,
    infers: InferTypesMap,
    infer_table: InferTable,
    options: &'a CompilerOptions,
    auto_copys: VecSet<ExpressionId>,
    current_function: Option<FunctionId>,
    field_names: VecMap<FieldId, String>,

    u32_type: TypeId,
    none_type: TypeId,
    bool_type: TypeId,
    places: VecMap<PlaceId, LazyTypeId>,
    locals: VecMap<LocalId, LazyTypeId>,
    blocks: VecMap<BlockId, LazyTypeId>,
    functions: VecMap<FunctionId, TypeId>,
    fields: VecMap<FieldId, LazyFieldInfo>,
    place_fields: VecMap<PlaceId, FieldId>,
    statements: VecMap<StatementId, LazyTypeId>,
    sizeofs: VecMap<ExpressionId, LazyTypeId>,
    expressions: VecMap<ExpressionId, LazyTypeId>,
    generic_defines: VecMap<GenericId, VecSet<TypeId>>,

    faults: &'a mut Vec<SementicFault>,
}
impl<'a> TypedHirContext<'a> {
    fn new(
        hir: &'a HirTree,
        options: &'a CompilerOptions,
        faults: &'a mut Vec<SementicFault>,
    ) -> Self {
        let mut this = Self {
            hir,
            faults,
            options,
            current_function: None,
            types: hir.info.types.clone(),
            infers: hir.info.infers.clone(),
            infer_table: InferTable::new(&hir.info.infers),

            fields: VecMap::new(),
            auto_copys: VecSet::new(),
            none_type: TypeId::error(),
            bool_type: TypeId::error(),
            u32_type: TypeId::error(),
            place_fields: VecMap::new(),
            generic_defines: VecMap::new(),
            sizeofs: VecMap::new(),
            places: VecMap::with_capacity(hir.nodes.places.len()),
            locals: VecMap::with_capacity(hir.nodes.locals.len()),
            blocks: VecMap::with_capacity(hir.nodes.blocks.len()),
            statements: VecMap::with_capacity(hir.root.globals.len()),
            field_names: VecMap::with_capacity(hir.nodes.fields.len()),
            functions: VecMap::with_capacity(hir.nodes.functions.len()),
            expressions: VecMap::with_capacity(hir.nodes.expressions.len()),
        };
        this.none_type = this.add_type(HirType::none_type());
        this.bool_type = this.add_type(HirType::bool_type());
        this.u32_type = this.add_type(HirType::primitive_type(PrimitiveTypes::Uint32));
        this
    }

    fn id_to_type(&self, ty: TypeId) -> &HirType {
        static ERROR: HirType = HirType::error_type();
        self.types.id_to_type(ty).unwrap_or(&ERROR)
    }

    fn id_to_infer(&self, ty: InferTypeId) -> &InferType {
        self.infers
            .get_infer(ty)
            .expect("TypeId should always have a type")
    }

    fn lazy_id_get_modifier(&self, id: LazyTypeId) -> Option<TypeModifier> {
        match id {
            LazyTypeId::Known(type_id) => self.id_to_type(type_id).modifier,
            LazyTypeId::Infer(infer_type_id) => self.id_to_infer(infer_type_id).modifier,
        }
    }

    fn lazy_id_insure_modifier(
        &mut self,
        id: LazyTypeId,
        modifier: Option<TypeModifier>,
    ) -> LazyTypeId {
        match id {
            LazyTypeId::Known(type_id) => {
                if self.id_to_type(type_id).modifier == modifier {
                    return id;
                }

                let mut kown = self.id_to_type(type_id).clone();
                kown.modifier = modifier;
                self.add_type(kown).to_lazy()
            }
            LazyTypeId::Infer(infer_id) => {
                if self.id_to_infer(infer_id).modifier == modifier {
                    return id;
                }

                let mut infer = self.id_to_infer(infer_id).clone();
                let span = self
                    .infers
                    .get_span(infer_id)
                    .unwrap_or(Span::default_const());
                infer.modifier = modifier;
                hir::LazyTypeId::Infer(self.infers.insert(infer, span))
            }
        }
    }

    fn insert_generic_define(&mut self, id: GenericId, ty: TypeId) {
        let generic_defines = &mut self.generic_defines;
        if let Some(types) = generic_defines.get_mut(id) {
            types.insert(ty);
            return;
        }

        let types = VecSet::from_slice(&[ty]);
        generic_defines.insert(id, types);
    }

    fn resolve_generic(
        &mut self,
        generic_defines: &VecMap<GenericId, TypeId>,
        ty: LazyTypeId,
    ) -> LazyTypeId {
        let ty = match ty {
            LazyTypeId::Known(val) => val,
            LazyTypeId::Infer(_) => return ty,
        };

        let hir_type = self.id_to_type(ty);
        match hir_type.kind {
            HirTypeKind::Generic(generic_id) => match generic_defines.get(generic_id) {
                Some(val) => val.to_lazy(),
                None => LazyTypeId::error(),
            },
            _ => ty.to_lazy(),
        }
    }

    fn get_priority_lazy_type(&mut self, left: LazyTypeId, right: LazyTypeId) -> LazyTypeId {
        let (left, right) = match (left, right) {
            (LazyTypeId::Known(l), LazyTypeId::Known(r)) => (l, r),

            (LazyTypeId::Known(known), LazyTypeId::Infer(infer))
            | (LazyTypeId::Infer(infer), LazyTypeId::Known(known)) => {
                let ty = known.to_lazy();
                self.infer_table.add_infer_binding(infer, ty);
                return ty;
            }
            (LazyTypeId::Infer(_), LazyTypeId::Infer(_)) => return left,
        };

        self.infer_table
            .get_priority_type(&self.types, left, right)
            .to_lazy()
    }

    fn get_priority_type(&mut self, left: TypeId, right: TypeId) -> TypeId {
        self.infer_table.get_priority_type(&self.types, left, right)
    }

    fn get_place(&self, place: PlaceId) -> &Place {
        &self.hir.nodes.places[place]
    }

    fn add_type(&mut self, ty: HirType) -> TypeId {
        self.types.insert_type(ty)
    }

    fn statement_span(&self, id: StatementId) -> Span {
        self.hir.info.spans.statements[id]
    }

    fn expression_span(&self, id: ExpressionId) -> Span {
        self.hir.info.spans.expressions[id]
    }

    fn block_span(&self, id: BlockId) -> Span {
        self.hir.info.spans.blocks[id]
    }

    fn type_block(&mut self, id: BlockId, ty: LazyTypeId) {
        self.blocks.insert(id, ty);
    }

    fn type_statement(&mut self, id: StatementId, ty: LazyTypeId) {
        self.statements.insert(id, ty);
    }

    fn type_expression(&mut self, id: ExpressionId, ty: LazyTypeId) {
        self.expressions.insert(id, ty);
    }

    fn get_variable_value(&mut self, variable: &Variable) -> Option<ExpressionId> {
        let info = &self.hir.nodes.locals[variable.local];
        match &info.kind {
            hir::LocalKind::Variable(expression_id) => *expression_id,
            other => {
                self.log_error(soul_error_internal!(
                    format!("LocalKind::{} should be unreachable in TypedHirContext::get_variable_value", other.display_variant()), 
                    info.span
                ));
                None
            }
        }
    }

    fn type_field(&mut self, field: &Field, base: TypeId, index: usize) {
        let info = LazyFieldInfo {
            base_type: base,
            field_index: index,
            field_type: field.ty,
        };
        self.fields.insert(field.id, info);
    }

    fn get_variable_type(&self, variable: &Variable) -> LazyTypeId {
        self.hir.nodes.locals[variable.local].ty
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }
}
