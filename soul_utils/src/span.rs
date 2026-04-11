use crate::Ident;
use std::hash::Hash;

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
}

/// An attribute identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Attribute {
    pub name: Ident,
    pub values: Vec<Ident>,
}

/// All AST nodes are wrapped in `Spanned` to track their location in the
/// source code for error reporting and debugging.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Spanned<T> {
    /// The actual AST node.
    pub node: T,
    /// The source code location.
    pub span: Span,
}

/// Metadata associated with an AST item.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ItemMetaData {
    /// Additional attributes associated with this node.
    pub attributes: Vec<Attribute>,
}

impl ItemMetaData {
    /// Creates a default `ItemMetaData` with no attributes.
    pub const fn default_const() -> Self {
        Self::new(vec![])
    }

    /// Creates a new `ItemMetaData` with the given attributes.
    pub const fn new(attributes: Vec<Attribute>) -> Self {
        Self { attributes }
    }
}

impl Span {
    /// Creates a default (zero-valued) span.
    pub const fn default_const() -> Self {
        Self {
            start_line: 0,
            start_offset: 0,
            end_line: 0,
            end_offset: 0,
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
        }
    }

    /// Checks whether the span is on a single line.
    pub fn is_single_line(&self) -> bool {
        self.end_line == self.start_line
    }

    /// Returns a string representation of the span (e.g., "1:1" or "1:1-2:10").
    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb);
        sb
    }

    /// Writes a string representation of the span to the given buffer.
    pub fn inner_display(&self, sb: &mut String) {
        use std::fmt::Write;

        if self.is_single_line() {
            write!(sb, "{}:{}", self.start_line, self.start_offset)
        } else {
            write!(
                sb,
                "{}:{}-{}:{}",
                self.start_line, self.start_offset, self.end_line, self.end_offset
            )
        }
        .expect("write should not give error")
    }

    /// Combines this span with another, creating a new span that encompasses both.
    pub fn combine(self, other: Self) -> Self {
        let start_line = self.start_line.min(other.start_line);
        let start_offset = self.combine_start_offset(&other);

        let end_line = self.end_line.max(other.end_line);
        let end_offset = self.combine_end_offset(&other);

        Self {
            start_line,
            start_offset,
            end_line,
            end_offset,
        }
    }

    fn combine_start_offset(&self, other: &Self) -> usize {
        if self.start_line == other.start_line {
            return self.start_offset.min(other.start_offset);
        }

        if self.start_line < other.start_line {
            self.start_offset
        } else {
            other.start_offset
        }
    }

    fn combine_end_offset(&self, other: &Self) -> usize {
        if self.end_line == other.end_line {
            return self.end_offset.max(other.end_offset);
        }

        if self.end_line > other.end_line {
            self.end_offset
        } else {
            other.end_offset
        }
    }
}

impl<T: Hash> Hash for Spanned<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}
impl<T> Spanned<T> {
    /// Creates a new `Spanned` wrapper around a node with its source location.
    pub fn new(inner: T, span: Span) -> Self {
        Self { node: inner, span }
    }
}
