use crate::{ids::IdAlloc, impl_soul_ids};

impl_soul_ids!(ModuleId);

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpanLine {
    pub line: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub module: ModuleId,
    pub start: SpanLine,
    pub end: SpanLine,
}

impl Span {
    pub const fn new(module: ModuleId, start: SpanLine, end: SpanLine) -> Self {
        Self { module, start, end }
    }

    pub const fn new_line(module: ModuleId, span: SpanLine) -> Self {
        Self { module, start: span, end: span }
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

    pub fn error() -> Self {
        Self {
            module: ModuleId::error(),
            start: SpanLine { line: 0, offset: 0 },
            end: SpanLine { line: 0, offset: 0 },
        }
    }
}