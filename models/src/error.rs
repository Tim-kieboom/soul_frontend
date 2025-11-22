use std::usize;

/// A result type alias for operations that can fail with a `SoulError`.
pub type SoulResult<T> = std::result::Result<T, SoulError>;

/// The kind of error that occurred during parsing or compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ErrorKind {
    NoKind, // no kind selected

    InternalError,

    ArgError, // error with program args
    ReaderError, // e.g. could not read line

    UnterminatedStringLiteral, // e.g., string not closed
    InvalidEscapeSequence, // e.g., "\q" in a string
    EndingWithSemicolon, // if line ends with ';'
    UnmatchedParenthesis, // e.g., "(" without ")"
    
    WrongType,

    UnexpectedToken, // e.g., found ";" but expected "\n"
    
    NotFoundInScope,

    InvalidStringFormat, // if f"..." has incorrect argument
    InvalidInContext,
    InvalidPath,
    InvalidName,
    InvalidType,
    InvalidNumber,

    UnexpectedEnd,
}

/// An identifier for macro expansion context.
///
/// Used to track which macro expansion (if any) produced a particular piece of code.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    kind: ErrorKind,
    message: String,
    span: Option<Span>,
}

impl SoulError {
    pub fn new<S: Into<String>>(message: S, kind: ErrorKind, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            kind,
            span,
        }
    }

    /// Consumes the error and returns its message.
    pub fn to_message(self) -> String {
        self.message
    } 
}

impl Span {
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
}