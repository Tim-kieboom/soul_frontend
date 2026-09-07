use ast_model::statements::Assignment;
use ast_parser::fault::AstErrorKind;
use soul_utils::TypeModifier;

use super::function_call::is_generic_parameter;
use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(crate) fn check_assignment(&mut self, assignment: &Assignment) {
        let Some((modifier, left_ty)) = self.variable_lvalue(assignment.left) else {
            return;
        };

        if matches!(modifier, TypeModifier::Immut | TypeModifier::Const) {
            let span = self.get_expression(assignment.left).map(|expr| expr.span);

            self.log_error(AstErrorKind::AssignToImmutableVariable, span);
        }

        let empty = vec![];
        let generics = match self.current.function {
            Some(id) => self
                .declares
                .get_function(id)
                .map(|(signature, _)| &signature.generics)
                .unwrap_or(&empty),
            None => &empty,
        };

        if is_generic_parameter(&left_ty, generics) {
            return;
        }

        let Some(right_ty) = self.expression_type(assignment.right) else {
            return;
        };
        if self.combine_operand_types(&right_ty, &left_ty).is_some() {
            return;
        }

        let span = self.get_expression(assignment.right).map(|expr| expr.span);
        self.log_error(
            AstErrorKind::AssignmentTypeMismatch {
                expected: format!("{left_ty:?}").into(),
                got: format!("{right_ty:?}").into(),
            },
            span,
        );
    }
}
