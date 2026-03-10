use std::cmp::Ordering;
use ast::{BinaryOperator, Literal};

use crate::{literal_types_compatible, try_as_f64, try_as_i128, try_as_u128};

pub(crate) fn interpret_binary(
    left: &Literal,
    operator: &BinaryOperator,
    right: &Literal,
) -> Option<Literal> {
    
    if !literal_types_compatible(left, right) {
        return None
    }

    use ast::BinaryOperatorKind as B;

    Some(match operator.node {
        B::Invalid
        | B::TypeOf
        | B::Range => return None,

        B::Add | B::Sub | B::Mul | B::Div | B::Mod
        | B::Pow | B::Root | B::Log => {
            if let (Some(a), Some(b)) = (try_as_i128(left), try_as_i128(right)) {
                match operator.node {
                    B::Add => Literal::Int(a + b as i128),
                    B::Sub => Literal::Int(a - b as i128),
                    B::Mul => Literal::Int(a * b as i128),
                    B::Div => Literal::Float(a as f64 / b as f64),
                    B::Mod => Literal::Int(a % b),
                    B::Pow => Literal::Float((a as f64).powf(b as f64)),
                    B::Root => Literal::Float((a as f64).powf(1.0 / b as f64)),
                    B::Log => Literal::Float((a as f64).log(b as f64)),
                    _ => unreachable!(),
                }
            } else if let (Some(a), Some(b)) = (try_as_f64(left), try_as_f64(right)) {
                match operator.node {
                    B::Add => Literal::Float(a + b),
                    B::Sub => Literal::Float(a - b),
                    B::Mul => Literal::Float(a * b),
                    B::Div => Literal::Float(a / b),
                    B::Mod => Literal::Float(a % b),
                    B::Pow => Literal::Float(a.powf(b)),
                    B::Root => Literal::Float(a.powf(1.0 / b)),
                    B::Log => Literal::Float(a.log(b)),
                    _ => unreachable!(),
                }
            } else {
                return None
            }
        }

        B::BitAnd | B::BitOr | B::BitXor => {
            if let (Some(a), Some(b)) = (try_as_i128(left), try_as_i128(right)) {
                match operator.node {
                    B::BitAnd => Literal::Int((a & b) as i128),
                    B::BitOr  => Literal::Int((a | b) as i128),
                    B::BitXor => Literal::Int((a ^ b) as i128),
                    _ => unreachable!(),
                }
            } else if let (Some(a), Some(b)) = (try_as_u128(left), try_as_u128(right)) {
                match operator.node {
                    B::BitAnd => Literal::Uint((a & b) as u128),
                    B::BitOr  => Literal::Uint((a | b) as u128),
                    B::BitXor => Literal::Uint((a ^ b) as u128),
                    _ => unreachable!(),
                }
            } else {
                return None;
            }
        }

        B::LogAnd | B::LogOr => {
            match (left, right) {
                (Literal::Bool(a), Literal::Bool(b)) => match operator.node {
                    B::LogAnd => Literal::Bool(*a && *b),
                    B::LogOr  => Literal::Bool(*a || *b),
                    _ => unreachable!(),
                },
                _ => return None,
            }
        }

        B::Eq | B::NotEq => {
            let eq = match (left, right) {
                (Literal::Int(a), Literal::Int(b)) => a == b,
                (Literal::Uint(a), Literal::Uint(b)) => a == b,
                (Literal::Float(a), Literal::Float(b)) => a == b,
                (Literal::Bool(a), Literal::Bool(b)) => a == b,
                (Literal::Char(a), Literal::Char(b)) => a == b,
                (Literal::Str(a), Literal::Str(b)) => a == b,
                _ => {
                    if let (Some(a), Some(b)) = (try_as_f64(left), try_as_f64(right)) {
                        a == b
                    } else {
                        return None
                    }
                }
            };
            Literal::Bool(match operator.node {
                B::Eq => eq,
                B::NotEq => !eq,
                _ => unreachable!(),
            })
        }

        B::Lt | B::Gt | B::Le | B::Ge => {
            let ord = match (left, right) {
                (Literal::Int(a), Literal::Int(b)) => a.partial_cmp(b),
                (Literal::Uint(a), Literal::Uint(b)) => a.partial_cmp(b),
                (Literal::Float(a), Literal::Float(b)) => a.partial_cmp(b),
                (Literal::Char(a), Literal::Char(b)) => a.partial_cmp(b),
                (Literal::Str(a), Literal::Str(b)) => a.partial_cmp(b),
                _ => {
                    if let (Some(a), Some(b)) = (try_as_f64(left), try_as_f64(right)) {
                        a.partial_cmp(&b)
                    } else {
                        None
                    }
                }
            }?;

            let res = match operator.node {
                B::Lt => ord == Ordering::Less,
                B::Gt => ord == Ordering::Greater,
                B::Le => ord != Ordering::Greater,
                B::Ge => ord != Ordering::Less,
                _ => unreachable!(),
            };
            Literal::Bool(res)
        }
    })
}