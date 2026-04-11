use crate::{define_str_enum, error::SoulError};

define_str_enum!(
    /// Severity level for diagnostics and faults.
    pub enum SementicLevel {
        /// Error level (compilation fails).
        Error => "error", 0,
        /// Warning level (may continue).
        Warning => "warning", 1,
        /// Note level (informational).
        Note => "note", 2,
        /// Debug level (development only).
        Debug => "debug", 3,
    }
);

/// A fault (error/warning/note) that occurred during compilation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SementicFault {
    /// The underlying error.
    message: SoulError,
    /// The severity level of this fault.
    level: SementicLevel,
}
impl SementicFault {
    /// Creates a new error-level fault.
    pub const fn error(err: SoulError) -> Self {
        Self {
            message: err,
            level: SementicLevel::Error,
        }
    }

    /// Creates a new debug-level fault.
    pub const fn debug(err: SoulError) -> Self {
        Self {
            message: err,
            level: SementicLevel::Debug,
        }
    }

    /// Consumes the fault and returns the underlying error.
    pub fn consume_soul_error(self) -> SoulError {
        self.message
    }

    /// Returns a reference to the underlying error.
    pub const fn get_soul_error(&self) -> &SoulError {
        &self.message
    }

    /// Returns the severity level of this fault.
    pub const fn get_level(&self) -> SementicLevel {
        self.level
    }

    /// Checks whether this fault is fatal given the minimum fatal level.
    pub const fn is_fatal(&self, fatal_level: SementicLevel) -> bool {
        fatal_level.precedence().as_usize() >= self.get_level().precedence().as_usize()
    }
}
