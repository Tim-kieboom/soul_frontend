use crate::span::Span;

pub type SoulResult<T> = Result<T, Fault>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Note = 0,
    Warning = 1,
    Error = 2,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Fault {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
}
impl Fault {
    pub fn error(message: impl Into<String>, span: Option<Span>) -> Self  {
        Fault { 
            severity: Severity::Error, 
            message: message.into(), 
            span,
        }
    }

    pub fn warning(message: impl Into<String>, span: Option<Span>) -> Self  {
        Fault { 
            severity: Severity::Warning, 
            message: message.into(), 
            span,
        }
    }

    pub fn note(message: impl Into<String>, span: Option<Span>) -> Self {
        Fault { 
            severity: Severity::Note, 
            message: message.into(), 
            span,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FaultCollector {
    faults: Vec<Fault>,
}
impl FaultCollector {
    
    pub fn error(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.faults.push(Fault::error(message, span));
    }

    pub fn warning(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.faults.push(Fault::warning(message, span));
    }

    pub fn note(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.faults.push(Fault::note(message, span));
    }

    pub fn iter(&self) -> impl Iterator<Item = &Fault> {
        self.faults.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = Fault> {
        self.faults.into_iter()
    }

    pub fn fails(&self, fail_level: Severity) -> bool {
        self.faults
            .iter()
            .any(|d| d.severity == fail_level)
    }
}