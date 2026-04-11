use crate::error::{SoulError, SoulResult};

/// Error type for try-parsing operations.
///
/// - `TryError::IsNotValue(R)` - the value is not of the expected type
/// - `TryError::IsErr(SoulError)` - the value is of the expected type but has an error
pub enum TryError<R> {
    /// The value is of the correct type but an error occurred.
    IsErr(SoulError),
    /// The value is not of the expected type.
    IsNotValue(R),
}

/// Result type for try-parsing operations.
///
/// - `Ok(T)` success
/// - `Err(TryError::IsNotValue(R))` - the value is not of the expected type
/// - `Err(TryError::IsErr(SoulError))` - the value is of the type but has an error
pub type TryResult<T, R> = Result<T, TryError<R>>;

/// Creates a successful `TryResult`.
#[allow(non_snake_case)]
pub fn TryOk<T, R>(ok: T) -> TryResult<T, R> {
    Ok(ok)
}

/// Creates a `TryResult` with an error.
#[allow(non_snake_case)]
pub fn TryErr<T, R>(err: SoulError) -> TryResult<T, R> {
    Err(TryError::IsErr(err))
}

/// Creates a `TryResult` indicating the value is not of the expected type.
#[allow(non_snake_case)]
pub fn TryNotValue<T, R>(rest: R) -> TryResult<T, R> {
    Err(TryError::IsNotValue(rest))
}

/// Utility trait for converting `Result` to `TryResult`.
pub trait ResultTryErr<T, R> {
    fn try_err(self) -> TryResult<T, R>;
}

/// Utility trait for converting `Result` to `TryResult`.
pub trait ResultTryNotValue<T, R> {
    fn try_not_value(self) -> TryResult<T, R>;
}

/// Utility trait for mapping the "not value" case in `TryResult`.
pub trait ResultMapNotValue<T, R, V> {
    fn map_try_not_value<F: Fn(R) -> V>(self, func: F) -> TryResult<T, V>;
}

/// Utility trait for merging `TryResult` to `SoulResult`.
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
