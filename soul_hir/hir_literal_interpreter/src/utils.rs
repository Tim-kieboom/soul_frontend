use ast::Literal;

pub(crate) fn try_as_f64(l: &Literal) -> Option<f64> {
    match l {
        Literal::Int(i) => Some(*i as f64),
        Literal::Uint(u) => Some(*u as f64),
        Literal::Float(f) => Some(*f),
        _ => None,
    }
}

pub(crate) fn try_as_i128(l: &Literal) -> Option<i128> {
    match l {
        Literal::Int(i) => Some(*i),
        Literal::Uint(u) => Some(*u as i128),
        _ => None,
    }
}

pub(crate) fn try_as_u128(l: &Literal) -> Option<u128> {
    match l {
        Literal::Uint(u) => Some(*u),
        _ => None,
    }
}
