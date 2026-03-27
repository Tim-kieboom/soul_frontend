use hir::{Assign, ExpressionId, HirType, Place, PlaceId, PlaceKind, PossibleTypeId};
use soul_utils::{
    Ident,
    soul_names::TypeModifier,
    span::{ItemMetaData, Span},
};

use crate::{HirContext, create_local_name};

impl<'a> HirContext<'a> {
    pub(super) fn lower_array(
        &mut self,
        id: hir::ExpressionId,
        array: &ast::Array,
        span: Span,
    ) -> hir::Expression {
        let ty = hir::PossibleTypeId::Known(self.type_from_array(array, span));

        let temp_local = self.id_generator.alloc_local();
        let name = Ident::new(create_local_name(temp_local), span);
        let temp_place = Place::new(
            self.id_generator.alloc_place(),
            PlaceKind::Temp(temp_local),
            span,
        );

        let size = array.values.len() as u64;
        let element = self.new_infer_type(vec![], None);
        let infer_array = PossibleTypeId::Known(self.add_type(create_array(element, size)));

        let unalloc = self.create_unallocted_array(infer_array, element, size, span);
        self.insert_temp(&name, temp_local, ty, unalloc);

        let temp_array = hir::Variable {
            ty: self.new_infer_type(vec![], Some(TypeModifier::Mut)),
            is_temp: true,
            local: temp_local,
            value: Some(unalloc),
        };

        self.insert_desugar_variable(temp_array, span);

        for (i, element) in array.values.iter().enumerate() {
            let value = self.lower_expression(element);
            let assign = self.create_assign_array_element(i, temp_place.id, value, element.span);
            self.insert_desugar_assignment(assign, element.span);
        }

        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Load(self.insert_place(temp_place)),
        }
    }

    fn create_unallocted_array(
        &mut self,
        ty: PossibleTypeId,
        element_type: PossibleTypeId,
        size: u64,
        span: Span,
    ) -> ExpressionId {
        let uint = PossibleTypeId::Known(self.add_type(HirType::index_type()));

        let len = self.alloc_expression(span);
        self.insert_expression(
            len,
            hir::Expression {
                id: len,
                ty: uint,
                kind: hir::ExpressionKind::Literal(ast::Literal::Uint(size as u128)),
            },
        );

        let unalloc = self.alloc_expression(span);
        self.insert_expression(
            unalloc,
            hir::Expression {
                ty,
                id: unalloc,
                kind: hir::ExpressionKind::InnerRawStackArray(element_type),
            },
        );
        unalloc
    }

    fn create_assign_array_element(
        &mut self,
        i: usize,
        place: PlaceId,
        value: ExpressionId,
        span: Span,
    ) -> Assign {
        let ty = PossibleTypeId::Known(self.add_type(HirType::index_type()));

        let id = self.alloc_expression(span);
        let index = self.insert_expression(
            id,
            hir::Expression {
                id,
                ty,
                kind: hir::ExpressionKind::Literal(ast::Literal::Uint(i as u128)),
            },
        );

        let place = Place::new(
            self.id_generator.alloc_place(),
            PlaceKind::Index { base: place, index },
            span,
        );

        Assign {
            value,
            place: self.insert_place(place),
        }
    }

    pub(super) fn insert_desugar_variable(&mut self, variable: hir::Variable, span: Span) {
        let name = Ident::new(create_local_name(variable.local), span);

        self.insert_variable(&name, variable.local, variable.ty, variable.value);

        match self.current_body {
            crate::CurrentBody::Global => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let kind = hir::GlobalKind::InternalVariable(variable);
                self.tree.root.globals.push(hir::Global::new(kind, id));
            }
            crate::CurrentBody::Block(block_id) => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let kind = hir::StatementKind::Variable(variable);
                self.insert_in_block(block_id, hir::Statement::new(kind, id));
            }
        }
    }

    fn insert_desugar_assignment(&mut self, assign: hir::Assign, span: Span) {
        match self.current_body {
            crate::CurrentBody::Global => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let kind = hir::GlobalKind::InternalAssign(assign);
                self.tree.root.globals.push(hir::Global::new(kind, id));
            }
            crate::CurrentBody::Block(block_id) => {
                let id = self.alloc_statement(&ItemMetaData::default_const(), span);
                let kind = hir::StatementKind::Assign(assign);
                self.insert_in_block(block_id, hir::Statement::new(kind, id));
            }
        }
    }
}

fn create_array(element: PossibleTypeId, size: u64) -> HirType {
    HirType {
        kind: hir::HirTypeKind::Array {
            element,
            kind: ast::ArrayKind::StackArray(size),
        },
        modifier: None,
        generics: vec![],
    }
}
