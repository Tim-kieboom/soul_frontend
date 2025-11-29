use models::error::SoulError;

/// - `TryError::IsNotValue(R)` is not of type (type: `R` is so that you could give ownership of value back if needed) 
/// - `TryError::IsErr(SoulError)` is of type but has error
pub(crate) enum TryError<R> {
    IsErr(SoulError),
    IsNotValue(R),
}

/// # TryResult
/// used to try parse instead of parse value
/// - `Ok(T)` success
/// - `Err(TryError::IsNotValue(R))` is not of type (type: `R` is so that you could give ownership of value back if needed) 
/// - `Err(TryError::IsErr(SoulError))` is of type but has error
pub(crate) type TryResult<T, R> = Result<T, TryError<R>>;

#[allow(non_snake_case)]
pub(crate) fn TryOk<T, R>(ok: T) -> TryResult<T, R> {
    Ok(ok)
}

#[allow(non_snake_case)]
pub(crate) fn TryErr<T, R>(err: SoulError) -> TryResult<T, R> {
    Err(TryError::IsErr(err))
}

#[allow(non_snake_case)]
pub(crate) fn TryNotValue<T, R>(rest: R) -> TryResult<T, R> {
    Err(TryError::IsNotValue(rest))
}

pub(crate) trait OptionTryResult<T, R> {
    fn try_not_value(self, rest: R) -> TryResult<T, R>;
    fn try_err(self, err: SoulError) -> TryResult<T, R>;
}

pub(crate) trait ResultTryResult<T, R> {
    fn try_not_value(self, rest: R) -> TryResult<T, R>;
    fn try_err(self) -> TryResult<T, R>;
}

pub(crate) trait TryToResult<T> {
    fn to_result(self) -> Result<T, SoulError>;
}

pub(crate) trait MapNotValue<T, E> {
    fn map_not_value<R, F: FnOnce(E) -> R>(self, func: F) -> TryResult<T, R>;
}

impl<T, E> MapNotValue<T, E> for TryResult<T, E> {
    fn map_not_value<R, F: FnOnce(E) -> R>(self, func: F) -> TryResult<T, R> {
        
        match self {
            Ok(val) => TryOk(val),
            Err(TryError::IsErr(err)) => TryErr(err),
            Err(TryError::IsNotValue(err)) => TryNotValue(func(err)),
        }
    }
}

impl<T> TryToResult<T> for TryResult<T, SoulError> {
    fn to_result(self) -> Result<T, SoulError> {
        match self {
            Ok(val) => Ok(val),
            Err(TryError::IsErr(err)) |
            Err(TryError::IsNotValue(err)) => Err(err)
        }
    }
}

impl<T, R> OptionTryResult<T, R> for Option<T> {
    fn try_not_value(self, rest: R) -> TryResult<T, R> {
        match self {
            Some(val) => TryOk(val),
            None => TryNotValue(rest),
        }
    }

    fn try_err(self, err: SoulError) -> TryResult<T, R> {
        match self {
            Some(val) => TryOk(val),
            None => TryErr(err),
        }
    }
}

impl<T, R> ResultTryResult<T, R> for Result<T, SoulError> {
    fn try_not_value(self, rest: R) -> TryResult<T, R> {
        match self {
            Ok(val) => TryOk(val),
            Err(_) => TryNotValue(rest),
        }
    }

    fn try_err(self) -> TryResult<T, R> {
        match self {
            Ok(val) => TryOk(val),
            Err(err) => TryErr(err),
        }
    }
}
