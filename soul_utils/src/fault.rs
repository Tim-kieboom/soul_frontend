use std::fmt::Debug;

use crate::span::Span;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Severity {
    Note = 0,
    Warning = 1,
    Error = 2,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Fault {
    severity: Severity,
    message: String,
    span: Option<Span>,

    #[cfg(feature = "error_backtrace")]
    backtrace: String,
}
impl Fault {
    pub fn error(message: impl Into<String>, span: Option<Span>) -> Self {
        Fault::new(Severity::Error, message.into(), span)
    }

    pub fn warning(message: impl Into<String>, span: Option<Span>) -> Self {
        Fault::new(Severity::Warning, message.into(), span)
    }

    pub fn note(message: impl Into<String>, span: Option<Span>) -> Self {
        Fault::new(Severity::Note, message.into(), span)
    }

    #[cfg(feature = "error_backtrace")]
    pub fn empty() -> Self {
        Fault {
            backtrace: String::new(),
            severity: Severity::Error,
            message: String::new(),
            span: None,
        }
    }

    #[cfg(not(feature = "error_backtrace"))]
    pub fn empty() -> Self {
        Fault {
            severity: Severity::Error,
            message: String::new(),
            span: None,
        }
    }

    #[cfg(feature = "error_backtrace")]
    fn new(severity: Severity, message: String, span: Option<Span>) -> Self {
        use std::backtrace::Backtrace;

        Fault {
            backtrace: Backtrace::force_capture().to_string(),
            severity,
            message,
            span,
        }
    }

    #[cfg(not(feature = "error_backtrace"))]
    fn new(severity: Severity, message: String, span: Option<Span>) -> Self {
        Fault {
            severity,
            message,
            span,
        }
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn span(&self) -> Option<Span> {
        self.span
    }

    pub fn message(&self) -> &String {
        &self.message
    }

    #[cfg(feature = "error_backtrace")]
    pub fn backtract(&self) -> &String {
        &self.backtrace
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FaultCollector {
    pub faults: Vec<Fault>,
}
impl FaultCollector {
    pub fn push(&mut self, fault: Fault) {
        self.faults.push(fault);
    }

    pub fn push_error(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.faults.push(Fault::error(message, span));
    }

    pub fn push_warning(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.faults.push(Fault::warning(message, span));
    }

    pub fn push_note(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.faults.push(Fault::note(message, span));
    }

    pub fn iter(&self) -> impl Iterator<Item = &Fault> {
        self.faults.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = Fault> {
        self.faults.into_iter()
    }

    pub fn fails(&self, fail_level: Severity) -> bool {
        self.faults.iter().any(|d| d.severity == fail_level)
    }
}
