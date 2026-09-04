use std::path::{Path, PathBuf};

use crate::fault::Fault;

// A result type alias for operations that can fail with a `Fault`.
pub type SoulResult<T> = Result<T, Fault>;

/// Converts an absolute file path to be relative to the project root.
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

/// makes [Fault] that adds in message `(file!(), line!())`
///
/// ```
/// let span = Span::default();
/// let err = soul_error_internal!("msg", Some(span));
/// let expanded = Fault::error(
///     format!("!!internal_error!! {} at {}:{}", "msg", file!().to_string(), line!()),
///     Some(span)
/// );
/// ```
#[cfg(feature = "absolute_internal_error_path")]
#[macro_export]
macro_rules! soul_error_internal {
    ($msg:expr, $span:expr) => {
        $crate::fault::Fault::error(
            format!(
                "!!internal_error!! {} at {}:{}",
                $msg,
                file!().to_string(),
                line!()
            ),
            $span,
        )
    };
}

/// makes [Fault] that adds in message `(file!(), line!())`
///
/// ```
/// use soul_utils::{
///     fault::Fault,
///     soul_error_internal,
///     span::{Span, ModuleId},
///     error::relative_to_project,
/// };
/// 
/// let span = Span::default(ModuleId::ERROR);
/// let err = soul_error_internal!("msg", Some(span));
/// let expanded = Fault::error(
///     format!("!!internal_error!! {} at {}:{}", "msg", relative_to_project(file!()), line!()),
///     Some(span)
/// );
/// ```
#[cfg(not(feature = "absolute_internal_error_path"))]
#[macro_export]
macro_rules! soul_error_internal {
    ($msg:expr, $span:expr) => {
        $crate::fault::Fault::error(
            format!(
                "!!internal_error!! {} at {}:{}",
                $msg,
                $crate::error::relative_to_project(file!()),
                line!()
            ),
            $span,
        )
    };
}
