use ast::{UnaryOperator, Literal};

use crate::{try_as_f64, try_as_i128};

pub(crate) fn interpret_unary(
    operator: &UnaryOperator,
    operand: &Literal,
) -> Option<Literal> {
    use ast::UnaryOperatorKind as U;

    Some(match operator.node {
        U::Invalid => return None,
        U::Neg => {
            if let Some(a) = try_as_f64(operand) {
                Literal::Float(-a)
            } else if let Some(a) = try_as_i128(operand) {
                Literal::Int((-a) as i128)
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
            let value = match operand {
                Literal::Int(i) => *i as i128,
                Literal::Uint(u) => *u as i128,
                Literal::Float(value) => {
                    return Some(match operator.node {
                        U::Increment { .. } => Literal::Float(value + 1.0),
                        U::Decrement { .. } => Literal::Float(value - 1.0),
                        _ => unreachable!(),
                    })
                }
                _ => return None,
            };

            match operator.node {
                U::Increment { .. } => Literal::Int((value + 1) as i128),
                U::Decrement { .. } => Literal::Int((value - 1) as i128),
                _ => unreachable!(),
            }
        }

    })
}