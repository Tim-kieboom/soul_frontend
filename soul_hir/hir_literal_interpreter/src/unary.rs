use ast::{Literal, UnaryOperator};

use crate::{try_as_f64, try_as_i128};

pub(crate) fn interpret_unary(operator: &UnaryOperator, operand: &Literal) -> Option<Literal> {
    use ast::UnaryOperatorKind as U;

    Some(match operator.node {
        U::Invalid => return None,
        U::Neg => {
            if let Some(a) = try_as_f64(operand) {
                Literal::Float(-a)
            } else if let Some(a) = try_as_i128(operand) {
                Literal::Int(-a)
            } else {
                return None;
            }
        }

        U::Not => {
            if let Literal::Bool(b) = operand {
                Literal::Bool(!*b)
            } else {
                return None;
            }
        }

        U::Increment { before_var: _ } | U::Decrement { before_var: _ } => {
            // For literals, we treat increment/decrement as simple arithmetic
            // (ignoring the before_var flag since literals aren't variables)
            match operand {
                Literal::Int(value) => match operator.node {
                    U::Increment { .. } => Literal::Int(value + 1),
                    U::Decrement { .. } => Literal::Int(value - 1),
                    _ => unreachable!(),
                },
                Literal::Uint(value) => match operator.node {
                    U::Increment { .. } => Literal::Uint(value + 1),
                    U::Decrement { .. } => Literal::Uint(value - 1),
                    _ => unreachable!(),
                },
                Literal::Float(value) => match operator.node {
                    U::Increment { .. } => Literal::Float(value + 1.0),
                    U::Decrement { .. } => Literal::Float(value - 1.0),
                    _ => unreachable!(),
                },
                _ => return None,
            }
        }
    })
}
