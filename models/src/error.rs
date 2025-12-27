use std::usize;

/// A result type alias for operations that can fail with a `SoulError`.
pub type SoulResult<T> = std::result::Result<T, SoulError>;

/// The kind of error that occurred during parsing or compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoulErrorKind {
    SourceReadError,
    ScopeError,
    ScopeOverride(Span),

    UnexpecedToken,
    UnexpecedFileEnd,
    UnexpecedStatmentStart,

    InvalidAssignType,
    InvalidContext,
    InvalidChar,
    InvalidName,
    InvalidIdent,
    InvalidNumber,
    InvalidOperator,
    InvalidStatment,
    InvalidTokenKind,
    InvalidExpression,
    InvalidEscapeSequence,
}

/// An identifier for macro expansion context.
///
/// Used to track which macro expansion (if any) produced a particular piece of code.
#[derive(
    Debug,
    Clone,
    Default,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ExpansionId(usize);
impl ExpansionId {
    /// Creates a new `ExpansionId` with the given value.
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the underlying `usize` value.
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

/// Represents a source code location span.
///
/// Tracks the start and end positions of code in the source file, along with
/// any macro expansion context.
#[derive(
    Debug,
    Clone,
    Default,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Span {
    /// The starting line number (1-indexed).
    pub start_line: usize,
    /// The starting column/offset within the line (1-indexed).
    pub start_offset: usize,
    /// The ending line number (1-indexed).
    pub end_line: usize,
    /// The ending column/offset within the line (1-indexed).
    pub end_offset: usize,
    /// Macro expansion context identifier, or 0 if not from a macro expansion.
    pub expansion_id: ExpansionId,
}

/// An error that occurred during parsing or compilation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SoulError {
    pub kind: SoulErrorKind,
    pub message: String,
    pub span: Option<Span>,
}

impl SoulError {
    pub fn empty() -> Self {
        Self {
            kind: SoulErrorKind::InvalidName,
            message: String::default(),
            span: None,
        }
    }

    pub fn new<S: Into<String>>(message: S, kind: SoulErrorKind, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            kind,
            span,
        }
    }
}

impl Span {
    pub const fn default_const() -> Self {
        Self {
            start_line: 0,
            start_offset: 0,
            end_line: 0,
            end_offset: 0,
            expansion_id: ExpansionId(0),
        }
    }

    /// Creates a span that represents a single point on a line.
    ///
    /// Both start and end positions are set to the same line and offset.
    pub fn new_line(line: usize, offset: usize) -> Self {
        Self {
            start_line: line,
            start_offset: offset,
            end_line: line,
            end_offset: offset,
            expansion_id: ExpansionId::default(),
        }
    }

    pub fn combine(mut self, other: Self) -> Self {
        self.start_line = self.start_line.min(other.start_line);
        self.start_offset = self.start_offset.min(other.start_offset);
        self.end_line = self.end_line.max(other.end_line);
        self.end_offset = self.end_offset.max(other.end_offset);
        self
    }
}
