use std::path::{Path, PathBuf};

use crate::span::Span;
#[cfg(feature = "error_backtrace")]
use std::backtrace::Backtrace;

// A result type alias for operations that can fail with a `SoulError`.
pub type SoulResult<T> = std::result::Result<T, SoulError>;

pub fn relative_to_project(file_path: &str) -> String {
    inner_relative_to_project(file_path).unwrap_or(file_path.to_string())
}

fn inner_relative_to_project(file_path: &str) -> Option<String> {
    let mut root = PathBuf::from(file_path);
    while root.parent().is_some() {
        let cargo_toml = root.join("Cargo.toml");
        if cargo_toml.exists() {
            let parent_root = root.parent().unwrap();
            return Path::new(file_path)
                .strip_prefix(parent_root)
                .ok()
                .map(|p| p.to_string_lossy().to_string());
        }
        root.pop();
    }
    None
}

/// makes [SoulError] of kind `SoulErrorKind::InternalError(file!(), line!())`
///
/// ```
/// let span = Span::default();
/// let err = soul_error_internal!("msg", Some(span));
/// let expanded = SoulError::new(
///     "msg",
///     SoulErrorKind::InternalError(file!().to_string(), line!()),
///     Some(span)
/// );
/// ```
#[cfg(feature = "absolute_internal_error_path")]
#[macro_export]
macro_rules! soul_error_internal {
    ($msg:expr, $span:expr) => {
        $crate::error::SoulError::new(
            $msg,
            $crate::error::SoulErrorKind::InternalError(file!().to_string(), line!()),
            $span,
        )
    };
}

/// makes [SoulError] of kind `SoulErrorKind::InternalError(file!(), line!())`
///
/// ```
/// let span = Span::default();
/// let err = soul_error_internal!("msg", Some(span));
/// let expanded = SoulError::new(
///     "msg",
///     SoulErrorKind::InternalError(relative_to_project(file!()), line!()),
///     Some(span)
/// );
/// ```
#[cfg(not(feature = "absolute_internal_error_path"))]
#[macro_export]
macro_rules! soul_error_internal {
    ($msg:expr, $span:expr) => {
        $crate::error::SoulError::new(
            $msg,
            $crate::error::SoulErrorKind::InternalError(
                $crate::error::relative_to_project(file!()),
                line!(),
            ),
            $span,
        )
    };
}

/// The kind of error that occurred during parsing or compilation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoulErrorKind {
    Empty,
    InternalError(String, u32),

    NotFoundInScope,

    UnifyTypeError,
    PlaceTypeError,
    TypeInferenceError,

    InvalidIdent,
    InvalidNumber,
    InvalidContext,
    InvalidOperator,
    InvalidTokenKind,
    UnexpecedFileEnd,
    ScopeOverride(Span),
    UnexpectedCharacter,
    InvalidEscapeSequence,
}

/// An error that occurred during parsing or compilation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SoulError {
    pub kind: SoulErrorKind,
    pub message: String,
    pub span: Option<Span>,

    #[cfg(feature = "error_backtrace")]
    pub backtrace: String,
}

impl SoulError {
    #[cfg(not(feature = "error_backtrace"))]
    pub fn empty() -> Self {
        Self {
            kind: SoulErrorKind::Empty,
            message: String::new(),
            span: None,
        }
    }

    #[cfg(feature = "error_backtrace")]
    pub fn empty() -> Self {
        Self {
            kind: SoulErrorKind::Empty,
            backtrace: String::new(),
            message: String::new(),
            span: None,
        }
    }

    #[cfg(not(feature = "error_backtrace"))]
    pub fn new<S: Into<String>>(message: S, kind: SoulErrorKind, span: Option<Span>) -> Self {

        Self {
            message: message.into(),
            kind,
            span,
        }
    }

    #[cfg(feature = "error_backtrace")]
    pub fn new<S: Into<String>>(message: S, kind: SoulErrorKind, span: Option<Span>) -> Self {
        Self {
            kind,
            span,
            message: message.into(),
            backtrace: Backtrace::force_capture().to_string(),
        }
    }

}
