use std::fmt::Debug;

use crate::{Ident, impl_soul_ids};

impl_soul_ids!(ModuleId);

/// Metadata associated with an AST item.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ItemMetaData {
    /// Additional attributes associated with this node.
    pub attributes: Vec<Attribute>,
}

/// An attribute identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Attribute {
    pub name: Ident,
    pub values: Vec<Ident>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SpanLine {
    pub line: usize,
    pub offset: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub module: ModuleId,
    pub start: SpanLine,
    pub end: SpanLine,
}

impl<T> Spanned<T> {
    pub const fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn map<R, F: FnOnce(T) -> R>(self, mapper: F) -> Spanned<R> {
        let Spanned { value, span } = self;
        Spanned {
            value: mapper(value),
            span,
        }
    }
}

impl Span {
    pub const fn new(module: ModuleId, start: SpanLine, end: SpanLine) -> Self {
        Self { module, start, end }
    }

    pub const fn new_line(module: ModuleId, span: SpanLine) -> Self {
        Self {
            module,
            start: span,
            end: span,
        }
    }

    pub const fn default(module: ModuleId) -> Self {
        Self {
            module,
            start: SpanLine { line: 0, offset: 0 },
            end: SpanLine { line: 0, offset: 0 },
        }
    }

    pub fn is_single(&self) -> bool {
        let offset = self.end.offset.saturating_sub(self.start.offset);
        self.start.line == self.end.line && offset <= 1
    }

    pub const fn error() -> Self {
        Self {
            module: ModuleId::ERROR,
            start: SpanLine { line: 0, offset: 0 },
            end: SpanLine { line: 0, offset: 0 },
        }
    }

    /// Combines this span with another, creating a new span that encompasses both.
    pub fn combine(self, other: Self) -> Self {
        debug_assert_eq!(self.module, other.module);

        let start_line = self.start.line.min(other.start.line);
        let start_offset = self.combine_start_offset(&other);

        let end_line = self.end.line.max(other.end.line);
        let end_offset = self.combine_end_offset(&other);

        Self {
            module: self.module,
            end: SpanLine {
                line: end_line,
                offset: end_offset,
            },
            start: SpanLine {
                line: start_line,
                offset: start_offset,
            },
        }
    }

    fn combine_start_offset(&self, other: &Self) -> usize {
        if self.start.line == other.start.line {
            return self.start.offset.min(other.start.offset);
        }

        if self.start.line < other.start.line {
            self.start.offset
        } else {
            other.start.offset
        }
    }

    fn combine_end_offset(&self, other: &Self) -> usize {
        if self.end.line == other.end.line {
            return self.end.offset.max(other.end.offset);
        }

        if self.end.line > other.end.line {
            self.end.offset
        } else {
            other.end.offset
        }
    }
}
impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Span {
            module: _,
            start,
            end,
        } = self;
        f.write_fmt(format_args!("{}:{}", start.line, start.offset))?;
        if self.is_single() {
            return Ok(());
        }

        f.write_fmt(format_args!(" - {}:{}", end.line, end.offset))?;
        Ok(())
    }
}
