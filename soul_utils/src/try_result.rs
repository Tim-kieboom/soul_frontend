use crate::error::{SoulError, SoulResult};


/// - `TryError::IsNotValue(R)` is not of type (type: `R` is so that you could give ownership of value back if needed)
/// - `TryError::IsErr(SoulError)` is of type but has error
pub enum TryError<R> {
    IsErr(SoulError),
    IsNotValue(R),
}

/// # TryResult
/// used to try parse instead of parse value
/// - `Ok(T)` success
/// - `Err(TryError::IsNotValue(R))` is not of type (type: `R` is so that you could give ownership of value back if needed)
/// - `Err(TryError::IsErr(SoulError))` is of type but has error
pub type TryResult<T, R> = Result<T, TryError<R>>;

#[allow(non_snake_case)]
pub fn TryOk<T, R>(ok: T) -> TryResult<T, R> {
    Ok(ok)
}

#[allow(non_snake_case)]
pub fn TryErr<T, R>(err: SoulError) -> TryResult<T, R> {
    Err(TryError::IsErr(err))
}

#[allow(non_snake_case)]
pub fn TryNotValue<T, R>(rest: R) -> TryResult<T, R> {
    Err(TryError::IsNotValue(rest))
}

pub trait ResultTryErr<T, R> {
    fn try_err(self) -> TryResult<T, R>;
}

pub trait ResultTryNotValue<T, R> {
    fn try_not_value(self) -> TryResult<T, R>;
}
pub trait ResultMapNotValue<T, R, V> {
    fn map_try_not_value<F: Fn(R) -> V>(self, func: F) -> TryResult<T, V>;
}

pub trait ToResult<T> {
    fn merge_to_result(self) -> SoulResult<T>;
}

impl<T> ToResult<T> for TryResult<T, SoulError> {
    fn merge_to_result(self) -> SoulResult<T> {
        match self {
            Ok(val) => Ok(val),
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue(err)) => Err(err),
        }
    }
}

impl<T, R> ResultTryErr<T, R> for Result<T, SoulError> {
    fn try_err(self) -> TryResult<T, R> {
        match self {
            Ok(val) => TryOk(val),
            Err(err) => TryErr(err),
        }
    }
}

impl<T> ResultTryNotValue<T, SoulError> for Result<T, SoulError> {
    fn try_not_value(self) -> TryResult<T, SoulError> {
        match self {
            Ok(val) => TryOk(val),
            Err(err) => TryNotValue(err),
        }
    }
}

impl<T> ResultTryNotValue<T, ()> for Result<T, SoulError> {
    fn try_not_value(self) -> TryResult<T, ()> {
        match self {
            Ok(val) => TryOk(val),
            Err(_) => TryNotValue(()),
        }
    }
}

impl<T, R, V> ResultMapNotValue<T, R, V> for TryResult<T, R> {
    fn map_try_not_value<F: FnOnce(R) -> V>(self, func: F) -> TryResult<T, V> {
        match self {
            Ok(val) => TryOk(val),
            Err(TryError::IsErr(err)) => TryErr(err),
            Err(TryError::IsNotValue(err)) => TryNotValue(func(err)), 
        }
    }
}
