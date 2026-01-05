/// Emits a compile-time reminder warning.
///
/// # Purpose
/// This function exists solely to trigger a **compiler warning** via
/// `#[deprecated]`. Calling it will produce a warning at the call site,
/// making it useful for leaving temporary reminders such as:
///
/// - unfinished refactors
/// - missing invariants
/// - TODOs that should not be forgotten
///
/// # Usage
/// ```rust
/// remind_warning("add private constructor");
/// ```
///
/// # Notes
/// - The argument is ignored at runtime and exists only for readability.
/// - The warning category will always be **`deprecated`**, as Rust does not
///   support custom warning kinds on stable.
/// - This function should be removed once the reminder is addressed.
///
/// # Example
/// ```rust
/// fn my_type() {
///     remind_warning("replace with typestate pattern");
/// }
/// ```
///
/// Calling this function will cause a compiler warning similar to:
/// ```text
/// warning: use of deprecated function `remind_warning`
/// ```
///
/// # Implementation Detail
/// This is intentionally implemented as a function rather than a macro to
/// ensure the warning is attributed to the call site and remains simple
/// and stable.
#[deprecated(note = "not realy deprecated just a reminder warning")]
pub fn remind_warning(_msg: &str){}