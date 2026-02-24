use hir::{Assign, ExpressionId, HirType, Place, PlaceKind, TypeId};
use soul_utils::{Ident, soul_names::TypeModifier, span::Span};

use crate::{HirContext, create_local_name};

impl<'a> HirContext<'a> {
    pub(super) fn lower_array(
        &mut self,
        id: hir::ExpressionId,
        array: &ast::Array,
        span: Span,
    ) -> hir::Expression {
        let ty = self.type_from_array(array, span);

        let temp_local = self.id_generator.alloc_local();
        let name = Ident::new(create_local_name(temp_local), span);
        self.insert_local(&name, temp_local, ty);
        let temp_place = Place::new(PlaceKind::Local(temp_local), span);

        let size = array.values.len() as u64;
        let element = self.new_infer_type(span);
        let infer_array = self.add_type(create_array(element, size));
        let unalloc = self.create_unallocted_array(infer_array, element, size, span);

        let temp_array = hir::Variable {
            ty: self.new_infer_with_modifier(TypeModifier::Mut, span),
            local: temp_local,
            value: Some(unalloc),
        };

        self.insert_desugar_variable(temp_array, span);

        for (i, element) in array.values.iter().enumerate() {
            let value = self.lower_expression(element);
            let assign = self.create_assign_array_element(i, &temp_place, value, element.span);
            self.insert_desugar_assignment(assign, element.span);
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
        size: u64,
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

fn create_array(element: TypeId, size: u64) -> HirType {
    HirType { kind: hir::HirTypeKind::Array { element, kind: ast::ArrayKind::StackArray(size) }, modifier: None }
}