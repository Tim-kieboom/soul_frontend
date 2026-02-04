use std::hash::Hash;
use crate::Ident;

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

/// A node wrapped with source location information and attributes.
///
/// All AST nodes are wrapped in `Spanned` to track their location in the
/// source code for error reporting and debugging.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Spanned<T> {
    /// The actual AST node.
    pub node: T,
    meta_data: NodeMetaData
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NodeMetaData {
    /// The source code span where this node appears.
    pub span: Span,
    /// Additional attributes associated with this node.
    pub attributes: Vec<Attribute>,
}

impl Span {
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

    pub fn combine(mut self, other: Self) -> Self {
        self.start_line = self.start_line.min(other.start_line);
        self.start_offset = self.start_offset.min(other.start_offset);
        self.end_line = self.end_line.max(other.end_line);
        self.end_offset = self.end_offset.max(other.end_offset);
        self
    }

    pub fn is_single_line(&self) -> bool {
        self.end_line == self.start_line
    }

    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb);
        sb        
    }

    pub fn inner_display(&self, sb: &mut String) {
        use std::fmt::Write;

        if self.is_single_line() {
            write!(sb, "{}:{}", self.start_line, self.start_offset)
        } else {
            write!(sb, "{}:{}-{}:{}", self.start_line, self.start_offset, self.end_line, self.end_offset)
        }.expect("write should not give error")
    }
}

impl<T: Hash> Hash for Spanned<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}
impl<T> Spanned<T> {
    pub fn new(inner: T, span: Span) -> Self {
        Self {
            node: inner,
            meta_data: NodeMetaData { span, attributes: vec![] }
        }
    }

    pub fn with_meta_data(inner: T, meta_data: NodeMetaData) -> Self {
        Self {
            node: inner,
            meta_data
        }
    }

    pub fn get_span(&self) -> Span {
        self.meta_data.span
    }

    pub fn get_meta_data(&self) -> &NodeMetaData {
        &self.meta_data
    }

    pub fn get_meta_data_mut(&mut self) -> &mut NodeMetaData {
        &mut self.meta_data
    }

    pub fn consume(self) -> (T, NodeMetaData) {
        (self.node, self.meta_data)
    }
}