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

        if self.is_single_line() {
            format!("{}:{}", self.start_line, self.start_offset)
        } else {
            format!("{}:{}-{}:{}", self.start_line, self.start_offset, self.end_line, self.end_offset)
        }
    }
}