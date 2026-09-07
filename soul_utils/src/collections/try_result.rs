use crate::{
    error::SoulResult,
    fault::{Fault, UnclassifiedKind},
};

/// Error type for try-parsing operations.
///
/// - `TryError::IsNotValue(R)` - the value is not of the expected type
/// - `TryError::IsErr(Fault<K>)` - the value is of the expected type but has an error
pub enum TryError<R, K = UnclassifiedKind> {
    /// The value is of the correct type but an error occurred.
    IsErr(Fault<K>),
    /// The value is not of the expected type.
    IsNotValue(R),
}

/// Result type for try-parsing operations.
///
/// - `Ok(T)` success
/// - `Err(TryError::IsNotValue(R))` - the value is not of the expected type
/// - `Err(TryError::IsErr(Fault<K>))` - the value is of the type but has an error
pub type TryResult<T, R, K = UnclassifiedKind> = Result<T, TryError<R, K>>;

/// Creates a successful `TryResult`.
#[allow(non_snake_case)]
pub fn TryOk<T, R, K>(ok: T) -> TryResult<T, R, K> {
    Ok(ok)
}

/// Creates a `TryResult` with an error.
#[allow(non_snake_case)]
pub fn TryErr<T, R, K>(err: Fault<K>) -> TryResult<T, R, K> {
    Err(TryError::IsErr(err))
}

/// Creates a `TryResult` indicating the value is not of the expected type.
#[allow(non_snake_case)]
pub fn TryNotValue<T, R, K>(rest: R) -> TryResult<T, R, K> {
    Err(TryError::IsNotValue(rest))
}

/// Utility trait for converting `Result` to `TryResult`.
pub trait ResultTryErr<T, R, K = UnclassifiedKind> {
    fn try_err(self) -> TryResult<T, R, K>;
}

/// Utility trait for converting `Result` to `TryResult`.
pub trait ResultTryNotValue<T, R, K = UnclassifiedKind> {
    fn try_not_value(self) -> TryResult<T, R, K>;
}

/// Utility trait for mapping the "not value" case in `TryResult`.
pub trait ResultMapNotValue<T, R, V, K = UnclassifiedKind> {
    fn map_try_not_value<F: Fn(R) -> V>(self, func: F) -> TryResult<T, V, K>;
}

/// Utility trait for merging `TryResult` to `SoulResult`.
pub trait ToResult<T> {
    fn merge_to_result(self) -> SoulResult<T>;
}

impl<T> ToResult<T> for TryResult<T, Fault> {
    fn merge_to_result(self) -> SoulResult<T> {
        match self {
            Ok(val) => Ok(val),
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue(err)) => Err(err),
        }
    }
}

/// Converts `Result<T, Fault<K1>>` into `TryResult<T, R, K2>` for any target
/// `K2` that `K1` can convert into (including `K1 == K2`, via the reflexive
/// `From` impl). This is what lets a not-yet-migrated utility method
/// returning `Fault<UnclassifiedKind>` plug into a migrated function's
/// `AstErrorKind`-typed `TryResult` via `.try_err()`, and what lets an
/// already-migrated method's `Fault<K>` pass through unchanged.
impl<T, R, K1, K2> ResultTryErr<T, R, K2> for Result<T, Fault<K1>>
where
    K2: From<K1>,
{
    fn try_err(self) -> TryResult<T, R, K2> {
        match self {
            Ok(val) => TryOk(val),
            Err(err) => TryErr(err.map_kind(Into::into)),
        }
    }
}

impl<T, K1, K2> ResultTryNotValue<T, Fault<K1>, K2> for Result<T, Fault<K1>> {
    fn try_not_value(self) -> TryResult<T, Fault<K1>, K2> {
        match self {
            Ok(val) => TryOk(val),
            Err(err) => TryNotValue(err),
        }
    }
}

impl<T, K1, K2> ResultTryNotValue<T, (), K2> for Result<T, Fault<K1>> {
    fn try_not_value(self) -> TryResult<T, (), K2> {
        match self {
            Ok(val) => TryOk(val),
            Err(_) => TryNotValue(()),
        }
    }
}

impl<T, R, V, K> ResultMapNotValue<T, R, V, K> for TryResult<T, R, K> {
    fn map_try_not_value<F: FnOnce(R) -> V>(self, func: F) -> TryResult<T, V, K> {
        match self {
            Ok(val) => TryOk(val),
            Err(TryError::IsErr(err)) => TryErr(err),
            Err(TryError::IsNotValue(err)) => TryNotValue(func(err)),
        }
    }
}
