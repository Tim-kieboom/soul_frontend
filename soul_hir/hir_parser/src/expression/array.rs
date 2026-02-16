use hir::{Assign, ExpressionId, HirType, Place, PlaceKind, TypeId};
use soul_utils::span::Span;

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_array(
        &mut self,
        id: hir::ExpressionId,
        array: &ast::Array,
        span: Span,
    ) -> hir::Expression {
        let (ty, element_type) = self.type_from_array(array, span);

        let temp_local = self.id_generator.alloc_local();
        self.insert_local_type(temp_local, ty);
        let temp_place = Place::new(PlaceKind::Local(temp_local), span);

        let size = array.values.len();
        let unalloc = self.create_unallocted_array(ty, element_type, size, span);

        let temp_array = hir::Variable {
            ty,
            local: temp_local,
            value: Some(unalloc),
        };

        self.insert_desugar_variable(temp_array, span);

        for (i, element) in array.values.iter().enumerate() {
            let value = self.lower_expression(element);
            let assign = self.create_assign_array_element(i, &temp_place, value, span);
            self.insert_desugar_assignment(assign, span);
        }

        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Load(temp_place),
        }
    }

    fn create_unallocted_array(
        &mut self,
        ty: TypeId,
        element_type: TypeId,
        size: usize,
        span: Span,
    ) -> ExpressionId {
        let uint = self.add_type(HirType::index_type());

        let len = self.alloc_expression(span);
        self.insert_expression(
            len,
            hir::Expression {
                id: len,
                ty: uint,
                kind: hir::ExpressionKind::Literal(ast::Literal::Uint(size as u64)),
            },
        );

        let unalloc = self.alloc_expression(span);
        self.insert_expression(
            unalloc,
            hir::Expression {
                ty,
                id: unalloc,
                kind: hir::ExpressionKind::InnerRawStackArray {
                    ty: element_type,
                    len,
                },
            },
        );
        unalloc
    }

    fn create_assign_array_element(
        &mut self,
        i: usize,
        place: &hir::Place,
        value: ExpressionId,
        span: Span,
    ) -> Assign {
        let ty = self.add_type(HirType::index_type());
        let id = self.alloc_expression(span);
        let index = self.insert_expression(
            id,
            hir::Expression {
                id,
                ty,
                kind: hir::ExpressionKind::Literal(ast::Literal::Uint(i as u64)),
            },
        );

        Assign {
            value,
            place: Place::new(
                PlaceKind::Index {
                    base: Box::new(place.clone()),
                    index,
                },
                span,
            ),
        }
    }
}
