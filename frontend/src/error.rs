use std::usize;

pub type SoulResult<T> = std::result::Result<T, SoulError>;

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

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExpansionId(usize);
impl ExpansionId {
    pub fn new(value: usize) -> Self {
        Self(value)
    } 

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub start_line: usize,
    pub start_offset: usize,
    pub end_line: usize,
    pub end_offset: usize,
    pub expansion_id: ExpansionId, // macro expansion context or 0 if none
}

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

    pub fn to_message(self) -> String {
        self.message
    } 
}

impl Span {

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